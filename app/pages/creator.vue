<script setup lang="ts">
/**
 * クリエイターモード (Phase 5-5)
 *
 * design/creator.jsx を Vue 化したもの。3 カラム構成:
 *  - 左:  17 役割リスト (filled/partial/empty ドット付き)
 *  - 中央: ビッグプレビュー + 6 サイズストリップ + リサンプル切替
 *  - 右:  Hotspot / アセット / Validation の各プロパティ
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
import BulkImportButton from '~/components/creator/BulkImportButton.vue'
import BulkImportPreviewModal, {
  type ApplyPayload,
} from '~/components/creator/BulkImportPreviewModal.vue'
import NewThemeStartModal from '~/components/creator/NewThemeStartModal.vue'
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
  const ratioX = clamp((e.clientX - rect.left) / rect.width, 0, 1)
  const ratioY = clamp((e.clientY - rect.top) / rect.height, 0, 1)
  const ref = hotspotReferenceSize.value
  hotspotX.value = Math.round(ratioX * ref)
  hotspotY.value = Math.round(ratioY * ref)
}

function onHotspotPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  hotspotDragging.value = true
  ;(e.currentTarget as Element).setPointerCapture?.(e.pointerId)
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

function selectSize(s: number) {
  activeSize.value = s
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

interface ExportResult {
  theme_id: string
  size_bytes: number
  signed: boolean
  key_id: string | null
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

/** クリエイターの全状態を `.cursorpack` として書き出す (ストリーム式)。 */
async function exportCursorpack(opts: { sign: boolean }) {
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
    const { save } = await import('@tauri-apps/plugin-dialog')
    const safeName = metaName.value.replace(/[^\p{L}\p{N}_-]+/gu, '_').slice(0, 64) || 'theme'
    const target = await save({
      defaultPath: `${safeName}.cursorpack`,
      filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
    })
    if (!target) {
      exportMessage.value = '保存先が選択されませんでした'
      return
    }

    // build_id は時刻 + 乱数で衝突回避
    const buildId = `build-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
    currentBuildId.value = buildId

    // 進捗イベントを購読 (build_id でフィルタ)
    try {
      const { listen } = await import('@tauri-apps/api/event')
      unlisten = await listen<BuildProgress>('build-progress', (e) => {
        if (e.payload.buildId === buildId) exportProgress.value = e.payload
      })
    } catch {
      // Web 開発時は購読をスキップ
    }

    const roles = toExportPayload(resample.value)

    const result = await invokeTauri<ExportResult>('export_cursorpack_streamed', {
      req: {
        buildId,
        nameJa: metaName.value,
        nameEn: metaNameEn.value || null,
        author: metaAuthor.value || null,
        version: metaVersion.value,
        requiresOsShadow: shadowEnabled.value,
        roles,
        outputPath: target,
        sign: opts.sign,
      },
    })

    if (!result) throw new Error('エクスポート結果が空でした')
    const sigText = result.signed ? `署名: ${result.key_id ?? '?'}` : '未署名'
    exportMessage.value = `.cursorpack を書き出しました (${result.size_bytes} bytes, ${sigText}) → ${target}`
  } catch (err) {
    exportMessage.value = `エクスポート失敗: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    if (unlisten) unlisten()
    currentBuildId.value = null
    exportBusy.value = false
    // 完了表示は 3 秒残す
    setTimeout(() => {
      if (!exportBusy.value) exportProgress.value = null
    }, 3000)
  }
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
 * 統合エントリ。png/svg/cur/ico/.cursorpack をまとめて選べるダイアログを開き、
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
        extensions: ['png', 'svg', 'cur', 'ico', 'cursorpack'],
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
        @export="exportCursorpack"
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
        @dismiss-export-message="exportMessage = null"
        @cancel-export="cancelExport"
      />

      <!-- 3 カラムグリッド (assign タブのみ) -->
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
                @pointerdown="onHotspotPointerDown"
                @pointermove="onHotspotPointerMove"
                @pointerup="onHotspotPointerUp"
                @pointercancel="onHotspotPointerUp"
              >
                <div class="crosshair-h" />
                <div class="crosshair-v" />
                <img
                  v-if="activePreviewUrl"
                  :src="activePreviewUrl"
                  :alt="activeRole.jp"
                  draggable="false"
                  style="
                    max-width: 90%;
                    max-height: 90%;
                    image-rendering: pixelated;
                    pointer-events: none;
                  "
                />
                <CursorIcon
                  v-else
                  :role="activeRole.id"
                  :size="90"
                  style="color: var(--fg); pointer-events: none"
                />
                <div
                  class="hot"
                  :style="{
                    left: (hotspotX / hotspotReferenceSize) * 100 + '%',
                    top: (hotspotY / hotspotReferenceSize) * 100 + '%',
                  }"
                />
                <div class="preview-meta tl">{{ activeSize }} × {{ activeSize }}</div>
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
            </div>
          </div>
        </div>

        <CreatorPropertiesPane
          v-model:hotspot-x="hotspotX"
          v-model:hotspot-y="hotspotY"
          v-model:per-size-hotspot="perSizeHotspot"
          v-model:shadow-enabled="shadowEnabled"
          :show-advanced-resolutions="showAdvancedResolutions"
          :imported-preview-url="importedPreviewUrl"
          :sanitized-removals="sanitizedRemovals"
          :resample="resample"
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
.creator-host {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
}

