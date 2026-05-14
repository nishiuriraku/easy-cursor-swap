/**
 * テーマ一覧のリアクティブシングルトン。
 * `get_themes` IPC を呼び、ThemeCardData[] にマップして提供する。
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

export function useThemes() {
  return { themes, loading, lastError, refresh }
}
