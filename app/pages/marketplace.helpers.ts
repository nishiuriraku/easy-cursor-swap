/**
 * Marketplace ページのフィルタリングロジック (純関数)。
 *
 * `pages/marketplace.vue` の `filteredGrid` computed から切り出している。
 * テストしやすくするため副作用なし。
 *
 * 2026-05-14: Featured ストリップ廃止に伴い、Featured 除外ロジックを削除し
 * 全エントリを一つのグリッドに並べる方式に統一。
 */
import type { MarketplaceEntry, MarketplaceTag } from '~/types/marketplace'

export function computeFilteredGrid(
  entries: MarketplaceEntry[],
  filter: MarketplaceTag,
  searchQuery: string,
): MarketplaceEntry[] {
  let result = [...entries]

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
