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
import { useThemePreviews } from '~/composables/useThemePreviews'
import { useAppSettings } from '~/composables/useAppSettings'
import { useCursorpackOpener } from '~/composables/useCursorpackOpener'

const { t } = useI18n()
// UiIcon / ThemeCard / ApplyModal / AppStatusbar は Nuxt の自動インポートで解決される。

type FilterChip = 'all' | 'favorites' | 'recent' | 'unsigned'
/** 並び替えキー。
 *  - `updated` / `name` / `applied`: グリッド・一覧表示の両方で使う既存キー
 *  - `coverage` / `size`: 一覧表示のヘッダクリック用に追加
 *  なお Q2 で「sortKey/sortDir はグリッドと一覧で共有」と決定したため、
 *  グリッド側のソート巡回ボタンも増えたキーを順番に巡回する。 */
type SortKey = 'name' | 'updated' | 'applied' | 'coverage' | 'size'
type SortDir = 'asc' | 'desc'

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
const applyError = ref<string | null>(null)

// 詳細モーダル制御。モーダルは画面に同時に 1 つしか出さない。
const detailTheme = ref<ThemeCardData | null>(null)
const detailPreviewMap = ref<Record<string, string> | null>(null)
const detailPreviewDetails = ref<Record<
  string,
  import('~/composables/useThemePreviews').RolePreviewDetail
> | null>(null)
const themePreviewCache = useThemePreviews()

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
// design/library-list.jsx の demo データに合わせて tags / size / signed を追加。
// 実機では Rust の get_themes が ThemeSummary 経由でこれらを返すので、ここはあくまで
// Tauri 未起動時 (web preview) のフォールバック表示。
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
    includedRoles: [
      'Arrow',
      'Help',
      'AppStarting',
      'Wait',
      'IBeam',
      'Hand',
      'No',
      'SizeNS',
      'SizeWE',
      'SizeNWSE',
      'SizeNESW',
      'SizeAll',
    ],
    tags: ['animated', 'dark'],
    sizeBytes: 2_202_009,
    signed: true,
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
    tags: ['minimal', 'light'],
    sizeBytes: 419_430,
    signed: true,
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
    includedRoles: [
      'Arrow',
      'Help',
      'AppStarting',
      'Wait',
      'Crosshair',
      'IBeam',
      'NWPen',
      'No',
      'SizeNS',
      'SizeWE',
      'SizeNWSE',
      'SizeNESW',
      'SizeAll',
      'UpArrow',
      'Hand',
      'Pin',
      'Person',
    ],
    tags: ['pixel', 'retro'],
    sizeBytes: 943_718,
    signed: true,
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
    tags: ['seasonal', 'soft'],
    sizeBytes: 1_363_148,
    signed: true,
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
    includedRoles: [
      'Arrow',
      'Help',
      'AppStarting',
      'Wait',
      'Crosshair',
      'IBeam',
      'No',
      'SizeNS',
      'SizeWE',
      'SizeNWSE',
      'SizeNESW',
      'SizeAll',
      'Hand',
    ],
    tags: ['animated', 'neon'],
    sizeBytes: 4_928_307,
    signed: true,
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
    includedRoles: [
      'Arrow',
      'Wait',
      'IBeam',
      'Hand',
      'No',
      'SizeAll',
      'SizeNS',
      'SizeWE',
      'Crosshair',
      'Help',
    ],
    tags: ['draft'],
    sizeBytes: 209_715,
    signed: false,
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

  if (filter.value === 'favorites') result = result.filter((tt) => tt.isFavorite)
  else if (filter.value === 'recent')
    result = result.filter((tt) => Boolean(tt.lastAppliedAt) || tt.applyCount > 0)
  else if (filter.value === 'unsigned') result = result.filter((tt) => tt.signed === false)

  // Q2: sortKey/sortDir はグリッドと一覧で共有。sortDir で昇降を切替。
  const dirSign = sortDir.value === 'asc' ? 1 : -1
  result.sort((a, b) => {
    let cmp = 0
    switch (sortKey.value) {
      case 'name':
        cmp = a.name.localeCompare(b.name, 'ja')
        break
      case 'updated':
        cmp = a.date.localeCompare(b.date)
        break
      case 'applied':
        cmp = a.applyCount - b.applyCount
        break
      case 'coverage':
        cmp = a.includedRoles.length - b.includedRoles.length
        break
      case 'size':
        cmp = (a.sizeBytes ?? 0) - (b.sizeBytes ?? 0)
        break
    }
    return dirSign * cmp
  })

  return result
})

