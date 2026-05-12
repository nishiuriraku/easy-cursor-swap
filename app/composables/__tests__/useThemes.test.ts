/**
 * useThemes.ts の IpcThemeSummary → ThemeCardData マッパーが
 * Rust 側 ThemeSummary のフィールドを取りこぼさないことを固定化する。
 *
 * 直接 internal の mapSummary を export していないので、mock した
 * invokeTauri 経由で refresh() を回し、themes ref を観測する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('../useTauri', () => ({
  invokeTauri: vi.fn(),
}))

import { invokeTauri } from '../useTauri'
import { useThemes } from '../useThemes'

const SUMMARY = {
  id: '11111111-1111-4111-8111-111111111111',
  name: 'Sample',
  author: 'alice',
  version: '1.0.0',
  created_at: '2026-05-01T00:00:00Z',
  is_active: false,
  is_favorite: false,
  apply_count: 3,
  included_roles: ['Arrow'],
  path: '/themes/sample',
  tags: ['cute'],
  size_bytes: 1234,
  signed: false,
  description: 'Sample description.',
  schema_version: 2,
  license: 'MIT',
  homepage: 'https://example.com',
  last_applied_at: '2026-05-10T00:00:00Z',
}

describe('useThemes mapSummary', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('description と signed を ThemeCardData に中継する', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([SUMMARY])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value).toHaveLength(1)
    expect(themes.value[0]!.description).toBe('Sample description.')
    expect(themes.value[0]!.signed).toBe(false)
  })

  it('tags / sizeBytes / schemaVersion / license / homepage / lastAppliedAt を中継する', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([SUMMARY])
    const { refresh, themes } = useThemes()
    await refresh()
    const t = themes.value[0]!
    expect(t.tags).toEqual(['cute'])
    expect(t.sizeBytes).toBe(1234)
    expect(t.schemaVersion).toBe(2)
    expect(t.license).toBe('MIT')
    expect(t.homepage).toBe('https://example.com')
    expect(t.lastAppliedAt).toBe('2026-05-10T00:00:00Z')
  })
})
