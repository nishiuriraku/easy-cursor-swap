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
type ResampleMode = 'lanczos' | 'nearest' | 'auto'

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
const saveModalDefault = ref<'file' | 'library' | 'libraryAndApply'>('file')

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

// --- 一括インポート ---
const bulkImport = useBulkImport()
const pickers = useCreatorPickers()

// 既存テーマ複製ピッカー
const themePickerOpen = ref(false)
const themePickerSelected = ref<string | null>(null)
const { themes: pickerThemes, refresh: refreshPickerThemes } = useThemes()

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

/**
 * 現在ロールに「埋まっているサイズ」を assigned から導出。
 * primary は必ず含まれ、sized オーバーライドのキーを和集合で足す。
 */
const filledSizes = computed<number[]>(() => {
  const a = assigned.value[activeRoleId.value]
  if (!a) return []
  const set = new Set<number>([a.primarySize])
  if (a.sized) for (const k of a.sized.keys()) set.add(k)
  return Array.from(set).sort((x, y) => x - y)
})

function selectRole(id: string) {
  activeRoleId.value = id
}

/** ロール一覧で ↑↓ Home End キー操作 — リストボックス相当のフォーカス移動。 */
function onRoleListKeydown(e: KeyboardEvent) {
  const idx = CURSOR_ROLES.findIndex((r) => r.id === activeRoleId.value)
  if (idx === -1) return
  let next = idx
  if (e.key === 'ArrowDown' || e.key === 'j') next = Math.min(idx + 1, CURSOR_ROLES.length - 1)
  else if (e.key === 'ArrowUp' || e.key === 'k') next = Math.max(idx - 1, 0)
  else if (e.key === 'Home') next = 0
  else if (e.key === 'End') next = CURSOR_ROLES.length - 1
  else return
  e.preventDefault()
  selectRole(CURSOR_ROLES[next]!.id)
}

// 各ロールの primary バイト列から Blob URL を派生し、ロール切替時に正しいプレビューを表示する。
// ロール毎にキャッシュして、リスト中のロール切替で URL を毎回作り直さない。
const roleBlobCache = new Map<string, { url: string; ref: Uint8Array }>()
function ensureRoleBlobUrl(roleId: string, bytes: Uint8Array): string {
  const cached = roleBlobCache.get(roleId)
  if (cached && cached.ref === bytes) return cached.url
  if (cached) URL.revokeObjectURL(cached.url)
  // Uint8Array → BlobPart: 一旦 ArrayBuffer のスライスにコピーして TS 型互換にする
  const buf = bytes.slice().buffer
  const url = URL.createObjectURL(new Blob([buf], { type: 'image/png' }))
  roleBlobCache.set(roleId, { url, ref: bytes })
  return url
}

/** 現在の役割に紐付いた表示用 PNG URL。assigned が無いロールは null (既定アイコン表示)。 */
const activePreviewUrl = computed<string | null>(() => {
  const a = assigned.value[activeRoleId.value]
  if (a?.primary) return ensureRoleBlobUrl(activeRoleId.value, a.primary)
  return null
})

/**
 * `<CursorPreview>` に渡す現在の asset 形。
 * ANI フレームがあれば 'ani'、静止 PNG があれば 'static'、どちらもなければ 'empty'。
 */
const activePreviewAsset = computed<CursorPreviewAsset>(() => {
  const frames = activeAniFrames.value
  if (frames) {
    const a = assigned.value[activeRoleId.value]
    return {
      kind: 'ani',
      framePngs: frames.framePngs,
      sequence: frames.sequence,
      durations: frames.perStepDurationsMs,
      nativeSize: a?.primarySize ?? activeSize.value,
    }
  }
  const url = activePreviewUrl.value
  if (url) return { kind: 'static', url, alt: activeRole.value.jp }
  return { kind: 'empty' }
})

/**
 * 現在のロール + サイズで「表示・操作対象」のホットスポット (ratio)。
 * perSizeHotspot=ON かつ sized.hotspot=Some のとき sized 側を返す。
 */
const activeHotspot = computed<Hotspot>(() => {
  const a = assigned.value[activeRoleId.value]
  if (!a) return { x: 0, y: 0 }
  if (perSizeHotspot.value) {
    const sized = a.sized?.get(activeSize.value)
    if (sized?.hotspot) return sized.hotspot
  }
  return a.hotspot
})

