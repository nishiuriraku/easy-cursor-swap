/**
 * テーマ一覧のリアクティブシングルトン + テーマ操作 IPC 集約。
 *
 * 旧設計では `get_themes` だけを wrap し、apply / delete / duplicate /
 * repackage / set_favorite / inspect / import の 7 IPC が `pages/index.vue` から
 * 直接 `invokeTauri` 呼びされていた (audit B8-SIZE-001)。本 composable に
 * 集約することで page を presentation 中心に縮め、`docs/architecture.json` の
 * `useThemes.ipc_calls` の宣言と実態を一致させる。
 *
 * 各 mutation 系メソッドは「IPC 呼出 → 失敗時 throw → 成功時の refresh は
 * 呼出側で `void refresh()`」のシンプルな規約。呼出側で UI toast や confirm を
 * 出してから呼ぶフローが多いので、ここで自動 refresh しない。
 *
 * 複数画面で同じインスタンスを参照したいので Pinia 不使用のシンプル composable で実装。
 */
import { ref } from 'vue'
import type { ThemeCardData, ThemeKind } from '~/types/theme'
import { invokeTauri } from './useTauri'

/**
 * Rust 側 theme::types::ThemeSummary に対応する IPC ペイロード。
 * フィールド名は serde 既定 (snake_case) のままで、フロント型 ThemeCardData
 * には mapSummary で camelCase に揃えてコピーする。
 *
 * 過去にここで description / signed / tags / size_bytes / last_applied_at /
 * schema_version / license / homepage を **取りこぼしていた** ため、
 * テーマ詳細モーダルの DESCRIPTION 段落が出ず、ThemeRow の signed 判定が
 * 全テーマ "署名済" 扱いになるバグの原因になっていた。Rust を真とする。
 */
interface IpcThemeSummary {
  id: string
  name: string
  author: string | null
  version: string
  created_at: string
  is_active: boolean
  is_favorite: boolean
  apply_count: number
  last_applied_at: string | null
  included_roles: string[]
  path: string
  tags: string[]
  size_bytes: number
  signed: boolean
  description?: string | null
  schema_version: number
  license?: string | null
  homepage?: string | null
  source?: string
}

/**
 * `inspect_cursorpack` IPC の戻り型。インポート前に既存テーマと衝突するかを
 * 確認する用。`existing` が non-null なら同 id のテーマが既にあり、UI 側で
 * 上書き確認モーダルを出す。
 */
export interface InspectionResult {
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

const themes = ref<ThemeCardData[]>([])
const loading = ref(false)
const lastError = ref<string | null>(null)
let inflight: Promise<ThemeCardData[]> | null = null

export function mapSourceToKind(source: string | undefined): ThemeKind {
  if (source === 'marketplace') return 'marketplace'
  return 'local'
}

function mapSummary(t: IpcThemeSummary): ThemeCardData {
  return {
    id: t.id,
    name: t.name,
    author: t.author,
    version: t.version,
    date: t.created_at,
    applyCount: t.apply_count,
    isFavorite: t.is_favorite,
    isActive: t.is_active,
    kind: mapSourceToKind(t.source),
    includedRoles: t.included_roles,
    tags: t.tags,
    sizeBytes: t.size_bytes,
    signed: t.signed,
    description: t.description ?? null,
    schemaVersion: t.schema_version,
    license: t.license ?? null,
    homepage: t.homepage ?? null,
    lastAppliedAt: t.last_applied_at,
  }
}

async function refresh(): Promise<ThemeCardData[]> {
  if (inflight) return inflight
  loading.value = true
  lastError.value = null
  inflight = (async () => {
    try {
      const list = await invokeTauri<IpcThemeSummary[]>('get_themes')
      themes.value = (list ?? []).map(mapSummary)
      return themes.value
    } catch (err) {
      lastError.value = err instanceof Error ? err.message : String(err)
      console.warn('[useThemes] get_themes failed:', err)
      return themes.value
    } finally {
      loading.value = false
      inflight = null
    }
  })()
  return inflight
}

// ─────────────────────────────────────────────────────────────
// Mutation IPC ラッパー (audit B8-SIZE-001 で page から集約)
//
// 規約: throw on error / 成功時の refresh は caller が判断。
// caller は通常 `await applyTheme(id); await refresh()` のように使う。
// ─────────────────────────────────────────────────────────────

async function applyTheme(id: string): Promise<void> {
  await invokeTauri<void>('apply_theme', { themeId: id })
}

async function setFavorite(id: string, value: boolean): Promise<string[]> {
  return (
    (await invokeTauri<string[]>('set_theme_favorite', { themeId: id, isFavorite: value })) ?? []
  )
}

async function repackageTheme(id: string, outputPath: string): Promise<number> {
  return (await invokeTauri<number>('repackage_theme', { themeId: id, outputPath })) ?? 0
}

async function duplicateTheme(id: string): Promise<string | null> {
  return (await invokeTauri<string>('duplicate_theme', { themeId: id })) ?? null
}

async function deleteTheme(id: string): Promise<void> {
  await invokeTauri<void>('delete_theme', { themeId: id })
}

async function inspectCursorpack(path: string): Promise<InspectionResult | null> {
  return (await invokeTauri<InspectionResult>('inspect_cursorpack', { path })) ?? null
}

async function importCursorpack(path: string): Promise<string | null> {
  return (await invokeTauri<string>('import_cursorpack', { path })) ?? null
}

export function useThemes() {
  return {
    themes,
    loading,
    lastError,
    refresh,
    applyTheme,
    setFavorite,
    repackageTheme,
    duplicateTheme,
    deleteTheme,
    inspectCursorpack,
    importCursorpack,
  }
}
