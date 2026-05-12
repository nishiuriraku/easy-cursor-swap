<script setup lang="ts">
/**
 * クリエイターモード (Phase 5-5)
 *
 * design/creator.jsx を Vue 化したもの。2 カラム構成 (Assign タブ):
 *  - 左:  17 役割リスト (filled/partial/empty ドット付き)
 *  - 中央: ビッグプレビュー + 6 サイズストリップ + リサンプル切替
 *  ※ ホットスポット / 影フラグはメタデータタブに集約し、右ペインは廃止。
 *
 * NOTE: 実際の画像アップロード / .cur ビルド / 署名生成は今回はスタブ。
 *       UI 構造とインタラクションのみ実装し、IPC 配線は後続タスクに委ねる。
 */
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'
import { sanitizeSvg } from '~/composables/sanitizeSvg'
import { invokeTauri } from '~/composables/useTauri'
import { useKeystore } from '~/composables/useKeystore'
import { useI18n } from '~/composables/useI18n'
import { useCreatorAssets, scaleHotspot } from '~/composables/useCreatorAssets'
import { initialHotspotFor } from '~/composables/useHotspotDefaults'
import {
  useBulkImport,
  type ResolvedAsset,
  type ParsedCursorpack,
} from '~/composables/useBulkImport'
import AniThumb from '~/components/creator/AniThumb.vue'
import BulkImportButton from '~/components/creator/BulkImportButton.vue'
import BulkImportPreviewModal, {
  type ApplyPayload,
} from '~/components/creator/BulkImportPreviewModal.vue'
import NewThemeStartModal from '~/components/creator/NewThemeStartModal.vue'
import SaveDestinationModal, {
  type SaveSubmitPayload,
} from '~/components/creator/SaveDestinationModal.vue'
import ThemePickerModal from '~/components/library/ThemePickerModal.vue'
import { useThemes } from '~/composables/useThemes'

const { t } = useI18n()

const { info: keystoreInfo, refresh: refreshKeystore } = useKeystore()
const hasKeystoreSigning = computed(() => keystoreInfo.value.has_keypair)

type RoleStatus = 'filled' | 'partial' | 'empty'
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
  description: 'Windows 用カーソルテーマを 17 役割 × 6 解像度で作成しエクスポートする',
  ogImage: '/icon.png',
})

/* === useSeoMeta は title 用途 (上で設定済) ============================================
 * 以降は通常のページロジック。`stage` ref に応じて `<template>` 内で
 * CreatorStartScreen と編集 UI を切替える。useSeoMeta 設定はファイル冒頭で完結している。
 * ====================================================================================== */

// --- ダミーステート (実装は将来の IPC 連携で置換) ---
const filledRoles = reactive(
  new Set<string>([
    'Arrow',
    'Help',
    'Wait',
    'IBeam',
    'Hand',
    'No',
    'Crosshair',
    'SizeNS',
    'SizeWE',
    'SizeAll',
    'NWPen',
  ]),
)
const partialRoles = new Set<string>(['AppStarting', 'SizeNWSE'])

const activeTab = ref<TabId>('assign')
const activeRoleId = ref<string>('Arrow')
const activeSize = ref<number>(64)
const filledSizesByRole = ref<Record<string, number[]>>({
  Arrow: [32, 48, 64, 128],
})
const resample = ref<ResampleMode>('lanczos')
const hotspotX = ref(4)
const hotspotY = ref(4)
const perSizeHotspot = ref(true)
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
const shadowEnabled = ref(false)

/**
 * 役割ごとのインポート済みアセットを `useCreatorAssets` 経由で集約管理する。
 * 単一インポート / 一括インポート / `.cursorpack` 取り込みなどの経路はすべてここに合流する。
 */
const creatorAssets = useCreatorAssets()
const { assigned, setAsset, assignedRoleCount, arrowAssigned, toExportPayload } = creatorAssets

// メタデータタブの入力
const metaName = ref<string>('Untitled Theme')
const metaNameEn = ref<string>('')
const metaAuthor = ref<string>('')
const metaVersion = ref<string>('1.0.0')
const metaDescription = ref<string>('')

// --- 一括インポート ---
const bulkImport = useBulkImport()

// プレビューモーダル制御
const bulkModalOpen = ref(false)
const bulkResolved = ref<ResolvedAsset[] | null>(null)
const bulkCursorpack = ref<ParsedCursorpack | null>(null)
const bulkSourceLabel = ref('')

// 既存テーマ複製ピッカー
const themePickerOpen = ref(false)
const themePickerSelected = ref<string | null>(null)
const { themes: pickerThemes, refresh: refreshPickerThemes } = useThemes()

const existingRolesSet = computed(() => new Set(Object.keys(assigned.value)))

// --- 計算プロパティ ---
const activeRole = computed<CursorRoleDef>(
  () => CURSOR_ROLES.find((r) => r.id === activeRoleId.value) ?? CURSOR_ROLES[0]!,
)

function statusOf(id: string): RoleStatus {
  if (filledRoles.has(id)) return 'filled'
  if (partialRoles.has(id)) return 'partial'
  return 'empty'
}

const filledCount = computed(() => filledRoles.size)
const tabs = computed<Array<{ id: TabId; label: string; count?: string }>>(() => [
  { id: 'assign', label: t('creator.tabAssign'), count: `${filledCount.value}/17` },
  { id: 'metadata', label: t('creator.tabMetadata') },
])
const filledSizes = computed(() => filledSizesByRole.value[activeRoleId.value] ?? [])
const sizesCovered = computed(() => filledSizes.value.length)

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

/** 現在の役割に紐付いた表示用 PNG URL。優先順: そのロールの assigned → 直近インポート → null */
const activePreviewUrl = computed<string | null>(() => {
  const a = assigned.value[activeRoleId.value]
  if (a?.primary) return ensureRoleBlobUrl(activeRoleId.value, a.primary)
  return importedPreviewUrl.value
})

