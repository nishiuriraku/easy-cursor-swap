/**
 * index.helpers のフィルタ・ソート純関数テスト (P08a Step 4 / L1)。
 *
 * 既存 `index.source-mapping.test.ts` の隣で、5 sortKey × 2 dir、
 * フィルタ、scheme→card 変換を直接テストする。
 */
import { describe, expect, it } from 'vitest'
import type { ThemeCardData } from '~/types/theme'
import {
  countThemes,
  filterThemes,
  mapWindowsSchemeToCard,
  sortThemes,
} from '../index.helpers'

function card(partial: Partial<ThemeCardData> & { id: string; name: string }): ThemeCardData {
  return {
    author: null,
    version: '1.0',
    date: '2026-01-01',
    applyCount: 0,
    isFavorite: false,
    isActive: false,
    includedRoles: [],
    kind: 'local',
    tags: [],
    sizeBytes: undefined,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: undefined,
    license: null,
    homepage: null,
    ...partial,
  }
}

const A = () =>
  card({ id: 'a', name: 'Beta', applyCount: 3, includedRoles: ['Arrow'], sizeBytes: 10 })
const B = () =>
  card({
    id: 'b',
    name: 'alpha',
    applyCount: 9,
    includedRoles: ['Arrow', 'Help'],
    sizeBytes: 30,
    isFavorite: true,
    lastAppliedAt: '2026-02-01',
    date: '2026-02-01',
  })

describe('filterThemes', () => {
  it('matches name and author case-insensitively', () => {
    const withAuthor = card({ id: 'c', name: 'zzz', author: 'Nishi' })
    expect(filterThemes([A(), B(), withAuthor], { query: 'nish', filter: 'all' })).toHaveLength(1)
    expect(filterThemes([A(), B()], { query: 'ALP', filter: 'all' })[0]!.id).toBe('b')
  })

  it('filters favorites and recent', () => {
    expect(filterThemes([A(), B()], { query: '', filter: 'favorites' }).map((t) => t.id)).toEqual([
      'b',
    ])
    // applyCount > 0 も recent 扱いのため両方
    expect(
      filterThemes([A(), B()], { query: '', filter: 'recent' }).map((t) => t.id).sort(),
    ).toEqual(['a', 'b'])
  })
})

describe('sortThemes', () => {
  it.each([
    ['name', 'asc', ['b', 'a']],
    ['name', 'desc', ['a', 'b']],
    ['applied', 'asc', ['a', 'b']],
    ['applied', 'desc', ['b', 'a']],
    ['coverage', 'asc', ['a', 'b']],
    ['size', 'desc', ['b', 'a']],
  ] as const)('sorts by %s %s', (key, dir, expected) => {
    expect(sortThemes([A(), B()], key, dir).map((t) => t.id)).toEqual([...expected])
  })
})

describe('countThemes', () => {
  it('counts all/favorites/recent', () => {
    expect(countThemes([A(), B()])).toEqual({ all: 2, favorites: 1, recent: 2 })
  })
})

describe('mapWindowsSchemeToCard', () => {
  it('maps scheme with windows: prefix and system kind', () => {
    const c = mapWindowsSchemeToCard(
      { name: 'Default', cursor_paths: { Arrow: 'C:\\a.cur', Help: '' }, role_count: 2 },
      'Windows',
    )
    expect(c.id).toBe('windows:Default')
    expect(c.kind).toBe('system')
    expect(c.author).toBe('Windows')
    expect(c.includedRoles).toEqual(['Arrow'])
    expect(c.signed).toBe(false)
  })

  it('uses the given author label', () => {
    const c = mapWindowsSchemeToCard({ name: 'X', cursor_paths: {}, role_count: 0 }, 'OS')
    expect(c.author).toBe('OS')
  })
})