const counts = computed(() => ({
  all: themes.value.length,
  favorites: themes.value.filter((tt) => tt.isFavorite).length,
  recent: themes.value.filter((tt) => Boolean(tt.lastAppliedAt) || tt.applyCount > 0).length,
  unsigned: themes.value.filter((tt) => tt.signed === false).length,
}))

const activeTheme = computed(() => themes.value.find((tt) => tt.isActive))

/** ライブラリ全体のストレージ使用量を MB 表示。
 *  Source of Truth は Rust 側 `ThemeSummary.size_bytes` の合計。
 *  Windows システムスキームは `sizeBytes` が undefined なので除外される。 */
const totalStorageMb = computed(() => {
  const totalBytes = themes.value.reduce((sum, tt) => sum + (tt.sizeBytes ?? 0), 0)
  return (totalBytes / (1024 * 1024)).toFixed(1)
})

// --- ステータスバー用の動的情報 ---
// `useAppSettings` はグローバルシングルトン。Settings 画面で更新されると自動追従する。
const appSettings = useAppSettings()
const signatureVerifyLabel = computed(
  () => `${t('library.statusSignature')}: ${t('library.statusOn')}`,
)
const activeStatusLabel = computed(() =>
  activeTheme.value
    ? `${t('library.statusActive')}: ${activeTheme.value.name}`
    : `${t('library.statusActive')}: ${t('library.statusActiveDefault')}`,
)
const storageStatusLabel = computed(() => `~/.custom_cursors/ — ${totalStorageMb.value} MB`)

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
    const updated = await invokeTauri<string[]>('set_theme_favorite', {
      themeId: id,
      isFavorite: next,
    })
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
async function showDetails(id: string) {
  const found = themes.value.find((tt) => tt.id === id)
  if (!found) return
  detailTheme.value = found
  detailPreviewMap.value = null
  detailPreviewDetails.value = null
  // Windows システムスキームには ID が `windows:` プレフィックス付きでローカルテーマ
  // のキャッシュキーと衝突しないので、そのまま渡す。実体取得が無い場合は null のまま。
  // url 取得と詳細取得は同じキャッシュエントリを再利用する (in-flight 共有)。
  try {
    const [map, details] = await Promise.all([
      themePreviewCache.getMap(id),
      themePreviewCache.getDetails(id),
    ])
    detailPreviewMap.value = map
    detailPreviewDetails.value = details
  } catch (err) {
    console.warn('[Library] preview load for detail failed:', err)
  }
}

function closeDetails() {
  detailTheme.value = null
  detailPreviewMap.value = null
  detailPreviewDetails.value = null
}

/** 詳細モーダルから「適用」を選んだとき。確認モーダル経由で apply を実行する。 */
function applyFromDetail(id: string) {
  closeDetails()
  requestApply(id)
}

/**
 * 詳細モーダルからの「Creator で編集」。テーマを再パッケージして一時ファイル化し、
 * Creator の bulk import 経路で開く。一時ファイルは Rust 側 (tempdir) ではなく
 * OS の TEMP に書き出し、Nuxt から `parse_cursorpack_for_creator` で読み込む。
 */
async function editInCreator(id: string) {
  try {
    const { tempDir, sep } = await import('@tauri-apps/api/path')
    const dir = await tempDir()
    const tempPath = `${dir}${sep()}_easycursorswap_edit_${Date.now()}.cursorpack`
    await invokeTauri<number>('repackage_theme', { themeId: id, outputPath: tempPath })
    closeDetails()
    // Creator ページに遷移し、ロード対象のパスをクエリで渡す。Creator 側で
    // `editThemePath` クエリを拾って parse_cursorpack_for_creator を呼ぶ。
    await navigateTo({ path: '/creator', query: { editPath: tempPath } })
  } catch (err) {
    applyError.value = `編集モードへの遷移に失敗: ${err instanceof Error ? err.message : String(err)}`
  }
}

/** 詳細モーダルからの「複製」。`duplicate_theme` IPC で新 UUID を作りリロードする。 */
async function duplicateTheme(id: string) {
  try {
    await invokeTauri<string>('duplicate_theme', { themeId: id })
    closeDetails()
    await loadThemes()
    void notify({
      title: 'EasyCursorSwap',
      body: `${themes.value.find((tt) => tt.id === id)?.name ?? 'テーマ'} を複製しました`,
      level: 'success',
    })
  } catch (err) {
    applyError.value = `複製に失敗: ${err instanceof Error ? err.message : String(err)}`
  }
}