/** ホットスポットの基準サイズ。アセットがあればその primarySize、なければ現在の activeSize。 */
const hotspotReferenceSize = computed(
  () => assigned.value[activeRoleId.value]?.primarySize ?? activeSize.value,
)

/**
 * `.bigpreview` 内で `<img>` が占める最大寸法 (CSS の `max-width`/`max-height` と一致)。
 * ホットスポットドットの位置とドラッグ範囲をこの「画像領域」に揃えるために使う。
 * `global.css` の `.bigpreview img` スタイルを変えたら同期すること。
 */
const IMAGE_DISPLAY_PCT = 90

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

/** ロールが切り替わったら、そのロールの保存済みホットスポットを反映する。 */
watch(activeRoleId, (id) => {
  const a = assigned.value[id]
  if (a) {
    hotspotX.value = a.hotspot.x
    hotspotY.value = a.hotspot.y
  }
})

onBeforeUnmount(() => {
  for (const { url } of roleBlobCache.values()) URL.revokeObjectURL(url)
  roleBlobCache.clear()
})

// --- ホットスポットのドラッグ操作 ---
const bigpreviewEl = ref<HTMLElement | null>(null)
const hotspotDragging = ref(false)

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value))
}

function updateHotspotFromEvent(e: PointerEvent) {
  const el = bigpreviewEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  // 画像は中央配置で 90% サイズなので、コンテナ左右に各 5% (px = rect.width * 0.05)
  // の余白がある。ホットスポット 0 / refSize の境界を画像領域にそろえるため、
  // ポインタ位置をその余白を引いた値で割合化し、0-1 にクランプする。
  // ドット表示 (hotspotStyle) もこの座標系に合わせて 90% スケールしている。
  const margin = (100 - IMAGE_DISPLAY_PCT) / 2 / 100 // 0.05
  const innerLeft = rect.left + rect.width * margin
  const innerTop = rect.top + rect.height * margin
  const innerWidth = rect.width * (IMAGE_DISPLAY_PCT / 100)
  const innerHeight = rect.height * (IMAGE_DISPLAY_PCT / 100)
  const ratioX = clamp((e.clientX - innerLeft) / innerWidth, 0, 1)
  const ratioY = clamp((e.clientY - innerTop) / innerHeight, 0, 1)
  const ref = hotspotReferenceSize.value
  hotspotX.value = Math.round(ratioX * ref)
  hotspotY.value = Math.round(ratioY * ref)
}

function onHotspotPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  hotspotDragging.value = true
  const el = e.currentTarget as HTMLElement
  el.setPointerCapture?.(e.pointerId)
  el.focus({ preventScroll: true })
  updateHotspotFromEvent(e)
}

function onHotspotPointerMove(e: PointerEvent) {
  if (!hotspotDragging.value) return
  updateHotspotFromEvent(e)
}

function onHotspotPointerUp(e: PointerEvent) {
  if (!hotspotDragging.value) return
  hotspotDragging.value = false
  ;(e.currentTarget as Element).releasePointerCapture?.(e.pointerId)
  // 保存済みアセットがあればホットスポット値を反映する
  const a = assigned.value[activeRoleId.value]
  if (a) {
    setAsset(activeRoleId.value, {
      ...a,
      hotspot: { x: hotspotX.value, y: hotspotY.value },
    })
  }
}

/**
 * ビッグプレビューに focus がある状態で矢印キー / Home / End / PgUp / PgDn を
 * ハンドリングしてホットスポットを 1 px 単位で移動する。Shift 同時押しで 10 px。
 *
 * テキスト入力中の矢印操作と衝突しないよう、event.target が input/textarea のときは
 * 何もしない。bigpreviewEl 自身が tabindex=0 でフォーカス可能なので、
 * クリックやドラッグ後にこのハンドラが優先される。
 */
function onHotspotKeydown(e: KeyboardEvent) {
  const tag = (e.target as HTMLElement).tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return

  const ref = hotspotReferenceSize.value
  const step = e.shiftKey ? 10 : 1
  let nx = hotspotX.value
  let ny = hotspotY.value

  switch (e.key) {
    case 'ArrowLeft':
      nx -= step
      break
    case 'ArrowRight':
      nx += step
      break
    case 'ArrowUp':
      ny -= step
      break
    case 'ArrowDown':
      ny += step
      break
    case 'Home':
      nx = 0
      break
    case 'End':
      nx = ref
      break
    case 'PageUp':
      ny = 0
      break
    case 'PageDown':
      ny = ref
      break
    default:
      return
  }
  e.preventDefault()
  hotspotX.value = clamp(nx, 0, ref)
  hotspotY.value = clamp(ny, 0, ref)
  // 既存アセットがあればホットスポット値を永続化する
  const a = assigned.value[activeRoleId.value]
  if (a) {
    setAsset(activeRoleId.value, {
      ...a,
      hotspot: { x: hotspotX.value, y: hotspotY.value },
    })
  }
}

/**
 * 現在ロールのホットスポットを画像中央 (primarySize / 2) に移動する。
 * 既存アセットがあれば永続化、未割当なら UI 表示のみ更新。
 */
function centerHotspot() {
  const ref = hotspotReferenceSize.value
  const center = Math.round(ref / 2)
  hotspotX.value = center
  hotspotY.value = center
  const a = assigned.value[activeRoleId.value]
  if (a) {
    setAsset(activeRoleId.value, { ...a, hotspot: { x: center, y: center } })
  }
}

/**
 * 詳細設定で解像度 (`activeSize`) を切り替えたとき、ホットスポットを
 * 「画面上の同じ比率位置」に保ったまま追従させる。
 *
 * - アセット未割当のロールでは `hotspotReferenceSize = activeSize` なので
 *   解像度を変えると参照系が変わる。`(x/prevRef, y/prevRef)` の比率を
 *   `nextRef` ピクセル系に再投影することで dot が動かないようにする。
 * - アセット割当済みでは `hotspotReferenceSize = primarySize` 固定なので
 *   参照系は不変、再スケールは発生しない (no-op)。
 */
