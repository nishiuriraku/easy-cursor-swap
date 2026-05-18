import { describe, it, expect } from 'vitest'
import { computeFilteredGrid } from '~/pages/marketplace.helpers'
import type { MarketplaceEntry } from '~/types/marketplace'

function entry(overrides: Partial<MarketplaceEntry>): MarketplaceEntry {
  return {
    id: overrides.id ?? 'x',
    name: overrides.name ?? 'X',
    author: overrides.author ?? 'a',
    authorGithub: 'a',
    sha256: '00',
    signature: 'AA==',
    authorPubkeyId: 'k',
    downloadUrl: 'https://example.com/p',
    version: '1.0.0',
    downloadCount: 0,
    includedRoles: [],
    tags: overrides.tags ?? [],
    highlight: overrides.highlight ?? null,
    verified: true,
    previewBaseUrl: undefined,
    ...overrides,
  }
}

const baseEntries: MarketplaceEntry[] = [
  entry({ id: 'a', name: 'A', tags: ['minimal'], highlight: 'new' }),
  entry({ id: 'b', name: 'B', tags: ['minimal'] }),
  entry({ id: 'c', name: 'C', tags: ['dark'] }),
]

describe('computeFilteredGrid', () => {
  it('filter "all" + searchQuery 空ならすべてのエントリを返す', () => {
    const grid = computeFilteredGrid(baseEntries, 'all', '')
    expect(grid.map((e) => e.id)).toEqual(['a', 'b', 'c'])
  })

  it('tag フィルタはタグに合致するエントリのみ返す', () => {
    const grid = computeFilteredGrid(baseEntries, 'minimal', '')
    expect(grid.map((e) => e.id)).toEqual(['a', 'b'])
  })

  it('searchQuery は name / author 部分一致 (case-insensitive)', () => {
    const grid = computeFilteredGrid(baseEntries, 'all', 'C')
    expect(grid.map((e) => e.id)).toEqual(['c'])
  })

  it('searchQuery が空白だけならフィルタしない', () => {
    const grid = computeFilteredGrid(baseEntries, 'all', '   ')
    expect(grid.length).toBe(3)
  })

  it('searchQuery の前後空白は trim され、語そのもので部分一致する', () => {
    const grid = computeFilteredGrid(baseEntries, 'all', '  c  ')
    expect(grid.map((e) => e.id)).toEqual(['c'])
  })

  it('LocalizedString な name の全 locale 値が検索対象になる', () => {
    // 「JA モードで Mint と打っても、JA キーが ミント のエントリにマッチして欲しい」
    // 「JA モードで ミント と打っても、JA キーが Mint のエントリにマッチして欲しい」
    // どちらの方向にもヒットさせるため、name の全 value を haystack に連結する。
    const entries: MarketplaceEntry[] = [
      entry({ id: 'mint', name: { ja: 'ミント', en: 'Mint', default: 'EasyCursorSwap Mint' } }),
      entry({ id: 'sakura', name: { ja: '桜', en: 'Sakura' } }),
    ]
    expect(computeFilteredGrid(entries, 'all', 'Mint').map((e) => e.id)).toEqual(['mint'])
    expect(computeFilteredGrid(entries, 'all', 'ミント').map((e) => e.id)).toEqual(['mint'])
    expect(computeFilteredGrid(entries, 'all', '桜').map((e) => e.id)).toEqual(['sakura'])
    // 部分一致: "asy" は default 値の "EasyCursorSwap Mint" に含まれる
    expect(computeFilteredGrid(entries, 'all', 'asy').map((e) => e.id)).toEqual(['mint'])
  })
})
