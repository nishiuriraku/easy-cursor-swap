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
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { invokeTauri } from '~/composables/useTauri'
import { notify } from '~/composables/useNotify'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()
// UiIcon / ThemeCard / ApplyModal / AppStatusbar は Nuxt の自動インポートで解決される。

type FilterChip = 'all' | 'favorites' | 'recent' | 'unsigned'
type SortKey = 'name' | 'updated' | 'applied'

const themes = ref<ThemeCardData[]>([])
const searchQuery = ref('')
const filter = ref<FilterChip>('all')
const sortKey = ref<SortKey>('updated')
const viewMode = ref<'grid' | 'list'>('grid')
const isLoading = ref(true)
const showDrop = ref(false)

// 適用確認モーダル制御
const pendingTheme = ref<ThemeCardData | null>(null)
const applyBusy = ref(false)
const applyError = ref<string | null>(null)

// インポート衝突ダイアログ用
interface ConflictPending {
  path: string
  info: {
    id: string
    name: string
    version: string
    author: string | null
    roleCount: number
    existing: {
      name: string
      version: string
      author: string | null
      roleCount: number
    }
  }
}
const conflictDialog = ref<ConflictPending | null>(null)

// --- デモデータ (将来は invoke('get_themes')) ---
const demoThemes: ThemeCardData[] = [
  {
    id: 'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    name: 'Neon Glow',
    author: 'PixelMaster',
    version: '1.2.0',
    date: '2026-04-15',
    applyCount: 42,
    isFavorite: true,
    isActive: true,
    includedRoles: ['Arrow', 'Help', 'AppStarting', 'Wait', 'IBeam', 'Hand', 'No', 'SizeNS', 'SizeWE', 'SizeNWSE', 'SizeNESW', 'SizeAll'],
  },
  {
    id: 'b2c3d4e5-f6a7-8901-bcde-f23456789012',
    name: 'Minimal White',
    author: 'CleanDesign',
    version: '2.0.1',
    date: '2026-03-20',
    applyCount: 18,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow', 'Wait', 'IBeam', 'Hand', 'No'],
  },
  {
    id: 'c3d4e5f6-a7b8-9012-cdef-345678901234',
    name: 'ドット絵レトロ',
    author: 'RetroPixel',
    version: '1.0.0',
    date: '2026-05-01',
    applyCount: 7,
    isFavorite: true,
    isActive: false,
    includedRoles: ['Arrow', 'Help', 'AppStarting', 'Wait', 'Crosshair', 'IBeam', 'NWPen', 'No', 'SizeNS', 'SizeWE', 'SizeNWSE', 'SizeNESW', 'SizeAll', 'UpArrow', 'Hand', 'Pin', 'Person'],
  },
  {
    id: 'd4e5f6a7-b8c9-0123-defa-456789012345',
    name: 'Sakura Breeze',
    author: 'はむち',
    version: '1.1.0',
    date: '2026-04-28',
    applyCount: 31,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow', 'Help', 'Wait', 'IBeam', 'Hand', 'No', 'SizeNS', 'SizeWE'],
  },
  {
    id: 'e5f6a7b8-c9d0-1234-efab-567890123456',
    name: 'Cyber Punk 2077',
    author: 'NightCity',
    version: '3.0.0',
    date: '2026-02-14',
    applyCount: 56,
    isFavorite: true,
    isActive: false,
    includedRoles: ['Arrow', 'Help', 'AppStarting', 'Wait', 'Crosshair', 'IBeam', 'No', 'SizeNS', 'SizeWE', 'SizeNWSE', 'SizeNESW', 'SizeAll', 'Hand'],
  },
  {
    id: 'f6a7b8c9-d0e1-2345-fabc-678901234567',
    name: 'Monolith',
    author: 'studio.kane',
    version: '0.4.2',
    date: '2026-04-02',
    applyCount: 12,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow', 'Wait', 'IBeam', 'Hand', 'No', 'SizeAll', 'SizeNS', 'SizeWE', 'Crosshair', 'Help'],
  },
]