function selectSize(s: number) {
  const prevRef = hotspotReferenceSize.value
  activeSize.value = s
  const nextRef = hotspotReferenceSize.value
  if (prevRef !== nextRef && prevRef > 0) {
    const ratioX = hotspotX.value / prevRef
    const ratioY = hotspotY.value / prevRef
    hotspotX.value = clamp(Math.round(ratioX * nextRef), 0, nextRef)
    hotspotY.value = clamp(Math.round(ratioY * nextRef), 0, nextRef)
  }
}

function isRequired(id: string): boolean {
  return id === 'Arrow'
}

// 起動時に keystore 状態を取得して「署名 & エクスポート」ボタンの表示判定に使う
import { onMounted } from 'vue'
onMounted(async () => {
  void refreshKeystore()
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
      bulkSourceLabel.value = '📦 編集中'
      bulkModalOpen.value = true
      stage.value = 'editing'
      // `?editPath` 経由のみ元テーマ ID を保持。SaveDestinationModal が
      // 「上書き / 複製」セクションを出すトリガにも使う。
      sourceThemeId.value = parsed.metadata.id ?? null
      // `?editPath` 由来のテーマは「編集 → 再適用」が典型。デフォルトを Library+Apply に。
      saveModalDefault.value = 'libraryAndApply'
    } catch (err) {
      importMessage.value = `編集データの読込に失敗: ${err instanceof Error ? err.message : String(err)}`
      stage.value = 'editing'
    }
  }
})

// --- 画像インポート ---
const importBusy = ref(false)
const importMessage = ref<string | null>(null)
/** 直近インポートで除去された SVG 要素/属性 (デバッグ表示) */
const sanitizedRemovals = ref<string[]>([])
/** プレビュー用 URL (Object URL or data URL) */
const importedPreviewUrl = ref<string | null>(null)

const fileInput = ref<HTMLInputElement | null>(null)

/** Tauri plugin-fs でファイルを読み込み、PNG / SVG として現在のロールに反映する。 */
async function pickRasterFromPath(path: string, ext: string) {
  const { readFile } = await import('@tauri-apps/plugin-fs')
  const data = await readFile(path)
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data)
  if (ext === 'svg') {
    const text = new TextDecoder().decode(bytes)
    const { sanitized, removed } = sanitizeSvg(text)
    if (!sanitized) throw new Error('SVG が解析できません: ' + removed.join(', '))
    sanitizedRemovals.value = removed
    const png = await rasterizeSvgToPng(sanitized, 256)
    applyImportedRaster(png, 256)
    importMessage.value =
      removed.length > 0
        ? `SVG を sanitize しました (除去: ${removed.length} 件)`
        : `SVG をインポートしました`
  } else {
    if (
      bytes.length < 8 ||
      bytes[0] !== 0x89 ||
      bytes[1] !== 0x50 ||
      bytes[2] !== 0x4e ||
      bytes[3] !== 0x47
    ) {
      throw new Error('PNG ヘッダーが不正です')
    }
    applyImportedRaster(bytes, 256)
    importMessage.value = 'PNG をインポートしました'
  }
}

/**
 * 取り込んだ raster バイト列を「現在の activeRole」に反映する共通ロジック。
 * `pickRasterFromPath` と `onFileChange` (HTML input フォールバック) で共有。
 */
function applyImportedRaster(png: Uint8Array, primarySize: number) {
  if (importedPreviewUrl.value && importedPreviewUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(importedPreviewUrl.value)
  }
  importedPreviewUrl.value = URL.createObjectURL(
    new Blob([png.slice().buffer], { type: 'image/png' }),
  )
  filledRoles.add(activeRoleId.value)
  const map = filledSizesByRole.value[activeRoleId.value] ?? []
  if (!map.includes(activeSize.value)) {
    filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
  }
  // 新規ロールかつホットスポット未編集 (デフォ 4,4) の場合のみ中央既定値を適用。
  // PNG/SVG はホットスポット情報を持たないので、ロールに応じて中央 or 左上を選ぶ。
  const isNewRole = !creatorAssets.hasAsset(activeRoleId.value)
  const isDefault = hotspotX.value === 4 && hotspotY.value === 4
  const fromSize = hotspotReferenceSize.value
  const finalHotspot =
    isNewRole && isDefault
      ? initialHotspotFor(activeRoleId.value, primarySize)
      : scaleHotspot({ x: hotspotX.value, y: hotspotY.value }, fromSize, primarySize)
  setAsset(activeRoleId.value, {
    primary: png,
    primarySize,
    hotspot: finalHotspot,
    source: 'manual',
  })
  hotspotX.value = finalHotspot.x
  hotspotY.value = finalHotspot.y
}

/** `.cur` / `.ico` ファイルをパスから直接 Rust 側でパースする (ダイアログを開かない版)。 */
async function pickCursorFromPath(picked: string) {
  const result = await invokeTauri<{
    isCur: boolean
    width: number
    height: number
    hotspotX: number
    hotspotY: number
    pngBytes: number[]
    availableSizes: number[]
  }>('import_cursor_file', { path: picked })
  if (!result) throw new Error('IPC 結果が空でした')

  const png = new Uint8Array(result.pngBytes)
  if (importedPreviewUrl.value && importedPreviewUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(importedPreviewUrl.value)
  }
  importedPreviewUrl.value = URL.createObjectURL(
    new Blob([png.slice().buffer], { type: 'image/png' }),
  )

  filledRoles.add(activeRoleId.value)
  const map = filledSizesByRole.value[activeRoleId.value] ?? []
  if (!map.includes(activeSize.value)) {
    filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
  }
  // .cur/.ico に埋め込まれた hotspot が (0, 0) かつロール新規なら中央既定値を適用。
  // それ以外は元ファイルのホットスポット情報を尊重する。
  const isNewRole = !creatorAssets.hasAsset(activeRoleId.value)
  const noEmbeddedHotspot = result.hotspotX === 0 && result.hotspotY === 0
  const finalHotspot =
    isNewRole && noEmbeddedHotspot
      ? initialHotspotFor(activeRoleId.value, result.width)
      : { x: result.hotspotX, y: result.hotspotY }
  hotspotX.value = finalHotspot.x
  hotspotY.value = finalHotspot.y
  setAsset(activeRoleId.value, {
    primary: png,
    primarySize: result.width,
    hotspot: finalHotspot,
    source: 'manual',
  })
  const sizeList = result.availableSizes.length > 0 ? result.availableSizes.join('/') : '?'
  const kind = result.isCur ? '.cur' : '.ico'
  importMessage.value = `${kind} を取り込みました (${result.width}x${result.height}, 含解像度: ${sizeList})`
}

