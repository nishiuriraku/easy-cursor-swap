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
  schema_version: 1,
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
    expect(t.schemaVersion).toBe(1)
    expect(t.license).toBe('MIT')
    expect(t.homepage).toBe('https://example.com')
    expect(t.lastAppliedAt).toBe('2026-05-10T00:00:00Z')
  })

  // ── Wave 2AB / Task 13: clonedFromMarketplaceId lineage contract ──
  // マーケットプレイス由来テーマの複製が系譜を保持することを UI 側でも観測できる
  // ことを確認する (= 再提出ガードの UI 表示側の契約)。
  // Rust 側で Some(origin_id) を返したら、Vue 側 ThemeCardData は clonedFromMarketplaceId
  // に string で持つ。None / null は null に正規化される。

  it('clonedFromMarketplaceId を string → string にそのまま中継する', async () => {
    const originId = '22222222-2222-4222-8222-222222222222'
    vi.mocked(invokeTauri).mockResolvedValueOnce([
      { ...SUMMARY, cloned_from_marketplace_id: originId },
    ])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value[0]!.clonedFromMarketplaceId).toBe(originId)
  })

  it('cloned_from_marketplace_id が null → clonedFromMarketplaceId は null', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([
      { ...SUMMARY, cloned_from_marketplace_id: null },
    ])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value[0]!.clonedFromMarketplaceId).toBeNull()
  })

  it('cloned_from_marketplace_id フィールド欠落 → null に正規化', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([
      // SUMMARY から cloned_from_marketplace_id を意図的に省く
      {
        id: SUMMARY.id,
        name: SUMMARY.name,
        author: SUMMARY.author,
        version: SUMMARY.version,
        created_at: SUMMARY.created_at,
        is_active: SUMMARY.is_active,
        is_favorite: SUMMARY.is_favorite,
        apply_count: SUMMARY.apply_count,
        included_roles: SUMMARY.included_roles,
        path: SUMMARY.path,
        tags: SUMMARY.tags,
        size_bytes: SUMMARY.size_bytes,
        signed: SUMMARY.signed,
        description: SUMMARY.description,
        schema_version: SUMMARY.schema_version,
        license: SUMMARY.license,
        homepage: SUMMARY.homepage,
        last_applied_at: SUMMARY.last_applied_at,
      },
    ])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value[0]!.clonedFromMarketplaceId).toBeNull()
  })

  it('source: "marketplace" を kind: "marketplace" にマップする (lineage 表示用)', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([
      { ...SUMMARY, source: 'marketplace', cloned_from_marketplace_id: 'origin-uuid' },
    ])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value[0]!.kind).toBe('marketplace')
    expect(themes.value[0]!.clonedFromMarketplaceId).toBe('origin-uuid')
  })

  it('source フィールド欠落 / 未知の値 → kind: "local" にマップ', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce([
      { ...SUMMARY, source: 'unknown-source-value' },
    ])
    const { refresh, themes } = useThemes()
    await refresh()
    expect(themes.value[0]!.kind).toBe('local')
  })
})