// --- フィルタ・ソート ---
const filteredThemes = computed(() => {
  let result = [...themes.value]

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(
      (t) => t.name.toLowerCase().includes(q) || (t.author?.toLowerCase().includes(q) ?? false),
    )
  }

  if (filter.value === 'favorites') result = result.filter((t) => t.isFavorite)
  // `recent` / `unsigned` は将来の API で追加。現状は all と同じ。

  result.sort((a, b) => {
    switch (sortKey.value) {
      case 'name':
        return a.name.localeCompare(b.name, 'ja')
      case 'updated':
        return b.date.localeCompare(a.date)
      case 'applied':
        return b.applyCount - a.applyCount
    }
  })

  return result
})

const counts = computed(() => ({
  all: themes.value.length,
  favorites: themes.value.filter((t) => t.isFavorite).length,
  recent: themes.value.filter((t) => t.applyCount > 0).length,
  unsigned: 2,
}))

const activeTheme = computed(() => themes.value.find((t) => t.isActive))
const totalStorageMb = 412 // TODO: invoke('get_storage_usage')

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
      await invokeTauri<void>('apply_windows_scheme', { name: target.name })
    } else {
      await invokeTauri<void>('apply_theme', { themeId: id })
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

function toggleFavorite(id: string) {
  const t = themes.value.find((x) => x.id === id)
  if (t) t.isFavorite = !t.isFavorite
}

function showDetails(id: string) {
  console.info('[Library] showDetails', id)
}

// ブラウザの DragEvent は dataTransfer.files に絶対パスを含めないため、
// Tauri v2 のウィンドウ drag-drop イベントで実ファイルパスを取得する。
// onMounted で `onDragDropEvent` 購読 → unlisten をクリーンアップ用に保持。
let unlistenDrop: (() => void) | null = null

interface InspectionResult {
  id: string
  name: string
  version: string
  author: string | null
  role_count: number
  existing: {
    name: string
    version: string
    author: string | null
    role_count: number
  } | null
}

async function importByPath(path: string) {
  try {
    // まず軽量検査して既存テーマと衝突するか確認
    const inspection = await invokeTauri<InspectionResult>('inspect_cursorpack', { path })
    if (inspection?.existing) {
      conflictDialog.value = {
        path,
        info: {
          id: inspection.id,
          name: inspection.name,
          version: inspection.version,
          author: inspection.author,
          roleCount: inspection.role_count,
          existing: {
            name: inspection.existing.name,
            version: inspection.existing.version,
            author: inspection.existing.author,
            roleCount: inspection.existing.role_count,
          },
        },
      }
      return
    }
    await actuallyImport(path)
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    applyError.value = `インポートに失敗: ${msg}`
    console.error('[Library] import failed:', err)
  }
}

async function actuallyImport(path: string) {
  const id = await invokeTauri<string>('import_cursorpack', { path })
  if (id) {
    console.info('[Library] imported', id, 'from', path)
    await loadThemes()
    const imported = themes.value.find((t) => t.id === id)
    void notify({
      title: 'EasyCursorSwap',
      body: imported
        ? t('library.notifyImported', { name: imported.name })
        : t('library.notifyImportedFallback'),
      level: 'success',
    })
  }
}

async function confirmConflictOverwrite() {
  const pending = conflictDialog.value
  if (!pending) return
  conflictDialog.value = null
  try {
    await actuallyImport(pending.path)
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    applyError.value = `インポートに失敗: ${msg}`
  }
}

async function openImportDialog() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
    })
    if (!selected) return
    const paths = Array.isArray(selected) ? selected : [selected]
    for (const p of paths) await importByPath(p)
  } catch (err) {
    console.warn('[Library] dialog unavailable:', err)
  }
}