// --- パッケージエクスポート ---
const exportBusy = ref(false)
const exportMessage = ref<string | null>(null)
/** Library 保存成功 + apply 失敗時の theme_id。retry ボタンの活性化と invokeTauri('apply_theme', { themeId }) 呼出に使う。 */
const failedApplyThemeId = ref<string | null>(null)

interface ExportResult {
  theme_id: string
  size_bytes: number
  signed: boolean
  key_id: string | null
  applied: boolean
  apply_error: string | null
}

/** ストリームエクスポート時の進捗状態 */
interface BuildProgress {
  buildId: string
  stage: 'role' | 'package' | 'sign' | 'done' | 'cancelled' | 'error'
  current: number
  total: number
  message: string | null
}
const exportProgress = ref<BuildProgress | null>(null)
const currentBuildId = ref<string | null>(null)

/** 進行中のエクスポートを中止する。Rust 側は次のチェックポイントで終了する。 */
async function cancelExport() {
  if (!currentBuildId.value) return
  try {
    await invokeTauri('cancel_build', { buildId: currentBuildId.value })
  } catch {
    // ignore
  }
}

/**
 * SaveDestinationModal の submit を受けて Rust 側 export_cursorpack_streamed を呼ぶ。
 * destination ごとにトーストメッセージを切り替え、apply 失敗時は warning + retry。
 */
async function executeSave(payload: SaveSubmitPayload) {
  if (assignedRoleCount.value === 0) {
    exportMessage.value = '少なくとも 1 役割に画像を割り当ててください'
    return
  }
  if (!arrowAssigned.value) {
    exportMessage.value = 'Arrow ロールは必須です'
    return
  }
  exportBusy.value = true
  exportMessage.value = null
  exportProgress.value = null

  let unlisten: (() => void) | null = null
  try {
    const buildId = `build-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
    currentBuildId.value = buildId

    try {
      const { listen } = await import('@tauri-apps/api/event')
      unlisten = await listen<BuildProgress>('build-progress', (e) => {
        if (e.payload.buildId === buildId) exportProgress.value = e.payload
      })
    } catch {
      // Web 開発時は購読をスキップ
    }

    const roles = toExportPayload(resample.value)

    const destination =
      payload.destination === 'file'
        ? { kind: 'file', path: payload.filePath! }
        : { kind: 'library', applyAfter: payload.destination === 'libraryAndApply' }

    const result = await invokeTauri<ExportResult>('export_cursorpack_streamed', {
      req: {
        buildId,
        nameJa: payload.effectiveName,
        nameEn: metaNameEn.value || null,
        author: metaAuthor.value || null,
        version: metaVersion.value,
        requiresOsShadow: shadowEnabled.value,
        roles,
        destination,
        existingThemeId:
          payload.overwriteExisting && sourceThemeId.value ? sourceThemeId.value : null,
        sign: payload.sign,
      },
    })

    if (!result) throw new Error('エクスポート結果が空でした')

    if (result.apply_error) {
      exportMessage.value = t('saveModal.toastAppliedFailed').replace('{error}', result.apply_error)
      failedApplyThemeId.value = result.theme_id
    } else if (result.applied) {
      exportMessage.value = t('saveModal.toastSavedAndApplied')
      failedApplyThemeId.value = null
    } else if (payload.destination === 'file') {
      exportMessage.value = t('saveModal.toastSavedFile').replace('{path}', payload.filePath!)
      failedApplyThemeId.value = null
    } else {
      exportMessage.value = t('saveModal.toastSavedLibrary')
      failedApplyThemeId.value = null
    }
  } catch (err) {
    exportMessage.value = `エクスポート失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    if (unlisten) unlisten()
    currentBuildId.value = null
    exportBusy.value = false
    setTimeout(() => {
      if (!exportBusy.value) exportProgress.value = null
    }, 3000)
  }
}

/**
 * apply 失敗後の再試行 CTA。バナーから呼ばれる。
 * 失敗 ID をクリアして apply_theme を再度叩く。
 */
async function retryApply() {
  if (!failedApplyThemeId.value) return
  const themeId = failedApplyThemeId.value
  failedApplyThemeId.value = null
  try {
    await invokeTauri<void>('apply_theme', { themeId })
    exportMessage.value = t('saveModal.toastSavedAndApplied')
  } catch (err) {
    exportMessage.value = `再試行失敗: ${err instanceof Error ? err.message : String(err)}`
    failedApplyThemeId.value = themeId // 再再試行のため復元
  }
}

function onToolbarSave() {
  saveModalOpen.value = true
}

/** sanitized SVG 文字列 → 指定サイズの PNG バイト列 (Canvas 経由) */
async function rasterizeSvgToPng(svgString: string, size: number): Promise<Uint8Array> {
  const blob = new Blob([svgString], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  try {
    const img = new Image()
    img.decoding = 'async'
    img.src = url
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve()
      img.onerror = () => reject(new Error('SVG イメージの読み込みに失敗'))
    })
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error('Canvas 2D コンテキスト取得失敗')
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, 0, 0, size, size)
    const pngBlob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob 失敗'))), 'image/png')
    })
    return new Uint8Array(await pngBlob.arrayBuffer())
  } finally {
    URL.revokeObjectURL(url)
  }
}