.draft-tag {
  color: var(--fg-mute);
  font-family: var(--font-mono);
  font-size: 10.5px;
  margin-left: 6px;
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.pane-head h6 {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--fg-mute);
  font-weight: 500;
}

.role-key {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--fg-mute);
  font-weight: 400;
  margin-left: 8px;
}

.preview-meta {
  position: absolute;
  bottom: 10px;
  font-family: var(--font-mono);
  font-size: 10px;
}
.preview-meta.tl {
  left: 12px;
  color: var(--fg-mute);
}
.preview-meta.tr {
  right: 12px;
  color: var(--accent);
}

.resample-row {
  display: flex;
  gap: 8px;
  align-items: center;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
  letter-spacing: 0.04em;
}

.color-chips {
  display: flex;
  gap: 4px;
}
.cc {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: 1px solid var(--line);
}

.validation-body {
  gap: 8px;
  font-size: 11.5px;
  font-family: var(--font-mono);
  color: var(--fg-dim);
}
.vrow {
  display: flex;
  justify-content: space-between;
}
.vrow .ok {
  color: var(--accent);
}
.vrow .warn {
  color: var(--amber);
}
.vrow .dim {
  color: var(--fg-dim);
}

.import-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin: 0 18px 8px;
  background: rgba(124, 242, 212, 0.06);
  border: 1px solid var(--accent-line);
  border-radius: 8px;
  font-size: 12px;
  color: var(--fg-dim);
}

.export-progress {
  margin: 0 18px 8px;
  padding: 8px 12px;
  background: rgba(124, 242, 212, 0.04);
  border: 1px solid var(--accent-line);
  border-radius: 8px;
  font-size: 12px;
  color: var(--fg-dim);
}
.export-progress-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.export-progress-label {
  font-variant-numeric: tabular-nums;
}
.export-progress-bar {
  height: 4px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
}
.export-progress-fill {
  height: 100%;
  background: var(--accent);
  transition: width 120ms ease-out;
}

.metadata-pane {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px 32px;
  background: radial-gradient(800px 600px at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%);
}
.metadata-grid {
  max-width: 760px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.advanced-toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.advanced-toggle {
  background: transparent;
  border: 1px dashed var(--line);
  color: var(--fg-mute);
  padding: 6px 10px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11.5px;
  font-family: var(--font-mono);
  letter-spacing: 0.04em;
  transition:
    color 160ms ease,
    border-color 160ms ease;
}
.advanced-toggle:hover {
  color: var(--fg);
  border-color: var(--accent-line);
}
.advanced-toggle[aria-expanded='true'] {
  color: var(--accent);
  border-color: var(--accent-line);
}
.advanced-hint {
  color: var(--fg-dim);
  font-family: var(--font-body);
  font-size: 10.5px;
  margin-left: 4px;
}

.advanced-panel {
  margin-top: 8px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(124, 242, 212, 0.02);
}
.advanced-label {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--fg-mute);
  margin-bottom: 8px;
}
</style>
