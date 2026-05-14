/**
 * Marketplace ページのフィルタリングロジック (純関数)。
 *
 * `pages/marketplace.vue` の `filteredGrid` computed から切り出している。
 * テストしやすくするため副作用なし。
 *
 * Featured ストリップに既に出ているエントリ (= `featured` 配列に含まれる id)
 * は Grid から除外し、二重表示を防ぐ。
 */
import type { MarketplaceEntry, MarketplaceTag } from '~/types/marketplace'

export function computeFilteredGrid(
  entries: MarketplaceEntry[],
  featured: MarketplaceEntry[],
  filter: MarketplaceTag,
  searchQuery: string,
): MarketplaceEntry[] {
  const featuredIds = new Set(featured.map((e) => e.id))
  let result = entries.filter((e) => !featuredIds.has(e.id))

  if (filter !== 'all') {
    result = result.filter((e) => e.tags.includes(filter))
  }

  const q = searchQuery.trim().toLowerCase()
  if (q) {
    result = result.filter(
      (e) => e.name.toLowerCase().includes(q) || e.author.toLowerCase().includes(q),
    )
  }

  return result
}
