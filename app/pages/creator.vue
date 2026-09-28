<script setup lang="ts">
/**
 * クリエイターモード。
 *
 * 2 カラム構成 (Assign タブ):
 *  - 左:  17 役割リスト (filled / empty ドット付き)
 *  - 中央: ビッグプレビュー + 6 サイズストリップ + リサンプル切替
 *  ホットスポット / 影フラグはメタデータタブに集約。
 *
 * 画像アップロード / .cur ビルド / 署名生成は全て IPC 配線済み:
 *  - useCreatorImport (PNG/SVG/.cur/.ico 単一ファイル取込)
 *  - useCreatorBulkImportFlow (複数ファイル / .cursorpack の bulk 取込)
 *  - useCreatorExport (Rust 側 export_cursorpack_streamed への引き渡し)
 */
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'
import type { Hotspot } from '~/composables/useCreatorAssets'
import type { CursorPreviewAsset } from '~/components/preview/CursorPreview.vue'

const { t } = useI18n()

const { info: keystoreInfo, refresh: refreshKeystore } = useKeystore()
const hasKeystoreSigning = computed(() => keystoreInfo.value.has_keypair)

type RoleStatus = 'filled' | 'empty'
type ResampleMode = 'lanczos' | 'nearest'

const SIZES = [32, 48, 64, 96, 128, 256] as const
type TabId = 'assign' | 'metadata'

/**
 * Creator のセッションステージ。
 * - `start`: design/empty-states.jsx::CreatorStart のヒーロー画面。
 *            「新規作成」を押すと editing に遷移する。
 * - `editing`: 既存の 17 役割割り当て + メタデータ編集 UI。
 *
 * Clear ボタンで editing → start に戻れる。アセットとメタデータは戻る際にクリアする。
 */
type CreatorStage = 'start' | 'editing'
const stage = ref<CreatorStage>('start')

/**
 * 新規作成モーダル (画像選択 → 編集画面) の開閉制御。
 * デザイン要件: 「新規作成」を押したらまずモーダルでベース画像を選ばせ、
 * Arrow ロールに割り当ててから編集画面に遷移する。
 */
const newThemeModalOpen = ref(false)

/**
 * 解像度ごとに別画像を割り当てる詳細フローのトグル。
 *
 * 1 枚画像から 6 解像度を自動生成するのが基本。詳細設定を ON にすると
 * SizeStrip と Per-size Hotspot トグルが現れて、サイズ別の上書きができる。
 */
const showAdvancedResolutions = ref(false)

// useSeoMeta は Tauri アプリでは document.title 等の最小用途。Nuxt ページ規約に従って
// title / description / ogImage を定義しておく。
useSeoMeta({
  title: 'EasyCursorSwap — Creator',
  description: t('creator.appDescription'),
  ogImage: '/icon.png',
})

/* === useSeoMeta は title 用途 (上で設定済) ============================================
 * 以降は通常のページロジック。`stage` ref に応じて `<template>` 内で
 * CreatorStartScreen と編集 UI を切替える。useSeoMeta 設定はファイル冒頭で完結している。
 * ====================================================================================== */

// --- ロール状態は useCreatorAssets.assigned を Single Source of Truth として導出する ---
// (以前は filledRoles / partialRoles / filledSizesByRole をハードコードで初期化していたが、
// 画像未インポート時に虚偽の "filled" 表示が出る原因になっていたため computed に変更)

