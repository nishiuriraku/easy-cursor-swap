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

const { t } = useI18n()
// UiIcon / ThemeCard / ApplyModal は Nuxt の自動インポートで解決される。

type FilterChip = 'all' | 'favorites' | 'recent'
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
// Theme mutation IPC は useThemes に集約 (audit B8-SIZE-001)。
// `themes` ref と `refresh` は本ページが自前管理する `loadThemes()` を使い続けるため、
// メソッドだけ取り出す。
const {
  applyTheme: applyThemeIpc,
  setFavorite: setFavoriteIpc,
  repackageTheme: repackageThemeIpc,
  duplicateTheme: duplicateThemeIpc,
  deleteTheme: deleteThemeIpc,
  inspectCursorpack: inspectCursorpackIpc,
  importCursorpack: importCursorpackIpc,
} = useThemes()

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
}))

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
      await invokeTauri<void>('apply_windows_scheme', { name: target.name })
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
    await repackageThemeIpc(id, tempPath)
    closeDetails()
    // Creator ページに遷移し、ロード対象のパスをクエリで渡す。Creator 側で
    // `editThemePath` クエリを拾って parse_cursorpack_for_creator を呼ぶ。
    await navigateTo({ path: '/creator', query: { editPath: tempPath } })
  } catch (err) {
    applyError.value = t('library.errEditModeTransition', {
      detail: err instanceof Error ? err.message : String(err),
    })
  }
}

/** 詳細モーダルからの「複製」。`duplicate_theme` IPC で新 UUID を作りリロードする。 */
async function duplicateTheme(id: string) {
  try {
    await duplicateThemeIpc(id)
    closeDetails()
    await loadThemes()
    void notify({
      title: 'EasyCursorSwap',
      body: t('library.notifyDuplicated', {
        name: themes.value.find((tt) => tt.id === id)?.name ?? t('library.fallbackThemeName'),
      }),
      level: 'success',
    })
  } catch (err) {
    applyError.value = t('library.errDuplicate', {
      detail: err instanceof Error ? err.message : String(err),
    })
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
      bytes = await repackageThemeIpc(id, outputPath)
    }
    void notify({
      title: 'EasyCursorSwap',
      body: t('library.notifyExported', {
        name: target.name,
        bytes: bytes ?? t('library.bytesUnknown'),
      }),
      level: 'success',
    })
  } catch (err) {
    applyError.value = t('library.errExport', {
      detail: err instanceof Error ? err.message : String(err),
    })
  }
}

/** 詳細モーダルからの「削除」。確認ダイアログを挟んでから `delete_theme` を実行。 */
async function deleteTheme(id: string) {
  const target = themes.value.find((tt) => tt.id === id)
  if (!target) return
  // UI で削除ボタンを disabled にしているが、IPC 直叩きや競合状態 (削除直前に
  // 別経路で apply されたケース) を防ぐため二重チェック。
  if (target.isActive) {
    applyError.value = t('library.errDeleteActive')
    return
  }
  // ネイティブ confirm はテストしづらいが Tauri WebView では機能するので暫定使用。
  // 将来的には専用の確認モーダルに置き換える。
  const ok = window.confirm(t('library.confirmDeleteMsg', { name: target.name }))
  if (!ok) return
  try {
    await deleteThemeIpc(id)
    closeDetails()
    await loadThemes()
    void notify({
      title: 'EasyCursorSwap',
      body: t('library.notifyDeleted', { name: target.name }),
      level: 'info',
    })
  } catch (err) {
    applyError.value = t('library.errDelete', {
      detail: err instanceof Error ? err.message : String(err),
    })
  }
}

// ブラウザの DragEvent は dataTransfer.files に絶対パスを含めないため、
// Tauri v2 のウィンドウ drag-drop イベントで実ファイルパスを取得する。
// onMounted で `onDragDropEvent` 購読 → unlisten をクリーンアップ用に保持。
let unlistenDrop: (() => void) | null = null

async function importByPath(path: string) {
  try {
    // まず軽量検査して既存テーマと衝突するか確認
    const inspection = await inspectCursorpackIpc(path)
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
    applyError.value = t('library.errImport', { detail: msg })
    console.error('[Library] import failed:', err)
  }
}

async function actuallyImport(path: string) {
  const id = await importCursorpackIpc(path)
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
    applyError.value = t('library.errImport', { detail: msg })
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
    // sizeBytes = undefined ('—' 表示)、signed = false として扱う。
    // signed=false の理由: Marketplace の Ed25519 検証済テーマ (= "公式" バッジ) と
    // OS 提供スキームは別概念。Windows のものは「信頼できる」が「公式インデックス由来」では
    // ないため、ThemeDetailDrawer の「公式」ピルを点灯させない。
    tags: [],
    sizeBytes: undefined,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: undefined,
    license: null,
    homepage: null,
  }
}

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
      invokeTauri<IpcWindowsScheme[]>('list_windows_schemes').catch((err) => {
        console.warn('[Library] list_windows_schemes failed (non-fatal):', err)
        return [] as IpcWindowsScheme[]
      }),
    ])

    const local: ThemeCardData[] = (localList ?? []).map(mapLocalSummaryToCard)

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

// 外部カーソル変更検知 — Rust 側で SPI_SETCURSORS を購読し、変更があれば UI 更新
let unlistenCursorChange: (() => void) | null = null
async function setupCursorChangeListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenCursorChange = await listen('cursor-changed', () => {
      console.info('[Library] cursor-changed event received → reload')
      // 元アクティブテーマは Creator overwrite + apply 経路で再生成された可能性が高い。
      // useCreatorExport 側の invalidate を A 経路としつつ、外部 apply / panic restore
      // など Creator を経由しない経路の取りこぼし対策として B 経路でも invalidate を打つ。
      const previouslyActiveId = themes.value.find((t) => t.isActive)?.id
      if (previouslyActiveId) themePreviewCache.invalidate(previouslyActiveId)
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
        {{ t('library.applyFailedBanner', { detail: applyError }) }}
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
