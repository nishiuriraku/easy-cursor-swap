/**
 * テーマ一覧のリアクティブシングルトン。
 * `get_themes` IPC を呼び、ThemeCardData[] にマップして提供する。
 * 複数画面で同じインスタンスを参照したいので Pinia 不使用のシンプル composable で実装。
 */
import { ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { invokeTauri } from './useTauri'

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

const themes = ref<ThemeCardData[]>([])
const loading = ref(false)
const lastError = ref<string | null>(null)
let inflight: Promise<ThemeCardData[]> | null = null

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
    includedRoles: t.included_roles,
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