// --- 一括インポート ハンドラ ---

/**
 * 統合エントリ。png/svg/cur/ico/ani/.cursorpack をまとめて選べるダイアログを開き、
 * 拡張子で内部分岐:
 *   - 単独 `.cursorpack`     → cursorpack 解析経路 (parseCursorpack)
 *   - 通常ファイル (複数可)   → bulk_resolve_assets 経路
 *   - 混在 / 複数 cursorpack  → 警告メッセージを表示し非サポート部分を除外
 *
 * フォルダ取込はネイティブダイアログ仕様で別経路 (`pickBulkFolder`) になる。
 */
async function pickBulkAuto() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const picked = await open({
    multiple: true,
    filters: [
      {
        name: 'Cursor assets / pack',
        extensions: ['png', 'svg', 'cur', 'ico', 'ani', 'cursorpack'],
      },
    ],
  })
  if (!picked) return
  const paths = Array.isArray(picked) ? picked : [picked]
  await dispatchBulkPaths(paths)
}

/** フォルダから取込 (chevron サブメニュー / 新規作成モーダル経由)。 */
async function pickBulkFolder() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const picked = await open({ directory: true })
  if (!picked || typeof picked !== 'string') return
  await runBulkResolve([picked], false, `📁 ${picked}`)
}

/**
 * 拡張子分岐の本体。`pickBulkAuto` から、または将来のドラッグ&ドロップから呼ばれる想定。
 *
 * 設計判断:
 *  - 単一ファイル (1 件) で `.cursorpack` 以外 (= png/svg/cur/ico) の場合は、
 *    bulk preview を経由せず **現在編集中のロールに直接代入** する fast-path に流す。
 *    これは旧「画像 / カーソルを取込」ボタンの挙動を維持するためで、エディタ内で
 *    特定ロールを選んで素早く差し替えるワークフローを壊さない
 *  - 単一 `.ani` は static fast-path には乗せず、bulk preview 経路に通す
 *    (アニメ再生 + ホットスポット調整 UI が必要なため)
 *  - `.cursorpack` は他のファイルと一緒に取り込む意味的整合性が無い (パッケージ単位の取込なので)
 *    ため、混在時は通常ファイルのみ取り込み、cursorpack 部分は無視する
 *  - 複数 `.cursorpack` の同時取込もサポートしない (ロール衝突解決が複雑になるため)
 */
async function dispatchBulkPaths(paths: string[]) {
  // Fast-path: 1 件かつ非 cursorpack なら現在ロールに直接代入
  if (paths.length === 1) {
    const p = paths[0]!
    const ext = p.split('.').pop()?.toLowerCase() ?? ''
    if (ext === 'cur' || ext === 'ico') {
      importBusy.value = true
      importMessage.value = null
      try {
        await pickCursorFromPath(p)
      } catch (err) {
        importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
      } finally {
        importBusy.value = false
      }
      return
    }
    if (ext === 'png' || ext === 'svg') {
      importBusy.value = true
      importMessage.value = null
      sanitizedRemovals.value = []
      try {
        await pickRasterFromPath(p, ext)
      } catch (err) {
        importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
      } finally {
        importBusy.value = false
      }
      return
    }
    // .cursorpack は下の bulk preview ロジックに fallthrough
  }

  const packs = paths.filter((p) => p.toLowerCase().endsWith('.cursorpack'))
  const others = paths.filter((p) => !p.toLowerCase().endsWith('.cursorpack'))

  if (packs.length === 1 && others.length === 0) {
    try {
      const parsed = await bulkImport.parseCursorpack(packs[0]!)
      bulkCursorpack.value = parsed
      bulkResolved.value = null
      bulkSourceLabel.value = `📦 ${packs[0]!.split(/[\\/]/).pop()}`
      bulkModalOpen.value = true
      // `.cursorpack` を取り込んだ瞬間、編集対象のソースが入れ替わる。
      // `?editPath` で引き継いだ UUID は無効になるのでクリア (SaveDestinationModal の
      // 誤 overwrite 提案を防ぐ)。
      sourceThemeId.value = null
    } catch (err) {
      importMessage.value = `cursorpack 取り込み失敗: ${err instanceof Error ? err.message : String(err)}`
    }
    return
  }

  if (packs.length >= 2) {
    importMessage.value = '.cursorpack は 1 つだけ選択してください'
    return
  }

  if (packs.length === 1 && others.length > 0) {
    importMessage.value =
      '.cursorpack は他のファイルと同時に取り込めません (.cursorpack 以外を取り込みました)'
  }

  if (others.length === 0) return
  await runBulkResolve(others, false, `${others.length} 個のファイル`)
}

async function runBulkResolve(paths: string[], recursive: boolean, label: string) {
  try {
    const r = await bulkImport.resolveAssets(paths, recursive)
    if (r.assets.length === 0) {
      importMessage.value = '対応ファイルが見つかりません'
      return
    }
    bulkResolved.value = r.assets
    bulkCursorpack.value = null
    bulkSourceLabel.value = label
    bulkModalOpen.value = true
    if (r.failures.length > 0) {
      // 後で本格的な警告 UI に置換可能。現状はトースト風メッセージ。
      importMessage.value = `${r.failures.length} 件のファイルをスキップしました`
    }
  } catch (err) {
    importMessage.value = `一括インポート失敗: ${err instanceof Error ? err.message : String(err)}`
  }
}