const sortLabel = computed(() => ({
  name: t('library.sortName'),
  updated: t('library.sortUpdated'),
  applied: t('library.sortApplied'),
}[sortKey.value]))

function cycleSort() {
  const order: SortKey[] = ['updated', 'name', 'applied']
  const idx = order.indexOf(sortKey.value)
  sortKey.value = order[(idx + 1) % order.length]!
}

/** Rust から `get_themes` を取得し、UI 形状にマップする */
interface IpcThemeSummary {
  id: string
  name: string
  author: string | null
  version: string
  created_at: string
  is_active: boolean
  is_favorite: boolean
  apply_count: number
  included_roles: string[]
  path: string
}

/** `list_windows_schemes` のレスポンス。Windows レジストリ HKCU\Cursors\Schemes 由来。 */
interface IpcWindowsScheme {
  name: string
  cursor_paths: Record<string, string>
  role_count: number
}

/**
 * Windows レジストリのスキームを ThemeCardData に変換する。
 *
 * - id は `windows:<name>` のプレフィックスでローカルテーマと衝突を避ける
 * - kind: 'system' を立てて UI 側でバッジ・編集不可表示に切り替える
 * - included_roles は cursor_paths のキー (空でないもの) を使う
 * - active 判定は `HKCU\Control Panel\Cursors` の (Default) 値と比較できるが
 *   今は読み取り経路を増やさず、適用後にイベントで再ロードする
 */
function mapWindowsSchemeToCard(s: IpcWindowsScheme): ThemeCardData {
  const includedRoles = Object.entries(s.cursor_paths)
    .filter(([, path]) => path.length > 0)
    .map(([role]) => role)
  return {
    id: `windows:${s.name}`,
    name: s.name,
    author: 'Windows',
    version: '—',
    date: '',
    applyCount: 0,
    isFavorite: false,
    isActive: false,
    includedRoles,
    kind: 'system',
  }
}

async function loadThemes() {
  isLoading.value = true
  try {
    // ローカルテーマと Windows スキームを並列取得。Windows スキーム取得はベストエフォート
    // (権限不足やキー不存在はログに残して空配列扱い) なので失敗してもライブラリ全体は表示する。
    const [localList, schemes] = await Promise.all([
      invokeTauri<IpcThemeSummary[]>('get_themes').catch((err) => {
        console.warn('[Library] get_themes failed:', err)
        return null
      }),
      invokeTauri<IpcWindowsScheme[]>('list_windows_schemes').catch((err) => {
        console.warn('[Library] list_windows_schemes failed (non-fatal):', err)
        return [] as IpcWindowsScheme[]
      }),
    ])

    const local: ThemeCardData[] = (localList ?? []).map((t) => ({
      id: t.id,
      name: t.name,
      author: t.author,
      version: t.version,
      date: t.created_at,
      applyCount: t.apply_count,
      isFavorite: t.is_favorite,
      isActive: t.is_active,
      includedRoles: t.included_roles,
      kind: 'local' as const,
    }))

    // EasyCursorSwap が register_scheme で書き込んだスキームはローカルテーマと
    // 名前が一致するので除外する (重複表示防止)。
    const localNames = new Set(local.map((l) => l.name))
    const system: ThemeCardData[] = (schemes ?? [])
      .filter((s) => !localNames.has(s.name))
      .map(mapWindowsSchemeToCard)

    if (local.length > 0 || system.length > 0) {
      themes.value = [...local, ...system]
    } else if (themes.value.length === 0) {
      // Tauri 未接続 or 空ライブラリ → デモ表示
      themes.value = demoThemes
    }
  } catch (err) {
    console.warn('[Library] loadThemes failed entirely, using demo:', err)
    if (themes.value.length === 0) themes.value = demoThemes
  } finally {
    isLoading.value = false
  }
}

