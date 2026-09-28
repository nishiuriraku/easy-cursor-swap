/**
 * Library のフィルタ・ソート状態 (P08a Step 4 / L1)。
 *
 * `sortKey/sortDir/filter/searchQuery` ref と派生 (`filteredThemes` /
 * `counts` / `sortLabel` / `cycleSort` / `sortBy`) を提供する。
 * 純関数部分は `pages/index.helpers.ts`。
 */
import type { Ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import type { FilterChip, SortDir, SortKey } from '~/pages/index.helpers'
import { countThemes, filterThemes, sortThemes } from '~/pages/index.helpers'

export interface LibraryFilterSortDeps {
  themes: Ref<ThemeCardData[]>
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useLibraryFilterSort(deps: LibraryFilterSortDeps) {
  const { themes, t } = deps
  const searchQuery = ref('')
  const filter = ref<FilterChip>('all')
  const sortKey = ref<SortKey>('updated')
  /** ソート方向。一覧の列ヘッダクリックでトグル、グリッドの cycleSort では `desc` 固定。 */
  const sortDir = ref<SortDir>('desc')

  const filteredThemes = computed(() =>
    sortThemes(
      filterThemes(themes.value, { query: searchQuery.value, filter: filter.value }),
      sortKey.value,
      sortDir.value,
    ),
  )

  const counts = computed(() => countThemes(themes.value))

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

  return {
    searchQuery,
    filter,
    sortKey,
    sortDir,
    filteredThemes,
    counts,
    sortLabel,
    cycleSort,
    sortBy,
  }
}