function applyBulkImport(payload: ApplyPayload) {
  for (const { roleId, asset } of payload.roleAssets) {
    setAsset(roleId, asset)
    // filledSizesByRole も更新 (UI バッジ用)
    const sizes = asset.sized ? Array.from(asset.sized.keys()) : [asset.primarySize]
    filledSizesByRole.value = { ...filledSizesByRole.value, [roleId]: sizes }
    filledRoles.add(roleId)
  }

  // メタデータ反映 (.cursorpack のみ)
  if (payload.metadata && payload.metadataChoice !== 'keep') {
    metaName.value = payload.metadata.nameJa ?? metaName.value
    if (payload.metadataChoice === 'overwrite') {
      metaNameEn.value = payload.metadata.nameEn ?? metaNameEn.value
      metaAuthor.value = payload.metadata.author ?? metaAuthor.value
      metaVersion.value = payload.metadata.version ?? metaVersion.value
      metaDescription.value = payload.metadata.description ?? metaDescription.value
    }
  }

  bulkModalOpen.value = false
  importMessage.value = `${payload.roleAssets.length} 件のロールを適用しました`

  // ✓ 「すぐシステムに反映」がチェックされていれば SaveDestinationModal を
  // Library+Apply 既定で開く。ユーザーは [保存] を押すだけで適用まで進める。
  if (payload.applyImmediately) {
    saveModalDefault.value = 'libraryAndApply'
    saveModalOpen.value = true
  }
}

function cancelBulkImport() {
  bulkModalOpen.value = false
  bulkResolved.value = null
  bulkCursorpack.value = null
}

/**
 * Creator の編集状態を完全にリセットして初期画面に戻す。
 *
 * アセット・メタデータ・インポートメッセージ・進捗バナーを全てクリアして
 * 「Clear」ボタンを押した瞬間に Cmd+N と同等の状態に戻す。プレビュー Blob URL も
 * 解放してメモリリークを防ぐ。
 */