/** 詳細モーダルからの「エクスポート」。`repackage_theme` で .cursorpack を保存する。 */
async function exportTheme(id: string) {
  try {
    const target = themes.value.find((tt) => tt.id === id)
    if (!target) return
    const { save } = await import('@tauri-apps/plugin-dialog')
    const safeName = target.name.replace(/[^\p{L}\p{N}_-]+/gu, '_').slice(0, 64) || 'theme'
    const outputPath = await save({
      defaultPath: `${safeName}.cursorpack`,
      filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
    })
    if (!outputPath) return
    let bytes: number | null = null
    if (target.kind === 'system') {
      // Windows レジストリスキームはローカルテーマディレクトリを持たないので
      // 専用の export_windows_scheme_as_cursorpack を経由する。`%SystemRoot%`
      // 配下の .cur / .ani をそのまま zip 化する設計。
      const result = await invokeTauri<{ theme_id: string; size_bytes: number }>(
        'export_windows_scheme_as_cursorpack',
        { name: target.name, outputPath },
      )
      bytes = result?.size_bytes ?? null
    } else {
      bytes = await invokeTauri<number>('repackage_theme', { themeId: id, outputPath })
    }
    void notify({
      title: 'EasyCursorSwap',
      body: `${target.name} をエクスポートしました (${bytes ?? '?'} bytes)`,
      level: 'success',
    })
  } catch (err) {
    applyError.value = `エクスポートに失敗: ${err instanceof Error ? err.message : String(err)}`
  }
}

/** 詳細モーダルからの「削除」。確認ダイアログを挟んでから `delete_theme` を実行。 */
async function deleteTheme(id: string) {
  const target = themes.value.find((tt) => tt.id === id)
  if (!target) return
  // ネイティブ confirm はテストしづらいが Tauri WebView では機能するので暫定使用。
  // 将来的には専用の確認モーダルに置き換える。
  const ok = window.confirm(`「${target.name}」を完全に削除します。この操作は元に戻せません。`)
  if (!ok) return
  try {
    await invokeTauri<void>('delete_theme', { themeId: id })
    closeDetails()
    await loadThemes()
    void notify({
      title: 'EasyCursorSwap',
      body: `${target.name} を削除しました`,
      level: 'info',
    })
  } catch (err) {
    applyError.value = `削除に失敗: ${err instanceof Error ? err.message : String(err)}`
  }
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

const sortLabel = computed(() => {
  const map: Record<SortKey, string> = {
    name: t('library.sortName'),
    updated: t('library.sortUpdated'),
    applied: t('library.sortApplied'),
    coverage: t('library.colCoverage'),
    size: t('library.colSize'),
  }
  return map[sortKey.value]
})

/** グリッド側のソートボタン: 主要 3 キーを巡回。新キー (coverage/size) は
 *  一覧側の列ヘッダクリックでのみ立てる。グリッド表示中に列ヘッダで coverage 等を
 *  選んでも、ボタン押下で巡回するときは元の 3 キーに戻る挙動。 */
function cycleSort() {
  const order: SortKey[] = ['updated', 'name', 'applied']
  const idx = order.indexOf(sortKey.value)
  sortKey.value = order[(idx + 1) % order.length]!
  sortDir.value = 'desc'
}

/** 一覧表示の列ヘッダクリック: 同じキーなら方向トグル、別キーなら desc から開始。 */
function sortBy(key: SortKey) {
  if (sortKey.value === key) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortKey.value = key
    sortDir.value = 'desc'
  }
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
  /** theme.json の `tags` フィールドをそのまま転送 (Phase 5-3 一覧表示で chip 描画) */
  tags: string[]
  /** テーマディレクトリ全体のバイト合計 (一覧の「サイズ」列用) */
  size_bytes: number
  /** `metadata.signature.is_some()` の結果 (検証ではなく存在判定のみ) */
  signed: boolean
  /** 最終適用日時 (RFC3339)。一度も適用されていなければ null。 */
  last_applied_at: string | null
}

/** `list_windows_schemes` のレスポンス。Windows レジストリ HKCU\Cursors\Schemes 由来。 */
interface IpcWindowsScheme {
  name: string
  cursor_paths: Record<string, string>
  role_count: number
  /** Rust 側で `paths_match_current_registry` 判定済み。現在実態と一致するなら true。 */
  is_active?: boolean
}