/**
 * 現在の編集対象 (primary or sized override) に hotspot を書き込む。
 * perSizeHotspot=ON かつそのサイズに override が既に存在 (sized.hotspot=Some) なら sized 側に、
 * それ以外は primary に書く。editor 操作 (pointer / keyboard / model setter) 専用。
 * import 系 (applyImportedRaster / pickCursorFromPath) は primary 直接書込を維持する。
 */
function writeActiveHotspot(next: Hotspot) {
  const id = activeRoleId.value
  const a = assigned.value[id]
  if (!a) return
  const sized = a.sized?.get(activeSize.value)
  if (perSizeHotspot.value && sized?.hotspot) {
    const nextSizedMap = new Map(a.sized ?? new Map())
    nextSizedMap.set(activeSize.value, { ...sized, hotspot: next })
    setAsset(id, { ...a, sized: nextSizedMap })
  } else {
    setAsset(id, { ...a, hotspot: next })
  }
}

/**
 * activeHotspot の writable 版。pointer / keyboard ハンドラから setter 経由で更新する。
 * writeActiveHotspot 経由で perSizeHotspot=ON 時に sized へ書き込む。
 */
const activeHotspotModel = computed<Hotspot>({
  get: () => activeHotspot.value,
  set: (next) => {
    writeActiveHotspot(next)
  },
})

/**
 * 現在のアクティブサイズに sized.hotspot override が存在するか。
 * enableSizedOverride を押した後に true になる。
 */
const sizedOverrideActive = computed(() => {
  const a = assigned.value[activeRoleId.value]
  return !!a?.sized?.get(activeSize.value)?.hotspot
})

/**
 * sized override の有効化ボタンを押せる条件 (アセット割り当て済み + perSizeHotspot=ON)。
 */
const canEditSizedOverride = computed(() => {
  const a = assigned.value[activeRoleId.value]
  return !!a && perSizeHotspot.value
})

/**
 * このサイズの sized.hotspot を primary hotspot からコピーして初期化する。
 * 以後 writeActiveHotspot が sized 側に書き込むようになる。
 */
function enableSizedOverride() {
  const id = activeRoleId.value
  const a = assigned.value[id]
  if (!a) return
  const nextSizedMap = new Map(a.sized ?? new Map())
  const existing = nextSizedMap.get(activeSize.value)
  nextSizedMap.set(activeSize.value, {
    png: existing?.png ?? a.primary,
    // 現在の primary hotspot をコピーして編集起点にする
    hotspot: { ...a.hotspot },
  })
  setAsset(id, { ...a, sized: nextSizedMap })
}

/** アクティブロールに .ani フレームデータが存在する場合にそれを返す。 */
const activeAniFrames = computed(() => {
  const id = activeRoleId.value
  if (!id) return null
  return assigned.value[id]?.aniFrames ?? null
})

/** アクティブロールの .ani 元ファイルパス (存在する場合のみ)。 */
const activeAniSourcePath = computed(() => {
  const id = activeRoleId.value
  if (!id) return null
  return assigned.value[id]?.aniSourcePath ?? null
})

onBeforeUnmount(() => {
  for (const { url } of roleBlobCache.values()) URL.revokeObjectURL(url)
  roleBlobCache.clear()
})

onUnmounted(() => {
  if (unlistenDrop) {
    unlistenDrop()
    unlistenDrop = null
  }
})

/**
 * 現在ロールのホットスポットを画像中央 (0.5, 0.5) に移動する。
 */
function centerHotspot() {
  writeActiveHotspot({ x: 0.5, y: 0.5 })
}

/**
 * 詳細設定で解像度 (`activeSize`) を切り替える。
 * ratio は size 非依存なので再投影不要。
 */
function selectSize(s: number) {
  activeSize.value = s
}

/**
 * 現在ロールの各サイズに対する実画像 Blob URL マップ。
 * SizeStrip の各タイルに表示する。
 *  - sized[size] があればそれを (size 別オーバーライド)
 *  - 無く且つ size === primarySize なら primary を
 *
 * Blob URL は role + size でキャッシュし、ロール切替で revoke する。
 */