function resetCreator() {
  for (const role of Object.keys(assigned.value)) {
    creatorAssets.removeAsset(role)
  }
  filledRoles.clear()
  filledSizesByRole.value = {}
  activeRoleId.value = 'Arrow'
  sourceThemeId.value = null
  saveModalDefault.value = 'file'
  saveModalOpen.value = false
  activeSize.value = 64
  hotspotX.value = 4
  hotspotY.value = 4
  metaName.value = 'Untitled Theme'
  metaNameEn.value = ''
  metaAuthor.value = ''
  metaVersion.value = '1.0.0'
  metaDescription.value = ''
  shadowEnabled.value = false
  if (importedPreviewUrl.value && importedPreviewUrl.value.startsWith('blob:')) {
    URL.revokeObjectURL(importedPreviewUrl.value)
  }
  importedPreviewUrl.value = null
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
    await invokeTauri<number>('repackage_theme', { themeId: id, outputPath: tempPath })
    await dispatchBulkPaths([tempPath])
    if (bulkModalOpen.value) {
      stage.value = 'editing'
    }
  } catch (err) {
    importMessage.value = `既存テーマの複製に失敗: ${err instanceof Error ? err.message : String(err)}`
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
      throw new Error('ファイルサイズが 10 MB を超えています')
    }

    const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
    let pngBytes: Uint8Array | null = null
    if (ext === 'svg') {
      const text = await file.text()
      const { sanitized, removed } = sanitizeSvg(text)
      if (!sanitized) throw new Error('SVG が解析できません: ' + removed.join(', '))
      sanitizedRemovals.value = removed
      const blob = new Blob([sanitized], { type: 'image/svg+xml' })
      importedPreviewUrl.value = URL.createObjectURL(blob)
      // SVG → 256px PNG にラスタライズして Rust 側ビルダー用に保持
      pngBytes = await rasterizeSvgToPng(sanitized, 256)
      importMessage.value =
        removed.length > 0
          ? `SVG を sanitize しました (除去: ${removed.length} 件)`
          : `SVG をインポートしました`
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
        throw new Error('PNG ヘッダーが不正です (Magic Byte 不一致)')
      }
      pngBytes = fullBytes
      importedPreviewUrl.value = URL.createObjectURL(file)
      importMessage.value = 'PNG をインポートしました'
    } else {
      throw new Error(`未対応の拡張子: .${ext} (PNG / SVG のみ受付)`)
    }

    // 該当役割を partial → filled に
    filledRoles.add(activeRoleId.value)
    const map = filledSizesByRole.value[activeRoleId.value] ?? []
    if (!map.includes(activeSize.value)) {
      filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
    }

    // 役割マップに登録 (エクスポート時に使用)
    // PNG/SVG はホットスポット情報を持たないので、現在表示されている比率を維持して
    // 新しい primarySize (= 256) 基準の px 値に変換する。
    // ref サイズは「既存アセットの primarySize」優先、なければ activeSize (UI 入力時の基準)。
    // これにより画像サイズが大きく変わっても見た目のホットスポット位置がずれない。
    if (pngBytes) {
      const isNewRole = !creatorAssets.hasAsset(activeRoleId.value)
      const isDefault = hotspotX.value === 4 && hotspotY.value === 4
      const fromSize = hotspotReferenceSize.value
      const finalHotspot =
        isNewRole && isDefault
          ? initialHotspotFor(activeRoleId.value, 256)
          : scaleHotspot({ x: hotspotX.value, y: hotspotY.value }, fromSize, 256)
      setAsset(activeRoleId.value, {
        primary: pngBytes,
        primarySize: 256,
        hotspot: finalHotspot,
        source: 'manual',
      })
      hotspotX.value = finalHotspot.x
      hotspotY.value = finalHotspot.y
    }
  } catch (err) {
    importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    importBusy.value = false
    if (fileInput.value) fileInput.value.value = ''
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
        v-model:hotspot-x="hotspotX"
        v-model:hotspot-y="hotspotY"
        v-model:per-size-hotspot="perSizeHotspot"
        :arrow-assigned="arrowAssigned"
        :assigned-role-count="assignedRoleCount"
        :export-message="exportMessage"
        :export-progress="exportProgress"
        :export-busy="exportBusy"
        :active-role-jp="activeRole.jp"
        :show-advanced-resolutions="showAdvancedResolutions"
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

        <!-- 中央: エディタ -->
        <div class="editor">
          <div class="editor-head">
            <div>
              <h2>
                {{ activeRole.jp }}
                <span class="role-key">{{ activeRole.id }}</span>
              </h2>
              <div class="desc">
                <template v-if="isRequired(activeRole.id)">
                  {{ t('creator.requiredRoleNote', { required: '' }).split('{required}')[0]
                  }}<b style="color: var(--accent)">{{ t('creator.requiredMark') }}</b
                  >{{ t('creator.requiredRoleNote', { required: '' }).split('{required}')[1] }}
                </template>
                <template v-else>
                  {{ t('creator.optionalRoleNote', { en: activeRole.en }) }}
                </template>
              </div>
            </div>
            <div style="display: flex; gap: 6px">
              <BulkImportButton @bulk-auto="pickBulkAuto" @bulk-folder="pickBulkFolder" />
              <input
                ref="fileInput"
                type="file"
                accept=".png,.svg,image/png,image/svg+xml"
                hidden
                @change="onFileChange"
              />
            </div>
          </div>

          <!-- インポート結果メッセージ -->
          <Transition name="fade">
            <div v-if="importMessage" class="import-banner" role="status">
              <UiIcon :name="importMessage.startsWith('失敗') ? 'Alert' : 'Check'" :size="13" />
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

          <div class="canvas-area">
            <div class="canvas-stage">
              <!-- ビッグプレビュー (ホットスポット ドラッグ対応) -->
              <div
                ref="bigpreviewEl"
                :class="['bigpreview', { dragging: hotspotDragging }]"
                :title="t('creator.hotspotHint')"
                tabindex="0"
                @pointerdown="onHotspotPointerDown"
                @pointermove="onHotspotPointerMove"
                @pointerup="onHotspotPointerUp"
                @pointercancel="onHotspotPointerUp"
                @keydown="onHotspotKeydown"
              >
                <div class="crosshair-h" />
                <div class="crosshair-v" />
                <!--
                  ホットスポット表示・操作は親 .bigpreview 側に一本化している
                  (pointerdown/move/up + keydown + `.hot` ドット)。AniThumb は
                  fit モードでアニメ表示だけを担当し、内部 overlay は使わない。
                -->
                <AniThumb
                  v-if="activeAniFrames"
                  :frame-pngs="activeAniFrames.framePngs"
                  :sequence="activeAniFrames.sequence"
                  :durations="activeAniFrames.perStepDurationsMs"
                  :width="hotspotReferenceSize"
                  :height="hotspotReferenceSize"
                  fit
                />
                <img
                  v-else-if="activePreviewUrl"
                  :src="activePreviewUrl"
                  :alt="activeRole.jp"
                  draggable="false"
                  style="width: 90%; height: 90%; image-rendering: pixelated; pointer-events: none"
                />
                <CursorIcon
                  v-else
                  :role="activeRole.id"
                  :size="90"
                  style="color: var(--fg); pointer-events: none"
                />
                <!--
                  画像は `max-width: 90%; max-height: 90%` で中央配置される。
                  純粋に `(hotspotX / refSize) * 100%` で左/上を指定すると
                  220px コンテナの端から計算されてしまい、5% (= 11px) 分の
                  余白を含めてしまうため、実際のホットスポット位置からずれる。
                  ここでは「コンテナ中央 ± 画像サイズの (比率 - 0.5)」で
                  画像領域内の正しいピクセル位置に揃える。
                -->
                <div
                  class="hot"
                  :style="{
                    left: `calc(50% + ${(hotspotX / hotspotReferenceSize - 0.5) * IMAGE_DISPLAY_PCT}%)`,
                    top: `calc(50% + ${(hotspotY / hotspotReferenceSize - 0.5) * IMAGE_DISPLAY_PCT}%)`,
                  }"
                />
                <div class="preview-meta tl">{{ activeSize }} × {{ activeSize }}</div>
                <button
                  class="hotspot-center-btn"
                  :title="t('creator.centerHotspot')"
                  @pointerdown.stop
                  @click.stop="centerHotspot"
                >
                  <UiIcon name="Crosshair" :size="11" />
                </button>
                <div class="preview-meta tr">hotspot {{ hotspotX }},{{ hotspotY }}</div>
              </div>

              <!-- 詳細設定トグル: 解像度別ワークフローを ON/OFF -->
              <div class="advanced-toggle-row">
                <button
                  class="advanced-toggle"
                  :aria-expanded="showAdvancedResolutions"
                  @click="showAdvancedResolutions = !showAdvancedResolutions"
                >
                  <UiIcon
                    name="ChevD"
                    :size="11"
                    :style="{
                      transform: showAdvancedResolutions ? 'rotate(0deg)' : 'rotate(-90deg)',
                      transition: 'transform 160ms',
                    }"
                  />
                  {{ t('creator.advancedSection') }}
                  <span class="advanced-hint">{{ t('creator.advancedHint') }}</span>
                </button>

                <!-- リサンプル切替 (基本フローでも見せておく) -->
                <div class="resample-row">
                  <span>RESAMPLE</span>
                  <div class="btn-group">
                    <button
                      v-for="mode in ['lanczos', 'nearest', 'auto'] as ResampleMode[]"
                      :key="mode"
                      :class="['btn', { active: resample === mode }]"
                      style="height: 26px; font-size: 11px"
                      @click="resample = mode"
                    >
                      {{ mode === 'lanczos' ? 'Lanczos' : mode === 'nearest' ? 'Nearest' : 'Auto' }}
                    </button>
                  </div>
                </div>
              </div>

              <!-- 詳細設定: 解像度別の上書きワークフロー -->
              <Transition name="fade">
                <div v-if="showAdvancedResolutions" class="advanced-panel">
                  <div class="advanced-label">{{ t('creator.perResolutionLabel') }}</div>
                  <SizeStrip
                    :sizes="[...SIZES]"
                    :filled-sizes="filledSizes"
                    :active-size="activeSize"
                    :role="activeRole.id"
                    @select="selectSize"
                  />
                </div>
              </Transition>

              <div class="next-step-row">
                <button
                  class="btn primary"
                  :disabled="!arrowAssigned"
                  :title="!arrowAssigned ? t('creator.requiredMark') : ''"
                  @click="activeTab = 'metadata'"
                >
                  {{ t('creator.nextToMetadata') }}
                  <UiIcon name="ChevD" :size="13" :style="{ transform: 'rotate(-90deg)' }" />
                </button>
              </div>
            </div>
          </div>
        </div>
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

      <AppStatusbar
        :items="[
          { dot: true, text: '編集中: ' + (metaName || 'Untitled') },
          { text: `${filledCount}/17 役割 · ${sizesCovered}/6 解像度` },
          { text: '未保存の変更 3件' },
          { text: 'WebView2 132.0.2957' },
        ]"
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
      @update:model-value="onThemePickerSelect"
      @cancel="onThemePickerCancel"
    />
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

