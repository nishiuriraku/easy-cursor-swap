import { describe, it, expect } from 'vitest'
import { mapLocalSummaryToCard, type IpcThemeSummary } from '~/pages/index.helpers'

function fixture(overrides: Partial<IpcThemeSummary> = {}): IpcThemeSummary {
  return {
    id: '6d364941-c605-4def-801a-14ebb401936f',
    name: 'Mint',
    author: 'nishiuriraku',
    version: '1.0.0',
    created_at: '2026-05-14T00:00:00Z',
    is_active: false,
    is_favorite: false,
    apply_count: 0,
    included_roles: ['Arrow'],
    path: '/tmp/x',
    tags: ['minimal'],
    size_bytes: 1024,
    signed: true,
    last_applied_at: null,
    schema_version: 1,
    ...overrides,
  }
}

describe('mapLocalSummaryToCard', () => {
  it('source: "marketplace" → kind: "marketplace"', () => {
    expect(mapLocalSummaryToCard(fixture({ source: 'marketplace' })).kind).toBe('marketplace')
  })

  it('source: "local" → kind: "local"', () => {
    expect(mapLocalSummaryToCard(fixture({ source: 'local' })).kind).toBe('local')
  })

  it('source 欠落 (旧スキーマ) → kind: "local"', () => {
    expect(mapLocalSummaryToCard(fixture()).kind).toBe('local')
  })

  it('未知の source 値 → kind: "local" (forward-compat)', () => {
    expect(mapLocalSummaryToCard(fixture({ source: 'future_value' })).kind).toBe('local')
  })

  it('description / license / homepage が undefined のとき null フォールバック', () => {
    const card = mapLocalSummaryToCard(fixture({ description: undefined }))
    expect(card.description).toBeNull()
    expect(card.license).toBeNull()
    expect(card.homepage).toBeNull()
  })

  it('description / license / homepage が明示的 null のとき null を保つ', () => {
    // Rust から JSON null が来るケース。?? null で吸収される。
    const card = mapLocalSummaryToCard(
      fixture({ description: null, license: null, homepage: null }),
    )
    expect(card.description).toBeNull()
    expect(card.license).toBeNull()
    expect(card.homepage).toBeNull()
  })
})
