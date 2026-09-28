<script setup lang="ts">
/**
 * テーマライブラリ (ホーム)
 *
 * design/library.jsx のデザインを Vue/Nuxt に移植したもの。
 * Phase 5-3 (5-1 のリデザイン) に対応。
 *
 * - 検索 / フィルター chip / ソート / グリッド表示
 * - ドラッグ&ドロップによる .cursorpack インポート (UI のみ; IPC 未接続)
 * - 適用ボタン → 親へ emit (将来的に invoke('apply_theme'))
 */
import type { ThemeCardData } from '~/types/theme'
import { mapLocalSummaryToCard, type IpcThemeSummary } from '~/pages/index.helpers'
import type { IpcWindowsScheme } from '~/composables/useWindowsSchemes'

const { t, locale } = useI18n()
// UiIcon / ThemeCard / ApplyModal は Nuxt の自動インポートで解決される。

import type { FilterChip, SortDir, SortKey } from './index.helpers'

const themes = ref<ThemeCardData[]>([])
const searchQuery = ref('')
const filter = ref<FilterChip>('all')
const sortKey = ref<SortKey>('updated')
/** ソート方向。一覧の列ヘッダクリックでトグル、グリッドの cycleSort では `desc` 固定。 */
const sortDir = ref<SortDir>('desc')
const viewMode = ref<'grid' | 'list'>('grid')
const isLoading = ref(true)
const showDrop = ref(false)

// 適用確認モーダル制御
const pendingTheme = ref<ThemeCardData | null>(null)
const applyBusy = ref(false)
// .cursorpack インポート (検査 + 取込) 実行中フラグ (LD8)。ファイルダイアログ自体は
// ネイティブのため busy にせず、選択後の inspect/actuallyImport の間だけ true にする。
const importBusy = ref(false)
const applyError = ref<string | null>(null)
// 詳細モーダルの二次アクション (edit/export/duplicate/delete) 実行中フラグ (LD8)。
// 該当ボタンにスピナーを出し、実行中はグループを無効化する。
const detailBusyAction = ref<'edit' | 'export' | 'duplicate' | 'delete' | null>(null)

// 詳細モーダル操作 (P08a L2: useThemeDetailActions に集約)。
const {
  detailTheme,
  detailPreviewMap,
  detailPreviewDetails,
  detailBusyAction,
  showDetails,
  closeDetails,
  applyFromDetail,
  editInCreator,
  duplicateTheme,
  exportTheme,
  deleteTheme,
} = useThemeDetailActions({
  themes,
  reload: () => loadThemes(),
  setError: (msg) => {
    applyError.value = msg
  },
  requestApply,
  t,
})
// Theme mutation IPC は useThemes に集約 (audit B8-SIZE-001)。
// `themes` ref と `refresh` は本ページが自前管理する `loadThemes()` を使い続けるため、
// メソッドだけ取り出す。
const { applyTheme: applyThemeIpc, setFavorite: setFavoriteIpc } = useThemes()
const { listSchemes, applyScheme } = useWindowsSchemes()

// インポートフロー (P08a L3: useLibraryImportFlow に集約)。
const { importBusy, conflictDialog, importByPath, confirmConflictOverwrite, openImportDialog } =
  useLibraryImportFlow({
    reload: () => loadThemes(),
    setError: (msg) => {
      applyError.value = msg
    },
    notifyImported: (id) => {
      const imported = themes.value.find((t) => t.id === id)
      void notify({
        title: 'EasyCursorSwap',
        body: imported
          ? t('library.notifyImported', { name: imported.name })
          : t('library.notifyImportedFallback'),
        level: 'success',
      })
    },
    t,
  })

// フィルタ・ソート (P08a L1: useLibraryFilterSort に集約)。
const {
  searchQuery,
  filter,
  sortKey,
  sortDir,
  filteredThemes,
  counts,
  sortLabel,
  cycleSort,
  sortBy,
} = useLibraryFilterSort({ themes, t })

// `useAppSettings` はグローバルシングルトン。Settings 画面で更新されると自動追従する。
const appSettings = useAppSettings()

// --- ハンドラ ---
/** カードの「適用」クリック → 確認モーダルを開く */
function requestApply(id: string) {
  const t = themes.value.find((x) => x.id === id)
  if (!t) return
  applyError.value = null
  pendingTheme.value = t
}

function cancelApply() {
  if (applyBusy.value) return
  pendingTheme.value = null
}