const sizeBlobCache = new Map<string, { url: string; ref: Uint8Array }>()
function ensureSizeBlobUrl(roleId: string, size: number, bytes: Uint8Array): string {
  const key = `${roleId}:${size}`
  const cached = sizeBlobCache.get(key)
  if (cached && cached.ref === bytes) return cached.url
  if (cached) URL.revokeObjectURL(cached.url)
  const buf = bytes.slice().buffer
  const url = URL.createObjectURL(new Blob([buf], { type: 'image/png' }))
  sizeBlobCache.set(key, { url, ref: bytes })
  return url
}

const sizePreviewMap = computed<Record<number, string>>(() => {
  const out: Record<number, string> = {}
  const a = assigned.value[activeRoleId.value]
  if (!a) return out
  const roleId = activeRoleId.value
  for (const size of filledSizes.value) {
    const sized = a.sized?.get(size)
    if (sized?.png) {
      out[size] = ensureSizeBlobUrl(roleId, size, sized.png)
    } else if (size === a.primarySize && a.primary) {
      out[size] = ensureSizeBlobUrl(roleId, size, a.primary)
    }
  }
  return out
})

function isRequired(id: string): boolean {
  return id === 'Arrow'
}

// 起動時に keystore 状態を取得して「署名 & エクスポート」ボタンの表示判定に使う
onMounted(async () => {
  void refreshKeystore()
  void setupTauriDrop()
  // ライブラリの「Creator で編集」から `?editPath=...` で .cursorpack を渡された場合は
  // 自動ロードして editing ステージを開く。一時ファイルなので読み込み後に放置しても
  // OS が TEMP を整理してくれるので明示削除はしない。
  const route = useRoute()
  const editPath = (route.query.editPath as string | undefined) ?? null
  if (editPath) {
    try {
      const parsed = await bulkImport.parseCursorpack(editPath)
      bulkCursorpack.value = parsed
      bulkResolved.value = null
      bulkSourceLabel.value = t('creator.bulkSourceEditing')
      bulkModalOpen.value = true
      stage.value = 'editing'
      // `?editPath` 経由のみ元テーマ ID を保持。SaveDestinationModal が
      // 「上書き / 複製」セクションを出すトリガにも使う。
      sourceThemeId.value = parsed.metadata.id ?? null
      // `?editPath` 由来のテーマは「編集 → 再適用」が典型。デフォルトを Library+Apply に。
      saveModalDefault.value = 'libraryAndApply'
    } catch (err) {
      importMessage.value = t('creator.errEditLoadFailed', {
        detail: err instanceof Error ? err.message : String(err),
      })
      stage.value = 'editing'
    }
  }
})

// --- 画像インポート / エクスポート / 一括インポートのフロー制御 ---
// 詳細は composable に分離 (Phase 3c)。creator.vue は組み立てだけを担当する。

/** sanitized SVG 文字列 → 指定サイズの PNG バイト列 (Canvas 経由)。Canvas API 依存なのでここに残す。 */
async function rasterizeSvgToPng(svgString: string, size: number): Promise<Uint8Array> {
  const blob = new Blob([svgString], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  try {
    const img = new Image()
    img.decoding = 'async'
    img.src = url
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve()
      img.onerror = () => reject(new Error(t('creator.errSvgImageLoadFailed')))
    })
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error(t('creator.errCanvas2dContext'))
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, 0, 0, size, size)
    const pngBlob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob(
        (b) => (b ? resolve(b) : reject(new Error(t('creator.errToBlobFailed')))),
        'image/png',
      )
    })
    return new Uint8Array(await pngBlob.arrayBuffer())
  } finally {
    URL.revokeObjectURL(url)
  }
}