.role-key {
  @apply ml-2 font-mono text-[12px] font-normal text-fg-mute;
}

.preview-meta {
  @apply absolute bottom-2.5 font-mono text-[10px];
}
.preview-meta.tl {
  @apply left-3 text-fg-mute;
}
.preview-meta.tr {
  @apply right-3 text-accent;
}

.hotspot-center-btn {
  /* preview-meta.tl / preview-meta.tr は名前に反して両方とも bottom 配置なので、
     中央ボタンは反対側の右上に置けばどのチップとも衝突しない。 */
  @apply absolute right-2 top-2 inline-flex size-[22px] cursor-pointer items-center justify-center rounded-full border border-accent-line p-0 text-accent;
  background: rgba(0, 0, 0, 0.4);
  transition:
    background 120ms ease,
    transform 120ms ease;
}
.hotspot-center-btn:hover {
  background: rgba(124, 242, 212, 0.15);
  transform: scale(1.08);
}

.next-step-row {
  /* NOTE: var(--border) は未定義 (元コード leftover) — 結果として border 無し。
   * 視覚的現状維持のためそのまま literal で残置。 */
  @apply mt-4 flex justify-end pt-4;
  border-top: 1px solid var(--border);
}
.next-step-row .btn.primary {
  @apply h-9 px-[18px] text-[13px];
}

.resample-row {
  @apply flex items-center gap-2 font-mono text-[10.5px] tracking-[0.04em] text-fg-mute;
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

.import-banner {
  @apply mx-[18px] mb-2 mt-0 flex items-center gap-2 rounded-[8px] border border-accent-line px-3 py-2 text-[12px] text-fg-dim;
  background: rgba(124, 242, 212, 0.06);
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

.advanced-toggle-row {
  @apply flex flex-wrap items-center justify-between gap-4;
}
.advanced-toggle {
  @apply inline-flex cursor-pointer items-center gap-1.5 rounded-[8px] border border-dashed border-line bg-transparent px-2.5 py-1.5 font-mono text-[11.5px] tracking-[0.04em] text-fg-mute;
  transition:
    color 160ms ease,
    border-color 160ms ease;
}
.advanced-toggle:hover {
  @apply border-accent-line text-fg;
}
.advanced-toggle[aria-expanded='true'] {
  @apply border-accent-line text-accent;
}
.advanced-hint {
  @apply ml-1 font-body text-[10.5px] text-fg-dim;
}

.advanced-panel {
  @apply mt-2 rounded-[8px] border border-line px-3 py-2.5;
  background: rgba(124, 242, 212, 0.02);
}
.advanced-label {
  @apply mb-2 font-mono text-[10px] uppercase tracking-[0.16em] text-fg-mute;
}

.editor {
  @apply flex min-w-0 flex-col;
  background: radial-gradient(800px 600px at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%);
}
.editor-head {
  @apply flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-line px-[18px] py-3.5;
}
.editor-head h2 {
  @apply m-0 font-display text-[18px] font-semibold tracking-[-0.01em];
}
.editor-head .desc {
  @apply mt-0.5 text-[12px] text-fg-dim;
}
.canvas-area {
  @apply grid min-h-0 flex-1 overflow-auto p-6;
  grid-template-columns: 1fr;
}
.canvas-stage {
  @apply flex flex-col items-center gap-[18px];
  align-self: center;
  justify-self: center;
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

.bigpreview {
  @apply relative grid size-[220px] min-h-0 cursor-crosshair place-items-center rounded-2xl border border-line-hi;
  background:
    repeating-conic-gradient(rgba(255, 255, 255, 0.025) 0% 25%, transparent 0% 50%) 0 / 18px 18px,
    var(--bg-1);
  box-shadow: var(--shadow-2);
  touch-action: none;
  user-select: none;
}
.bigpreview.dragging .hot {
  border-color: var(--accent-hi);
  background: rgba(124, 242, 212, 0.4);
  box-shadow:
    0 0 16px var(--accent),
    0 0 0 6px rgba(124, 242, 212, 0.18);
}
.bigpreview .hot {
  @apply absolute size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full;
  border: 1.5px solid var(--accent);
  background: rgba(124, 242, 212, 0.2);
  box-shadow: 0 0 12px var(--accent);
  transition:
    box-shadow 120ms ease,
    background 120ms ease;
}
.bigpreview .crosshair-h,
.bigpreview .crosshair-v {
  @apply absolute;
  background: var(--accent-line);
}
.bigpreview .crosshair-h {
  @apply inset-x-0 h-px;
  top: 50%;
}
.bigpreview .crosshair-v {
  @apply inset-y-0 w-px;
  left: 50%;
}
.bigpreview:focus {
  outline: 2px solid var(--accent-line);
  outline-offset: -2px;
}
.bigpreview:focus:not(:focus-visible) {
  outline: none;
}
</style>