/** モーダルの「適用する」確定 → Rust 側で実際にレジストリ書き込み */
async function confirmApply(id: string) {
  applyBusy.value = true
  applyError.value = null
  try {
    const target = themes.value.find((x) => x.id === id)
    // Windows システムスキームは別 IPC 経路で適用する。ID が `windows:` プレフィックス
    // の場合は UUID パースエラーを避けるためにこちらを呼ぶ。
    if (target?.kind === 'system') {
      await applyScheme(target.name)
    } else {
      await applyThemeIpc(id)
    }
    // 成功 → アクティブフラグを更新
    themes.value.forEach((t) => (t.isActive = t.id === id))
    pendingTheme.value = null
    if (target) {
      void notify({
        title: 'EasyCursorSwap',
        body: t('library.notifyApplied', { name: target.name }),
        level: 'success',
      })
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    applyError.value = msg
    console.error('[Library] apply failed:', err)
  } finally {
    applyBusy.value = false
  }
}

/**
 * お気に入り切替。Source of Truth は Rust 側 `AppConfig.general.favorites` で、
 * `set_theme_favorite` IPC が永続化を行い、戻り値で全体リストを返す。
 * Windows システムスキームには対応しないので Rust 側エラー時は UI 側で握り潰す。
 */
async function toggleFavorite(id: string) {
  const target = themes.value.find((x) => x.id === id)
  if (!target || target.kind === 'system') return
  const next = !target.isFavorite
  // 楽観的更新 (失敗時は Rust の戻り値で上書き)
  target.isFavorite = next
  try {
    const updated = await setFavoriteIpc(id, next)
    if (updated) {
      const set = new Set(updated)
      themes.value.forEach((tt) => {
        if (tt.kind !== 'system') tt.isFavorite = set.has(tt.id)
      })
    }
  } catch (err) {
    // Tauri 未起動時はエラーになるが、ローカル状態は既に更新済みなので無視。
    console.warn('[Library] set_theme_favorite failed:', err)
  }
}

/**
 * カードのシェブロン押下で開く詳細モーダル。
 *
 * モーダルが共有されているのでプレビューマップは開いた瞬間にロードする。
 * `useThemePreviews` 側で IPC 結果がキャッシュされているので 2 回目以降は即時表示。
 */

// Tauri v2 ウィンドウドラッグ&ドロップ (P08a: useTauriFileDrop に集約)。
// `.cursorpack` のみ受理し、Explorer からの取込は既存 importByPath フローへ。
const {
  showDrop,
  start: startFileDrop,
  stop: stopFileDrop,
} = useTauriFileDrop({
  accept: ['cursorpack'],
  onDrop: (paths) => {
    for (const path of paths) void importByPath(path)
  },
  onRejected: () => {
    applyError.value = t('library.importNotPack')
  },
})

/**
 * テーマ一覧をリロードする。
 *
 * `silent=true` のときはスケルトン表示 (isLoading) を切り替えない。
 * focus / visibilitychange / cursor-changed など、バックグラウンドで走る
 * 再取得経路でスケルトンを出すとカードがちらつくため、初回ロード以外は
 * 黙って差分更新する。
 */
async function loadThemes(opts: { silent?: boolean } = {}) {
  const silent = opts.silent === true || themes.value.length > 0
  if (!silent) isLoading.value = true
  try {
    // ローカルテーマと Windows スキームを並列取得。Windows スキーム取得はベストエフォート
    // (権限不足やキー不存在はログに残して空配列扱い) なので失敗してもライブラリ全体は表示する。
    const [localList, schemes] = await Promise.all([
      invokeTauri<IpcThemeSummary[]>('get_themes').catch((err) => {
        console.warn('[Library] get_themes failed:', err)
        return null
      }),
      listSchemes().catch((err) => {
        console.warn('[Library] list_windows_schemes failed (non-fatal):', err)
        return [] as IpcWindowsScheme[]
      }),
    ])

    const local: ThemeCardData[] = (localList ?? []).map((s) =>
      mapLocalSummaryToCard(s, locale.value),
    )

    // EasyCursorSwap が register_scheme で書き込んだスキームはローカルテーマと
    // 名前が一致するので除外する (重複表示防止)。
    const localNames = new Set(local.map((l) => l.name))
    const system: ThemeCardData[] = (schemes ?? [])
      .filter((s) => !localNames.has(s.name))
      .map(mapWindowsSchemeToCard)

    themes.value = [...local, ...system]
  } catch (err) {
    console.warn('[Library] loadThemes failed:', err)
    themes.value = []
  } finally {
    if (!silent) isLoading.value = false
  }
}

// 再読込シグナル購読 (P08a L6: useLibraryRefreshSignals に集約)。
// プレビューキャッシュの invalidate は B 経路 (Creator を経由しない外部変更用)。
const themePreviewCache = useThemePreviews()
useLibraryRefreshSignals({
  reload: (opts) => loadThemes(opts),
  onBeforeReload: () => {
    const previouslyActiveId = themes.value.find((t) => t.isActive)?.id
    if (previouslyActiveId) themePreviewCache.invalidate(previouslyActiveId)
  },
  locale,
})
// Explorer から渡された .cursorpack を受け取り、既存の importByPath フローに流す。
const cursorpackOpener = useCursorpackOpener((path) => {
  void importByPath(path)
})

onMounted(async () => {
  await loadThemes()
  await startFileDrop()
  // appSettings は本ページ起動時に常時必要 (active_theme_id 等)。初回ロードのみ取りに行く。
  // 既に Settings 画面などで取得済みならキャッシュが返る。
  await appSettings.load().catch(() => null)
  // .cursorpack のファイル関連付け経由インポートを開始
  void cursorpackOpener.start()
})

onUnmounted(() => {
  void cursorpackOpener.stop()
  stopFileDrop()
})
</script>

<template>
  <div class="library-host">
    <LibraryToolbar
      v-model:search-query="searchQuery"
      :import-busy="importBusy"
      @open-import="openImportDialog"
    />

    <!-- メインコンテンツ -->
    <div class="content">
      <div class="page-head">
        <div>
          <h1>{{ t('library.title') }}</h1>
          <p>{{ t('library.description', { count: themes.length }) }}</p>
        </div>
        <div class="right">
          <div class="btn-group">
            <button
              :class="['btn', { active: viewMode === 'grid' }]"
              aria-label="grid"
              @click="viewMode = 'grid'"
            >
              <UiIcon name="Grid" :size="14" />
            </button>
            <button
              :class="['btn', { active: viewMode === 'list' }]"
              aria-label="list"
              @click="viewMode = 'list'"
            >
              <UiIcon name="List" :size="14" />
            </button>
          </div>
        </div>
      </div>

      <LibraryFilterBar
        v-model:filter="filter"
        :counts="counts"
        :sort-label="sortLabel"
        @cycle-sort="cycleSort"
      />

      <!-- ローディング (スケルトン) -->
      <div v-if="isLoading" class="grid">
        <UiSkeletonCard v-for="i in 6" :key="i" />
      </div>

      <LibraryEmptyState
        v-else-if="themes.length === 0 && !searchQuery"
        @open-import="openImportDialog"
      />

      <!-- お気に入り 0 件 (ライブラリ自体は空ではない): 検索 0 件と同じ簡易表示 -->
      <div
        v-else-if="filter === 'favorites' && filteredThemes.length === 0 && !searchQuery"
        class="empty-state"
      >
        <UiIcon name="Star" :size="40" />
        <h3>{{ t('library.emptyFavorites') }}</h3>
      </div>

      <!-- 検索一致なし (空ライブラリではなく、検索 0 件) -->
      <div v-else-if="filteredThemes.length === 0" class="empty-state">
        <UiIcon name="Search" :size="40" />
        <h3>{{ t('library.emptySearch') }}</h3>
      </div>

      <!-- テーマグリッド -->
      <div v-else-if="viewMode === 'grid'" class="grid">
        <ThemeCard
          v-for="theme in filteredThemes"
          :key="theme.id"
          :theme="theme"
          @toggle-favorite="toggleFavorite"
          @show-details="showDetails"
        />
      </div>

      <!-- テーマ一覧 (Phase 5-3 / design/library-list.jsx) -->
      <div v-else class="lib-table" role="table" :aria-label="t('library.title')">
        <LibraryListHeader :sort-key="sortKey" :sort-dir="sortDir" @sort="sortBy" />

        <ThemeRow
          v-for="theme in filteredThemes"
          :key="theme.id"
          :theme="theme"
          @toggle-favorite="toggleFavorite"
          @show-details="showDetails"
        />
      </div>
    </div>

    <!-- 詳細モーダル (テーマカードのシェブロンで開く) -->
    <ThemeDetailModal
      :theme="detailTheme"
      :preview-map="detailPreviewMap"
      :preview-details="detailPreviewDetails"
      :busy-action="detailBusyAction"
      @close="closeDetails"
      @apply="applyFromDetail"
      @edit="editInCreator"
      @duplicate="duplicateTheme"
      @export-pack="exportTheme"
      @delete="deleteTheme"
    />

    <!-- 適用確認モーダル -->
    <Transition name="fade">
      <ApplyModal
        v-if="pendingTheme"
        :theme="pendingTheme"
        :busy="applyBusy"
        :signed-key-id="null"
        @cancel="cancelApply"
        @confirm="confirmApply"
      />
    </Transition>

    <!-- インポート衝突ダイアログ -->
    <Transition name="fade">
      <ImportConflictDialog
        v-if="conflictDialog"
        :info="conflictDialog.info"
        @cancel="conflictDialog = null"
        @overwrite="confirmConflictOverwrite"
      />
    </Transition>

    <!-- 適用エラー (簡易バナー、P08a: UiFloatingBanner に集約) -->
    <Transition name="fade">
      <UiFloatingBanner v-if="applyError" tone="danger" role="alert" @dismiss="applyError = null">
        <UiIcon name="Alert" :size="14" />
        {{ t('library.applyFailedBanner', { detail: applyError }) }}
      </UiFloatingBanner>
    </Transition>

    <LibraryDropOverlay :show="showDrop" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.library-host {
  @apply relative flex h-full flex-col;
}

.empty-state {
  @apply flex flex-col items-center justify-center gap-3 px-6 py-20 text-center text-fg-mute;
}
.empty-state h3 {
  @apply m-0 font-display text-[18px] font-semibold text-fg;
}
.empty-state p {
  @apply m-0 text-[13px] text-fg-dim;
}
.empty-state code {
  @apply font-mono text-accent;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