/**
 * Windows レジストリのスキームを ThemeCardData に変換する。
 *
 * - id は `windows:<name>` のプレフィックスでローカルテーマと衝突を避ける
 * - kind: 'system' を立てて UI 側でバッジ・編集不可表示に切り替える
 * - included_roles は cursor_paths のキー (空でないもの) を使う
 * - active 判定は Rust 側の `paths_match_current_registry` の結果 (`is_active`)
 *   をそのまま採用する。フロントで再判定すると IPC 往復が増えるため。
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
    isActive: s.is_active === true,
    includedRoles,
    kind: 'system',
    // Windows システムスキームに付随しない情報。一覧表示では tags = []、
    // sizeBytes = undefined ('—' 表示)、signed = true (システム提供なので信頼) として扱う。
    tags: [],
    sizeBytes: undefined,
    signed: true,
    lastAppliedAt: null,
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

    const local: ThemeCardData[] = (localList ?? []).map((tt) => ({
      id: tt.id,
      name: tt.name,
      author: tt.author,
      version: tt.version,
      date: tt.created_at,
      applyCount: tt.apply_count,
      isFavorite: tt.is_favorite,
      isActive: tt.is_active,
      includedRoles: tt.included_roles,
      kind: 'local' as const,
      tags: tt.tags,
      sizeBytes: tt.size_bytes,
      signed: tt.signed,
      lastAppliedAt: tt.last_applied_at,
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

/**
 * 「テーマ状態が変わったかも」というシグナルを 3 経路から拾う。
 *
 * 1. `easycs:cursors-changed` (DOM CustomEvent): default.vue の PanicFlow done フック
 *    と同じウィンドウから dispatch される。Tauri の listen に依らない確実経路。
 * 2. `focus` (window): 別ウィンドウやコントロールパネルでカーソルを変更後に
 *    EasyCursorSwap へ戻ってきたタイミング。
 * 3. `visibilitychange` (document): タブ非表示 → 表示時。focus と相補。
 *
 * いずれもデバウンスせずそのまま `loadThemes` を呼ぶ。`get_themes` 自体が
 * in-flight 共有しているので連発しても安全。
 */
function onExternalCursorsMaybeChanged() {
  void loadThemes()
}
function onWindowFocus() {
  void loadThemes()
}
function onVisibilityChange() {
  if (typeof document === 'undefined') return
  if (document.visibilityState === 'visible') void loadThemes()
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

// Explorer から渡された .cursorpack を受け取り、既存の importByPath フローに流す。
const cursorpackOpener = useCursorpackOpener((path) => {
  void importByPath(path)
})

onMounted(async () => {
  await loadThemes()
  await setupTauriDrop()
  await setupCursorChangeListener()
  if (typeof window !== 'undefined') {
    window.addEventListener('easycs:cursors-changed', onExternalCursorsMaybeChanged)
    window.addEventListener('focus', onWindowFocus)
  }
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', onVisibilityChange)
  }
  // appSettings は本ページ起動時に常時必要 (active_theme_id 等)。初回ロードのみ取りに行く。
  // 既に Settings 画面などで取得済みならキャッシュが返る。
  await appSettings.load().catch(() => null)
  // .cursorpack のファイル関連付け経由インポートを開始
  void cursorpackOpener.start()
})

onUnmounted(() => {
  void cursorpackOpener.stop()
  if (unlistenDrop) {
    unlistenDrop()
    unlistenDrop = null
  }
  if (unlistenCursorChange) {
    unlistenCursorChange()
    unlistenCursorChange = null
  }
  if (typeof window !== 'undefined') {
    window.removeEventListener('easycs:cursors-changed', onExternalCursorsMaybeChanged)
    window.removeEventListener('focus', onWindowFocus)
  }
  if (typeof document !== 'undefined') {
    document.removeEventListener('visibilitychange', onVisibilityChange)
  }
})
</script>