const { importBusy, importMessage, sanitizedRemovals, applyImportedRaster } = useCreatorImport({
  creatorAssets,
  activeRoleId,
  rasterizeSvgToPng,
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

// ----------------------------------------------------------------------------
// Tauri ウィンドウ Drag & Drop
//
// ブラウザの DragEvent は dataTransfer.files に絶対パスを含めないため、Library
// 画面と同じく Tauri v2 の onDragDropEvent で実パスを受け取って dispatchBulkPaths
// に流す。start ステージで受けた場合は NewThemeStartModal をスキップして editing
// へ直接遷移する (ユーザー選択: 両方で受け付ける)。
//
// dispatchBulkPaths は拡張子別に「.cursorpack 単独」と「bulk_resolve」に分岐する
// ので、ここでは拡張子ごとの分岐を再実装しない。サポート外の拡張子は
// dispatchBulkPaths 側で「マッチ 0 件」の空 preview として扱われる。
// ----------------------------------------------------------------------------
const showDrop = ref(false)
let unlistenDrop: (() => void) | null = null

const SUPPORTED_DROP_EXTS = ['png', 'svg', 'cur', 'ico', 'ani', 'cursorpack']

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
        const paths = (p.paths ?? []).filter((path: string) => {
          const ext = path.toLowerCase().split('.').pop() ?? ''
          return SUPPORTED_DROP_EXTS.includes(ext)
        })
        if (paths.length === 0) {
          importMessage.value = t('creator.errDropUnsupported')
          return
        }
        // start 画面で受けたときは NewThemeStartModal を閉じて editing に遷移。
        // dispatchBulkPaths 内で `.cursorpack` の場合は bulkModalOpen が立ち、
        // bulk_resolve 経路でも同様に preview が開くため stage 遷移は先んじて行う。
        if (stage.value === 'start') {
          newThemeModalOpen.value = false
          stage.value = 'editing'
        }
        void dispatchBulkPaths(paths)
      }
    })
  } catch (err) {
    console.warn('[Creator] Tauri drop API unavailable:', err)
  }
}

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
  saveModalDefault.value = 'file'
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
 * ヒーロー画面の「新規作成」CTA ハンドラ。
 * モーダルを開いてベース画像を選ばせる (デザイン要件) → Arrow ロールに割り当てて編集画面へ。
 */
function onStartNew() {
  newThemeModalOpen.value = true
}

/**
 * 「ファイル/パックから取り込む」CTA — bulkAuto を起動。
 * モーダルを閉じてから dispatch する。プレビューモーダルが開いたら editing へ遷移する。
 */
async function onNewThemePickFiles() {
  newThemeModalOpen.value = false
  await pickBulkAuto()
  if (bulkModalOpen.value) {
    stage.value = 'editing'
  }
}

/** 「フォルダから取り込む」CTA。 */
async function onNewThemePickFolder() {
  newThemeModalOpen.value = false
  await pickBulkFolder()
  if (bulkModalOpen.value) {
    stage.value = 'editing'
  }
}

/** モーダルから「画像なしで開始」を選んだ場合は従来通りの空エディタに遷移。 */
function onNewThemeStartEmpty() {
  newThemeModalOpen.value = false
  stage.value = 'editing'
}

function onNewThemeCancel() {
  newThemeModalOpen.value = false
}

/**
 * 「既存テーマを複製して編集」CTA ハンドラ。
 *
 * 1. ライブラリのテーマ一覧をロードしてピッカーモーダルを開く
 * 2. 選択されたテーマを `repackage_theme` で一時 `.cursorpack` 化
 * 3. 既存の bulk preview modal 経路 (parseCursorpack) に流して editing へ遷移
 *
 * 詳細モーダルの `editInCreator` と同じ IPC を使うので、ロール衝突解決や
 * メタデータ反映の挙動はそちらと統一される。
 */
async function onDuplicateExistingFromStart() {
  // 既存テーマの「複製」を起点にした新規作成セッション。`?editPath` で引き継いだ
  // ソース UUID は無効になるので、ピッカーを開く時点でクリアしておく
  // (SaveDestinationModal が誤って元テーマへの overwrite を提案するのを防ぐ)。
  sourceThemeId.value = null
  await refreshPickerThemes()
  themePickerSelected.value = null
  themePickerOpen.value = true
}

async function onThemePickerSelect(id: string | null) {
  themePickerOpen.value = false
  if (!id) return
  try {
    const { tempDir, sep } = await import('@tauri-apps/api/path')
    const dir = await tempDir()
    const tempPath = `${dir}${sep()}_easycursorswap_dup_${Date.now()}.cursorpack`
    await useThemes().repackageTheme(id, tempPath)
    await dispatchBulkPaths([tempPath])
    if (bulkModalOpen.value) {
      stage.value = 'editing'
    }
  } catch (err) {
    importMessage.value = t('creator.errDuplicateThemeFailed', {
      detail: err instanceof Error ? err.message : String(err),
    })
    stage.value = 'editing'
  }
}

function onThemePickerCancel() {
  themePickerOpen.value = false
}