// 外部カーソル変更検知 — Rust 側で SPI_SETCURSORS を購読し、変更があれば UI 更新
let unlistenCursorChange: (() => void) | null = null
async function setupCursorChangeListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenCursorChange = await listen('cursor-changed', () => {
      console.info('[Library] cursor-changed event received → reload')
      void loadThemes()
    })
  } catch (err) {
    console.warn('[Library] cursor-changed listener unavailable:', err)
  }
}

// --- Tauri v2 ウィンドウドラッグ&ドロップイベント購読 ---
async function setupTauriDrop() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const win = getCurrentWindow()
    unlistenDrop = await win.onDragDropEvent((event) => {
      const p = event.payload
      if (p.type === 'enter' || p.type === 'over') {
        showDrop.value = true
      } else if (p.type === 'leave') {
        showDrop.value = false
      } else if (p.type === 'drop') {
        showDrop.value = false
        const paths = (p.paths ?? []).filter((path: string) =>
          path.toLowerCase().endsWith('.cursorpack'),
        )
        if (paths.length === 0) {
          applyError.value = t('library.importNotPack')
          return
        }
        for (const path of paths) void importByPath(path)
      }
    })
  } catch (err) {
    console.warn('[Library] Tauri drop API unavailable:', err)
  }
}

onMounted(async () => {
  await loadThemes()
  await setupTauriDrop()
  await setupCursorChangeListener()
})

onUnmounted(() => {
  if (unlistenDrop) {
    unlistenDrop()
    unlistenDrop = null
  }
  if (unlistenCursorChange) {
    unlistenCursorChange()
    unlistenCursorChange = null
  }
})
</script>

