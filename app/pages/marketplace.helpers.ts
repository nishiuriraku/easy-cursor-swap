/**
 * Marketplace ページのフィルタリングロジック (純関数)。
 *
 * `pages/marketplace.vue` の `filteredGrid` computed から切り出している。
 * テストしやすくするため副作用なし。
 *
 * 2026-05-14: Featured ストリップ廃止に伴い、Featured 除外ロジックを削除し
 * 全エントリを一つのグリッドに並べる方式に統一。
 */
import type { MarketplaceEntry, MarketplaceName, MarketplaceTag } from '~/types/marketplace'

/**
 * `MarketplaceName` 内の全ての文字列値を 1 本の検索用テキストに連結する。
 * plain string ならそれだけ、locale map なら全 value を space 区切りで連結。
 * これにより JA モードで `"Mint"` (en 値) を検索しても、`"ミント"` (ja 値) を
 * 検索しても、同じエントリにマッチする (どちらでマッチさせるかが意図不明な
 * UX を避ける)。
 */
function nameHaystack(name: MarketplaceName): string {
  if (typeof name === 'string') return name
  return Object.values(name)
    .filter((v): v is string => typeof v === 'string')
    .join(' ')
}

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
      (e) => nameHaystack(e.name).toLowerCase().includes(q) || e.author.toLowerCase().includes(q),
    )
  }

  return result
}