async function onFileChange(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  importBusy.value = true
  importMessage.value = null
  sanitizedRemovals.value = []
  try {
    if (file.size > 10 * 1024 * 1024) {
      throw new Error(t('creator.errFileSizeOverMb'))
    }

    const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
    let pngBytes: Uint8Array | null = null
    if (ext === 'svg') {
      const text = await file.text()
      const { sanitized, removed } = sanitizeSvg(text)
      if (!sanitized)
        throw new Error(t('creator.errSvgUnparsable', { removed: removed.join(', ') }))
      sanitizedRemovals.value = removed
      // SVG → 256px PNG にラスタライズして Rust 側ビルダー用に保持
      pngBytes = await rasterizeSvgToPng(sanitized, 256)
      importMessage.value =
        removed.length > 0
          ? t('creator.notifySvgSanitized', { count: removed.length })
          : t('creator.notifySvgImported')
    } else if (ext === 'png') {
      // PNG は magic byte の弱検証のみ (89 50 4E 47)
      const fullBytes = new Uint8Array(await file.arrayBuffer())
      if (
        fullBytes.length < 8 ||
        fullBytes[0] !== 0x89 ||
        fullBytes[1] !== 0x50 ||
        fullBytes[2] !== 0x4e ||
        fullBytes[3] !== 0x47
      ) {
        throw new Error(t('creator.errPngBadHeader'))
      }
      pngBytes = fullBytes
      importMessage.value = t('creator.notifyPngImported')
    } else {
      throw new Error(t('creator.errUnsupportedExt', { ext }))
    }

    // 役割マップに登録 (assigned が真のソース。setAsset 経由で filledRoleSet
    // computed が追従するので filledRoles/filledSizesByRole の手動更新は不要。)
    // エクスポート時にも assigned 経由で使用。
    // PNG/SVG はホットスポット情報を持たないので、既存 hotspot を維持するか、
    // 新規ロールならロールに応じた初期値を適用する。ratio は size 非依存。
    if (pngBytes) {
      const existing = assigned.value[activeRoleId.value]
      const hotspot = existing?.hotspot ?? initialHotspotFor(activeRoleId.value, 256)
      setAsset(activeRoleId.value, {
        primary: pngBytes,
        primarySize: 256,
        hotspot,
        source: 'manual',
      })
    }
  } catch (err) {
    importMessage.value = t('creator.errImportFailed', {
      detail: err instanceof Error ? err.message : String(err),
    })
  } finally {
    importBusy.value = false
    // 同一ファイル再選択を許すため、change イベントの元 input をクリア。
    // input 要素は子コンポーネント (CreatorEditorCanvas) に移ったので、
    // e.target 経由で参照する (親の ref は持たない)。
    input.value = ''
  }
}
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
        @reset="resetCreator"
        @save="onToolbarSave"
        @bulk-auto="pickBulkAuto"
        @bulk-folder="pickBulkFolder"
      />

      <!-- タブバー -->
      <div class="tabs">
        <button
          v-for="t in tabs"
          :key="t.id"
          :class="['tab', { active: activeTab === t.id }]"
          @click="activeTab = t.id"
        >
          {{ t.label }}
          <span v-if="t.count" class="num">{{ t.count }}</span>
        </button>
      </div>

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
        :export-message="exportMessage"
        :export-progress="exportProgress"
        :export-busy="exportBusy"
        :failed-apply-theme-id="failedApplyThemeId"
        @dismiss-export-message="exportMessage = null"
        @cancel-export="cancelExport"
        @retry-apply="retryApply"
      />

      <!-- 2 カラムグリッド (assign タブのみ) -->
      <div v-if="activeTab === 'assign'" class="creator-grid">
        <CreatorRoleList
          :filled-count="filledCount"
          :active-role-id="activeRoleId"
          :status-of="statusOf"
          @select="selectRole"
          @keydown="onRoleListKeydown"
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
      @submit="
        (payload) => {
          saveModalOpen = false
          void executeSave(payload)
        }
      "
    />

    <!--
      既存テーマ複製ピッカー。「既存テーマを複製して編集」CTA から開く。
      選択時は `onThemePickerSelect` で repackage_theme → dispatchBulkPaths に流れる。
    -->
    <ThemePickerModal
      v-if="themePickerOpen"
      :model-value="themePickerSelected"
      :themes="pickerThemes"
      :title="t('creatorStart.duplicatePickerTitle')"
      :sub="t('creatorStart.duplicatePickerSub')"
      :show-clear="false"
      :show-footer-cancel="false"
      @update:model-value="onThemePickerSelect"
      @cancel="onThemePickerCancel"
    />

    <!--
      Tauri ウィンドウ DnD のフィードバック。Library 画面の LibraryDropOverlay と
      ほぼ同じだが、Creator では PNG/SVG/CUR/ICO/ANI/.cursorpack 全般を受け付ける
      ので文言を専用化している。
    -->
    <Transition name="fade">
      <div v-if="showDrop" class="creator-drop">
        <div class="creator-drop-inner">
          <UiIcon name="Import" :size="56" class="creator-drop-icon" />
          <h3>{{ t('creator.dropTitle') }}</h3>
          <p>{{ t('creator.dropSub') }}</p>
        </div>
      </div>
    </Transition>

    <!-- インポート結果メッセージ (Library の apply-error と同じく画面下部のポップアップ) -->
    <Transition name="fade">
      <div v-if="importMessage" class="import-banner" role="status">
        <UiIcon
          :name="importMessage.startsWith(t('creator.errImportPrefix')) ? 'Alert' : 'Check'"
          :size="13"
        />
        <span>{{ importMessage }}</span>
        <button
          class="btn ghost"
          style="margin-left: auto; height: 24px"
          @click="importMessage = null"
        >
          <UiIcon name="X" :size="11" />
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.creator-host {
  @apply relative flex h-full flex-col;
}