<template>
  <div class="library-host">
    <!-- ツールバー -->
    <div class="toolbar">
      <div class="bcrumb">
        <span class="crumb">{{ t('library.breadcrumbWorkspace') }}</span>
        <span class="sep">/</span>
        <span class="crumb active">{{ t('library.title') }}</span>
      </div>
      <div class="search">
        <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
        <input
          v-model="searchQuery"
          :placeholder="t('library.searchPlaceholder')"
          :aria-label="t('common.search')"
        />
        <span class="kbd">⌘K</span>
      </div>
      <div class="tb-actions">
        <button class="btn ghost" @click="openImportDialog">
          <UiIcon name="Import" :size="14" />{{ t('common.import') }}
        </button>
        <NuxtLink class="btn primary" to="/creator">
          <UiIcon name="Plus" :size="14" />{{ t('library.new') }}
        </NuxtLink>
      </div>
    </div>

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

      <!-- フィルタチップ -->
      <div class="filters" role="group" :aria-label="t('common.search')">
        <div class="chips" role="group" aria-label="フィルター">
          <button
            :class="['chip', { active: filter === 'all' }]"
            :aria-pressed="filter === 'all'"
            @click="filter = 'all'"
          >
            {{ t('library.filterAll') }}<span class="num" aria-hidden="true">{{ counts.all }}</span>
          </button>
          <button
            :class="['chip', { active: filter === 'favorites' }]"
            :aria-pressed="filter === 'favorites'"
            @click="filter = 'favorites'"
          >
            <UiIcon name="Star" :size="11" aria-hidden="true" />{{ t('library.filterFavorites') }}<span class="num" aria-hidden="true">{{ counts.favorites }}</span>
          </button>
          <button
            :class="['chip', { active: filter === 'recent' }]"
            :aria-pressed="filter === 'recent'"
            @click="filter = 'recent'"
          >
            {{ t('library.filterRecent') }}<span class="num" aria-hidden="true">{{ counts.recent }}</span>
          </button>
          <button
            :class="['chip', { active: filter === 'unsigned' }]"
            :aria-pressed="filter === 'unsigned'"
            @click="filter = 'unsigned'"
          >
            {{ t('library.filterUnsigned') }}<span class="num" aria-hidden="true">{{ counts.unsigned }}</span>
          </button>
        </div>
        <div class="spacer-x" />
        <div class="sort">
          <span class="lbl" aria-hidden="true">{{ t('library.sort') }}</span>
          <button class="btn ghost" style="height: 28px" :aria-label="`${t('library.sort')}: ${sortLabel}`" @click="cycleSort">
            <UiIcon name="Sort" :size="13" aria-hidden="true" />{{ sortLabel }}
            <UiIcon name="ChevD" :size="11" aria-hidden="true" />
          </button>
        </div>
      </div>

      <!-- ローディング (スケルトン) -->
      <div v-if="isLoading" class="grid">
        <div v-for="i in 6" :key="i" class="card skeleton-card" />
      </div>

      <!-- 空状態 -->
      <div v-else-if="filteredThemes.length === 0" class="empty-state">
        <UiIcon name="Pkg" :size="48" />
        <h3>{{ searchQuery ? t('library.emptySearch') : t('library.emptyTitle') }}</h3>
        <p v-if="!searchQuery">{{ t('library.emptySubText') }}</p>
      </div>

      <!-- テーマグリッド (td-grid-host: 詳細ドロワー展開時に 2 列スパンを許可する) -->
      <div v-else class="grid td-grid-host">
        <ThemeCard
          v-for="theme in filteredThemes"
          :key="theme.id"
          :theme="theme"
          @apply="requestApply"
          @toggle-favorite="toggleFavorite"
          @show-details="showDetails"
        />
      </div>
    </div>

    <!-- ステータスバー -->
    <AppStatusbar
      :items="[
        { dot: true, text: activeTheme ? `Active: ${activeTheme.name}` : 'Active: Windows 既定' },
        { text: 'ダークモード自動切替: ON' },
        { text: '署名検証: 有効' },
        { text: `~/.custom_cursors/ — ${totalStorageMb} MB` },
      ]"
    />

    <!-- 適用確認モーダル -->
    <Transition name="fade">
      <ApplyModal
        v-if="pendingTheme"
        :theme="pendingTheme"
        :busy="applyBusy"
        :signed-key-id="pendingTheme.author === 'PixelMaster' ? '7f3a9c…b21e' : null"
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

    <!-- 適用エラー (簡易バナー) -->
    <Transition name="fade">
      <div v-if="applyError" class="apply-error" role="alert">
        <UiIcon name="Alert" :size="14" />
        適用に失敗しました: {{ applyError }}
        <button class="btn ghost" style="height: 24px; margin-left: auto" @click="applyError = null">
          <UiIcon name="X" :size="11" />
        </button>
      </div>
    </Transition>

    <!-- ドロップオーバーレイ -->
    <Transition name="fade">
      <div v-if="showDrop" class="drop">
        <div class="drop-inner">
          <UiIcon name="Pkg" :size="56" class="ghost-icon" />
          <h3>{{ t('library.drop') }}</h3>
          <p>{{ t('library.dropSub') }}</p>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.library-host {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  text-align: center;
  color: var(--fg-mute);
  gap: 12px;
}
.empty-state h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 18px;
  color: var(--fg);
  font-weight: 600;
}
.empty-state p {
  margin: 0;
  font-size: 13px;
  color: var(--fg-dim);
}
.empty-state code {
  font-family: var(--font-mono);
  color: var(--accent);
}

.skeleton-card {
  height: 280px;
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.02) 0%,
    rgba(255, 255, 255, 0.04) 50%,
    rgba(255, 255, 255, 0.02) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.4s ease-in-out infinite;
}
@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}

.apply-error {
  position: fixed;
  bottom: 48px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 90;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  font-size: 12.5px;
  background: rgba(255, 107, 138, 0.12);
  border: 1px solid rgba(255, 107, 138, 0.4);
  border-radius: 8px;
  color: #ffb8c5;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  box-shadow: var(--shadow-2);
  min-width: 320px;
  max-width: 80%;
}
</style>