const activeTab = ref<TabId>('assign')
const activeRoleId = ref<string>('Arrow')
const activeSize = ref<number>(64)
const resample = ref<ResampleMode>('lanczos')
// 解像度別のホットスポット上書きを有効化するトグル。デフォルト OFF。
// ON のときは writeActiveHotspot が assigned[role].sized 側に書き込み、
// activeHotspotModel / sizedOverrideActive / enableSizedOverride で制御される。
const perSizeHotspot = ref(false)
/**
 * `?editPath` で既存テーマを Creator に取り込んでいる場合、その元テーマの UUID。
 * SaveDestinationModal の「上書き保存 / 複製」選択肢の表示と、
 * Rust 側 export 時の `existing_theme_id` 引き継ぎに使う。
 *
 * 設定タイミング: `?editPath` 経由の `onMounted` のみ。
 * クリアタイミング:
 *  - `resetCreator()` (= start ステージに戻る)
 *  - `onDuplicateExistingFromStart()` (= 別テーマを複製として新規作成)
 *  - `dispatchBulkPaths()` 内で `.cursorpack` を取り込んだ瞬間 (= ソースが入れ替わる)
 */
const sourceThemeId = ref<string | null>(null)
/** SaveDestinationModal の開閉と初期 destination 制御 */
const saveModalOpen = ref(false)
const saveModalDefault = ref<'file' | 'library' | 'libraryAndApply'>('library')

/**
 * 役割ごとのインポート済みアセットを `useCreatorAssets` 経由で集約管理する。
 * 単一インポート / 一括インポート / `.cursorpack` 取り込みなどの経路はすべてここに合流する。
 */
const creatorAssets = useCreatorAssets()
const { assigned, setAsset, assignedRoleCount, arrowAssigned, toExportPayload } = creatorAssets

// メタデータ入力欄の state はまとめて composable に寄せる (Phase: file-splits)。
// 個別 ref alias を export しているのは、`<CreatorMetadataPane v-model:meta-*>` と
// useCreatorExport / useCreatorBulkImportFlow が個別の ref を期待しているため。
const metaState = useCreatorMetaState()
const {
  name: metaName,
  nameEn: metaNameEn,
  author: metaAuthor,
  version: metaVersion,
  description: metaDescription,
  shadowEnabled,
} = metaState

/**
 * 破棄ガード (P08a C1: useCreatorDiscardGuard に集約)。
 */
const {
  hasUnsavedEdits,
  discardDialogOpen,
  discardDialogMode,
  bypassUnsavedGuard,
  requestReset,
  onDiscardConfirm,
  onDiscardCancel,
  scheduleNavigateAfterSave,
} = useCreatorDiscardGuard({
  stage,
  assignedRoleCount,
  isMetaDirty: computed(() => metaState.isDirty.value),
  onReset: () => resetCreator(),
})

// --- 一括インポート ---
const bulkImport = useBulkImport()
// LD5: resolve/parse 中 (プレビューモーダルが開く前) のオーバーレイ表示用に
// busy / progress を取り出す。これが無いと解析中 UI が無反応に見える。
const { busy: bulkBusy, progress: bulkProgress } = bulkImport
const pickers = useCreatorPickers()

// 既存テーマ複製ピッカー
const themePickerOpen = ref(false)
const themePickerSelected = ref<string | null>(null)
const { themes: pickerThemes, refresh: refreshPickerThemes } = useThemes()

// 公式インデックス由来 (kind === 'marketplace') は複製元として選ばせない。
// repackage_theme 自体はローカルファイル経由で実行可能だが、UI 上「公式テーマを
// ベースに新規作成」を許すと派生作品の出所が曖昧になりやすいため明示的に除外する。
const duplicatePickerThemes = computed(() =>
  pickerThemes.value.filter((th) => th.kind !== 'marketplace'),
)

const existingRolesSet = computed(() => new Set(Object.keys(assigned.value)))

// --- 計算プロパティ ---
const activeRole = computed<CursorRoleDef>(
  () => CURSOR_ROLES.find((r) => r.id === activeRoleId.value) ?? CURSOR_ROLES[0]!,
)

/** assigned に存在するロール ID のセット (filled/empty 判定の唯一の根拠)。 */
const filledRoleSet = computed(() => new Set(Object.keys(assigned.value)))

function statusOf(id: string): RoleStatus {
  return filledRoleSet.value.has(id) ? 'filled' : 'empty'
}