.draft-tag {
  @apply ml-1.5 font-mono text-[10.5px] text-fg-mute;
}

.pane-head {
  @apply mb-2.5 flex items-center justify-between;
}
.pane-head h6 {
  @apply m-0 font-mono text-[10px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}

.color-chips {
  @apply flex gap-1;
}
.cc {
  @apply size-[18px] rounded border border-line;
}

.validation-body {
  @apply gap-2 font-mono text-[11.5px] text-fg-dim;
}
.vrow {
  @apply flex justify-between;
}
.vrow .ok {
  @apply text-accent;
}
.vrow .warn {
  @apply text-amber;
}
.vrow .dim {
  @apply text-fg-dim;
}

/* インポート結果ポップアップ。Library の .apply-error と同じ位置/挙動で、
 * 色だけ accent (緑) を維持して「成功+失敗」両用で使う。 */
.import-banner {
  @apply fixed bottom-12 left-1/2 z-[90] flex min-w-[320px] max-w-[80%] -translate-x-1/2 items-center gap-2.5 rounded-[8px] border border-accent-line px-3.5 py-2.5 text-[12.5px] text-fg-dim backdrop-blur-[12px];
  background: rgba(124, 242, 212, 0.1);
  box-shadow: var(--shadow-2);
}

.export-progress {
  @apply mx-[18px] mb-2 mt-0 rounded-[8px] border border-accent-line px-3 py-2 text-[12px] text-fg-dim;
  background: rgba(124, 242, 212, 0.04);
}
.export-progress-row {
  @apply mb-1.5 flex items-center gap-2;
}
.export-progress-label {
  font-variant-numeric: tabular-nums;
}
.export-progress-bar {
  @apply h-1 overflow-hidden rounded-sm;
  background: rgba(255, 255, 255, 0.06);
}
.export-progress-fill {
  @apply h-full bg-accent;
  transition: width 120ms ease-out;
}

.metadata-pane {
  @apply flex-1 overflow-y-auto px-7 pb-8 pt-6;
  background: radial-gradient(800px 600px at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%);
}
.metadata-grid {
  @apply mx-auto flex max-w-[760px] flex-col gap-[18px];
}

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

/* Tauri DnD オーバーレイ。LibraryDropOverlay と同じ表現を Creator 用に inline 化。
 * Creator 配下のすべてのステージ (start / editing) を覆うので creator-host
 * (relative) の中で z-50 に積む。 */
.creator-drop {
  @apply absolute inset-0 z-50 grid place-items-center;
  background: rgba(10, 11, 15, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
.creator-drop-inner {
  @apply w-[480px] rounded-2xl p-10 text-center;
  border: 1.5px dashed var(--accent-line);
  background: rgba(124, 242, 212, 0.04);
}
.creator-drop-inner h3 {
  @apply m-0 font-display text-[18px] font-semibold tracking-[-0.01em];
  margin: 12px 0 6px;
}
.creator-drop-inner p {
  @apply m-0 text-[13px] text-fg-dim;
}
.creator-drop-icon {
  @apply text-accent;
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