<template>
  <div class="library-host">
    <LibraryToolbar v-model:search-query="searchQuery" @open-import="openImportDialog" />

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
        <div v-for="i in 6" :key="i" class="card skeleton-card" />
      </div>

      <LibraryEmptyState
        v-else-if="filteredThemes.length === 0 && !searchQuery"
        @open-import="openImportDialog"
      />

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
        <div class="lib-row lib-head" role="row">
          <div class="lt-col lt-fav" role="columnheader" />
          <div class="lt-col lt-preview" role="columnheader">{{ t('library.colPreview') }}</div>
          <div
            :class="['lt-col', 'lt-name', 'lt-sortable', { active: sortKey === 'name' }]"
            role="columnheader"
            :aria-sort="
              sortKey === 'name' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'
            "
            tabindex="0"
            @click="sortBy('name')"
            @keydown.enter.prevent="sortBy('name')"
            @keydown.space.prevent="sortBy('name')"
          >
            {{ t('library.colNameAuthor') }}
            <span v-if="sortKey === 'name'" class="sort-dir">{{
              sortDir === 'asc' ? '↑' : '↓'
            }}</span>
          </div>
          <div
            :class="['lt-col', 'lt-cov', 'lt-sortable', { active: sortKey === 'coverage' }]"
            role="columnheader"
            :aria-sort="
              sortKey === 'coverage' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'
            "
            tabindex="0"
            @click="sortBy('coverage')"
            @keydown.enter.prevent="sortBy('coverage')"
            @keydown.space.prevent="sortBy('coverage')"
          >
            {{ t('library.colCoverage') }}
            <span v-if="sortKey === 'coverage'" class="sort-dir">{{
              sortDir === 'asc' ? '↑' : '↓'
            }}</span>
          </div>
          <div class="lt-col lt-ver" role="columnheader">{{ t('library.colVersion') }}</div>
          <div
            :class="['lt-col', 'lt-date', 'lt-sortable', { active: sortKey === 'updated' }]"
            role="columnheader"
            :aria-sort="
              sortKey === 'updated' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'
            "
            tabindex="0"
            @click="sortBy('updated')"
            @keydown.enter.prevent="sortBy('updated')"
            @keydown.space.prevent="sortBy('updated')"
          >
            {{ t('library.colUpdated') }}
            <span v-if="sortKey === 'updated'" class="sort-dir">{{
              sortDir === 'asc' ? '↑' : '↓'
            }}</span>
          </div>
          <div
            :class="['lt-col', 'lt-size', 'lt-sortable', { active: sortKey === 'size' }]"
            role="columnheader"
            :aria-sort="
              sortKey === 'size' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'
            "
            tabindex="0"
            @click="sortBy('size')"
            @keydown.enter.prevent="sortBy('size')"
            @keydown.space.prevent="sortBy('size')"
          >
            {{ t('library.colSize') }}
            <span v-if="sortKey === 'size'" class="sort-dir">{{
              sortDir === 'asc' ? '↑' : '↓'
            }}</span>
          </div>
          <div class="lt-col lt-sig" role="columnheader">{{ t('library.colSignature') }}</div>
        </div>

        <ThemeRow
          v-for="theme in filteredThemes"
          :key="theme.id"
          :theme="theme"
          @toggle-favorite="toggleFavorite"
          @show-details="showDetails"
        />
      </div>
    </div>

    <!-- ステータスバー: 各項目は config / themes 由来で動的に算出。 -->
    <AppStatusbar
      :items="[
        { dot: true, text: activeStatusLabel },
        { text: signatureVerifyLabel },
        { text: storageStatusLabel },
      ]"
    />

    <!-- 詳細モーダル (テーマカードのシェブロンで開く) -->
    <ThemeDetailModal
      :theme="detailTheme"
      :preview-map="detailPreviewMap"
      :preview-details="detailPreviewDetails"
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

    <!-- 適用エラー (簡易バナー) -->
    <Transition name="fade">
      <div v-if="applyError" class="apply-error" role="alert">
        <UiIcon name="Alert" :size="14" />
        適用に失敗しました: {{ applyError }}
        <button
          class="btn ghost"
          style="height: 24px; margin-left: auto"
          @click="applyError = null"
        >
          <UiIcon name="X" :size="11" />
        </button>
      </div>
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

.skeleton-card {
  @apply h-[280px];
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
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.apply-error {
  @apply fixed bottom-12 left-1/2 z-[90] flex min-w-[320px] max-w-[80%] -translate-x-1/2 items-center gap-2.5 rounded-[8px] border px-3.5 py-2.5 text-[12.5px] backdrop-blur-[12px];
  background: rgba(255, 107, 138, 0.12);
  border-color: rgba(255, 107, 138, 0.4);
  color: #ffb8c5;
  box-shadow: var(--shadow-2);
}
</style>