const filledCount = computed(() => filledRoleSet.value.size)
const tabs = computed<Array<{ id: TabId; label: string; count?: string }>>(() => [
  { id: 'assign', label: t('creator.tabAssign'), count: `${filledCount.value}/17` },
  { id: 'metadata', label: t('creator.tabMetadata') },
])

function selectRole(id: string) {
  activeRoleId.value = id
}

/**
 * ホットスポット編集状態 (P08a C2: useCreatorHotspotState に集約)。
 * perSizeHotspot=ON かつ sized override があれば sized 側、それ以外は primary。
 */
const {
  activeHotspot,
  activeHotspotModel,
  sizedOverrideActive,
  canEditSizedOverride,
  activeAniFrames,
  activeAniSourcePath,
  writeActiveHotspot,
  enableSizedOverride,
  centerHotspot,
/**
 * プレビュー Blob URL 派生状態 (P08a C3: useCreatorPreviewUrls に集約)。
 */
const { filledSizes, activePreviewUrl, activePreviewAsset, sizePreviewMap } =
  useCreatorPreviewUrls({
    creatorAssets,
    activeRoleId,
    activeSize,
    activeRole,
    activeAniFrames,
  })


onUnmounted(() => {
  stopFileDrop()
})

/**
 * 詳細設定で解像度 (`activeSize`) を切り替える。
 * ratio は size 非依存なので再投影不要。
 */
function selectSize(s: number) {
  activeSize.value = s
}


function isRequired(id: string): boolean {
  return id === 'Arrow'
}

// 起動時に keystore 状態を取得して「署名 & エクスポート」ボタンの表示判定に使う
onMounted(async () => {
  void refreshKeystore()
  void startFileDrop()
  // ライブラリの「Creator で編集」から `?editPath=...` で .cursorpack を渡された場合は
  // 自動ロードして editing ステージを開く。
  const route = useRoute()
  const editPath = (route.query.editPath as string | undefined) ?? null
  if (editPath) await loadFromEditPath(editPath)
})

// --- 画像インポート / エクスポート / 一括インポートのフロー制御 ---
// 詳細は composable に分離 (Phase 3c)。creator.vue は組み立てだけを担当する。

const {
  importBusy,
  importMessage,
  sanitizedRemovals,
  applyImportedRaster,
  handleFileInput: onFileChange,
} = useCreatorImport({
  creatorAssets,
  activeRoleId,
  t,
})

const {
  exportBusy,
  exportMessage,
  failedApplyThemeId,
  exportProgress,
  currentBuildId,
  cancelExport,
  executeSave,
  retryApply,
} = useCreatorExport({
  creatorAssets,
  metaNameEn,
  metaAuthor,
  metaVersion,
  metaDescription,
  sourceThemeId,
  shadowEnabled,
  resample,
  t,
})

function onToolbarSave() {
  saveModalOpen.value = true
}

/**
 * SaveDestinationModal の submit を受けて保存実行 → 成功時はライブラリへ自動遷移する。
 *
 *  - `executeSave` が 'ok' を返した場合のみ遷移 (apply-error / failed は Creator に留まる)。
 *  - 直後に router.push すると成功トーストが描画される間もなく Creator が unmount される
 *    ため、~1 秒の遅延を入れてユーザーがフィードバックを視認できるようにする。
 *  - 遷移時は破棄確認ダイアログをスキップする (bypassUnsavedGuard を立てる)。
 */
async function handleSaveSubmit(
  payload: import('~/composables/useCreatorExport').SaveSubmitPayload,
) {
  saveModalOpen.value = false
  const status = await executeSave(payload)
  if (status !== 'ok') return
  // 保存成功 → ライブラリへ自動遷移。トーストが見える程度の短い遅延を挟む。
  scheduleNavigateAfterSave('/', 1000)
}

const {
  bulkModalOpen,
  bulkResolved,
  bulkCursorpack,
  bulkSourceLabel,
  dispatchBulkPaths,
  runBulkResolve,
  applyBulkImport,
  cancelBulkImport,
} = useCreatorBulkImportFlow({
  bulkImport,
  creatorAssets,
  sourceThemeId,
  metaName,
  metaNameEn,
  metaAuthor,
  metaVersion,
  metaDescription,
  importBusy,
  importMessage,
  sanitizedRemovals,
})

/** メイン取込ダイアログ → 拡張子 dispatch */
async function pickBulkAuto() {
  const paths = await pickers.pickAssetFiles()
  if (!paths) return
  await dispatchBulkPaths(paths)
}

/** フォルダから取込 (chevron サブメニュー / 新規作成モーダル経由)。 */
async function pickBulkFolder() {
  const picked = await pickers.pickFolder()
  if (!picked) return
  await runBulkResolve([picked], false, `📁 ${picked}`)
}

// Tauri ウィンドウ Drag & Drop (P08a: useTauriFileDrop に集約)。
// ブラウザの DragEvent は dataTransfer.files に絶対パスを含めないため、Library
// 画面と同じく Tauri v2 の onDragDropEvent で実パスを受け取って dispatchBulkPaths
// に流す。start ステージで受けた場合は NewThemeStartModal をスキップして editing
// へ直接遷移する (ユーザー選択: 両方で受け付ける)。
//
// dispatchBulkPaths は拡張子別に「.cursorpack 単独」と「bulk_resolve」に分岐する
// ので、ここでは拡張子ごとの分岐を再実装しない。サポート外の拡張子は
// dispatchBulkPaths 側で「マッチ 0 件」の空 preview として扱われる。
const {
  showDrop,
  start: startFileDrop,
  stop: stopFileDrop,
} = useTauriFileDrop({
  accept: ['png', 'svg', 'cur', 'ico', 'ani', 'cursorpack'],
  onDrop: (paths) => {
    // start 画面で受けたときは NewThemeStartModal を閉じて editing に遷移。
    // dispatchBulkPaths 内で `.cursorpack` の場合は bulkModalOpen が立ち、
    // bulk_resolve 経路でも同様に preview が開くため stage 遷移は先んじて行う。
    if (stage.value === 'start') {
      newThemeModalOpen.value = false
      stage.value = 'editing'
    }
    void dispatchBulkPaths(paths)
  },
  onRejected: () => {
    importMessage.value = t('creator.errDropUnsupported')
  },
})

/**
 * Creator の編集状態を完全にリセットして初期画面に戻す。
 *
 * アセット・メタデータ・インポートメッセージ・進捗バナーを全てクリアして
 * 「Clear」ボタンを押した瞬間に Cmd+N と同等の状態に戻す。プレビュー Blob URL も
 * 解放してメモリリークを防ぐ。assigned をクリアすれば filledRoleSet / filledSizes の
 * computed が自動的に空に戻るので、別途 filled* state をクリアする必要はない。
 */
function resetCreator() {
  for (const role of Object.keys(assigned.value)) {
    creatorAssets.removeAsset(role)
  }
  activeRoleId.value = 'Arrow'
  sourceThemeId.value = null
  saveModalDefault.value = 'library'
  saveModalOpen.value = false
  activeSize.value = 64
  metaState.reset()
  importMessage.value = null
  exportMessage.value = null
  exportProgress.value = null
  activeTab.value = 'assign'
  stage.value = 'start'
}

/**
 * 新規作成 / 複製フロー (P08a C5: useCreatorStartFlow に集約)。
 */
const {
  onStartNew,
  onNewThemePickFiles,
  onNewThemePickFolder,
  onNewThemeStartEmpty,
  onNewThemeCancel,
  onDuplicateExistingFromStart,
  onThemePickerSelect,
  onThemePickerCancel,
  loadFromEditPath,
} = useCreatorStartFlow({
  stage,
  newThemeModalOpen,
  themePickerOpen,
  themePickerSelected,
  sourceThemeId,
  saveModalDefault,
  bulkFlow: {
    bulkModalOpen,
    bulkCursorpack,
    bulkResolved,
    bulkSourceLabel,
    dispatchBulkPaths,
  },
  bulkImport,
  pickBulkAuto,
  pickBulkFolder,
  refreshPickerThemes,
  importMessage,
  t,
})
</script>

<template>
  <div class="creator-host">
    <CreatorStartScreen
      v-if="stage === 'start'"
      @start-new="onStartNew"
      @duplicate-existing="onDuplicateExistingFromStart"
    />
    <template v-else>
      <CreatorToolbar
        :meta-name="metaName"
        :meta-version="metaVersion"
        :has-keystore-signing="hasKeystoreSigning"
        :export-busy="exportBusy"
        :arrow-assigned="arrowAssigned"
        @reset="requestReset"
        @save="onToolbarSave"
        @bulk-auto="pickBulkAuto"
        @bulk-folder="pickBulkFolder"
      />

      <!-- タブバー (P08a V1) -->
      <CreatorTabBar v-model="activeTab" :tabs="tabs" />

      <CreatorMetadataPane
        v-if="activeTab === 'metadata'"
        v-model:meta-name="metaName"
        v-model:meta-name-en="metaNameEn"
        v-model:meta-author="metaAuthor"
        v-model:meta-version="metaVersion"
        v-model:meta-description="metaDescription"
        v-model:shadow-enabled="shadowEnabled"
        :arrow-assigned="arrowAssigned"
        :assigned-role-count="assignedRoleCount"
        :export-progress="exportProgress"
        :export-busy="exportBusy"
        @cancel-export="cancelExport"
      />

      <!-- 2 カラムグリッド (assign タブのみ) -->
      <div v-if="activeTab === 'assign'" class="creator-grid">
        <CreatorRoleList
          :filled-count="filledCount"
          :active-role-id="activeRoleId"
          :status-of="statusOf"
          @select="selectRole"
        />

        <CreatorEditorCanvas
          :active-role="activeRole"
          :active-size="activeSize"
          :preview-asset="activePreviewAsset"
          :hotspot="activeHotspot"
          :reference-size="assigned[activeRoleId]?.primarySize || activeSize"
          :filled-sizes="filledSizes"
          :size-preview-map="sizePreviewMap"
          :arrow-assigned="arrowAssigned"
          :is-required-role="isRequired(activeRole.id)"
          v-model:show-advanced-resolutions="showAdvancedResolutions"
          v-model:resample="resample"
          @update:hotspot="writeActiveHotspot"
          @select-size="selectSize"
          @file-selected="onFileChange"
          @next-tab="activeTab = 'metadata'"
        />
      </div>

      <BulkImportPreviewModal
        :open="bulkModalOpen"
        :resolved="bulkResolved"
        :cursorpack="bulkCursorpack"
        :existing-roles="existingRolesSet"
        :source-label="bulkSourceLabel"
        @apply="applyBulkImport"
        @cancel="cancelBulkImport"
      />
    </template>

    <!-- LD5: 一括インポート (resolve/parse) 中の進捗オーバーレイ。start / editing どちらの
         stage でも表示する必要がある — 新規作成フローは resolve 完了後に editing へ遷移するため、
         editing ブロック (v-else) 内に置くと resolve 中はマウントされず無反応に見える。
         .bulk-overlay は position:fixed なので v-if/v-else チェーンの外・任意位置に置いてよい。 -->
    <BulkImportProgressOverlay
      v-if="bulkBusy"
      :progress="bulkProgress"
      @cancel="bulkImport.cancel"
    />

    <!--
      新規作成モーダルは v-if/v-else チェーンの外に置く。
      間に挟むと v-if と v-else が直接の兄弟でなくなり Vue コンパイラが落ちるため。
      モーダルは `:open` で表示制御するので stage に依存せずどちらでもマウントできる。
    -->
    <NewThemeStartModal
      :open="newThemeModalOpen"
      @pick-files="onNewThemePickFiles"
      @pick-folder="onNewThemePickFolder"
      @start-empty="onNewThemeStartEmpty"
      @cancel="onNewThemeCancel"
    />

    <SaveDestinationModal
      :open="saveModalOpen"
      :has-keystore-signing="hasKeystoreSigning"
      :source-theme-id="sourceThemeId"
      :default-destination="saveModalDefault"
      :meta-name="metaName"
      @cancel="saveModalOpen = false"
      @submit="(payload) => void handleSaveSubmit(payload)"
    />

    <!--
      既存テーマ複製ピッカー。「既存テーマを複製して編集」CTA から開く。
      選択時は `onThemePickerSelect` で repackage_theme → dispatchBulkPaths に流れる。
    -->
    <ThemePickerModal
      v-if="themePickerOpen"
      :model-value="themePickerSelected"
      :themes="duplicatePickerThemes"
      :title="t('creatorStart.duplicatePickerTitle')"
      :sub="t('creatorStart.duplicatePickerSub')"
      :show-clear="false"
      :show-footer-cancel="false"
      @update:model-value="onThemePickerSelect"
      @cancel="onThemePickerCancel"
    />

    <!--
      編集破棄ダイアログ。Clear ボタン (mode='clear') と onBeforeRouteLeave
      による画面遷移 (mode='navigate') の両方で再利用する。
      hasUnsavedEdits=false のときは開かない (requestReset / route guard が直接実行)。
    -->
    <DiscardEditDialog
      :open="discardDialogOpen"
      :mode="discardDialogMode"
      @confirm="onDiscardConfirm"
      @cancel="onDiscardCancel"
    />

    <!--
      Tauri ウィンドウ DnD のフィードバック (P08a: LibraryDropOverlay を再利用。
      Creator では PNG/SVG/CUR/ICO/ANI/.cursorpack 全般を受け付けるので文言を専用化)。
    -->
    <LibraryDropOverlay
      :show="showDrop"
      :title="t('creator.dropTitle')"
      :sub="t('creator.dropSub')"
      icon="Import"
    />

    <!-- インポート結果メッセージ (画面下部のポップアップ、UiFloatingBanner) -->
    <Transition name="fade">
      <UiFloatingBanner
        v-if="importMessage"
        tone="accent"
        :icon="importMessage.startsWith(t('creator.errImportPrefix')) ? 'Alert' : 'Check'"
        @dismiss="importMessage = null"
      >
        {{ importMessage }}
      </UiFloatingBanner>
    </Transition>

    <!--
      エクスポート結果トースト。元々は CreatorMetadataPane 内に居たが、
      assign タブで保存したとき (= metadata pane が v-if で unmount されている時) に
      トーストが描画されない問題があったため、タブに依存しないこの層に持ち上げた。
    -->
    <Transition name="fade">
      <UiFloatingBanner
        v-if="exportMessage"
        tone="accent"
        :icon="exportMessage.startsWith(t('creator.exportFailPrefix')) ? 'Alert' : 'Check'"
        @dismiss="exportMessage = null"
      >
        {{ exportMessage }}
        <template v-if="failedApplyThemeId" #actions>
          <button class="btn ghost" style="height: 24px" @click="retryApply">
            {{ t('saveModal.retryApply') }}
          </button>
        </template>
      </UiFloatingBanner>
    </Transition>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.creator-host {
  @apply relative flex h-full flex-col;
}

/* (P08a: 未使用 scoped CSS 9 グループ + インポート結果ポップアップを削除。
 * `.export-progress*` / `.metadata-*` はテンプレ参照 0 件のため除去) */

.creator-grid {
  @apply grid min-h-0 flex-1 border-t border-line;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
}
@media (max-width: 880px) {
  .creator-grid {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
  }
}

/* (P08a: Tauri DnD オーバーレイは LibraryDropOverlay に集約し、ここから削除) */

.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
