/**
 * useMarketplace.ts の IPC 集約契約:
 * - loadIndex(): marketplace_fetch_index を呼んで entries を singleton state に格納
 *   (verified: true を付与 / 空レスポンス / エラーで fetchError セット)
 * - P11: IPC 戻り値は `MarketplaceIndexResult { index, stale, fetchedAt, error }`。
 *   stale:true でも entries はキャッシュ由来の一覧で埋め、stale フラグ + error を保持する。
 * - online/offline イベントで `online` を追随し、復帰時に自動再取得する。
 * - installEntry(id): marketplace_install を呼んで .cursorpack DL/検証/展開を依頼
 *   (失敗時は throw、caller の toast UI と責務分離)
 *
 * 既存の `useThemes.test.ts` と同じパターンで vi.mock + invokeTauri mock。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('../useTauri', () => ({
  invokeTauri: vi.fn(),
}))

import { invokeTauri } from '../useTauri'
import { useMarketplace } from '../useMarketplace'

const ENTRY = {
  id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
  name: 'Pixel Cats',
  author: 'alice',
  version: '1.0.0',
  downloadUrl: 'https://example.invalid/cats.cursorpack',
  sha256: '0'.repeat(64),
  signature: 'A'.repeat(128),
  authorGithub: 'alice',
  authorPubkeyId: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
  includedRoles: ['Arrow', 'Hand'],
  tags: ['cute'],
  previewBaseUrl: null,
  homepage: null,
  description: null,
  license: null,
  commitSha: null,
}

const INDEX_PAYLOAD = {
  index: {
    schema_version: 1,
    commit: 'abc1234',
    entries: [ENTRY],
  },
  stale: false,
  fetchedAt: '2026-09-28T00:00:00Z',
  error: null,
}

describe('useMarketplace', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useMarketplace().__resetForTests()
  })

  it('loadIndex() populates entries with verified:true flag added', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce(INDEX_PAYLOAD)
    const { entries, stale, fetchedAt, loadIndex } = useMarketplace()
    await loadIndex()
    expect(invokeTauri).toHaveBeenCalledWith('marketplace_fetch_index')
    expect(entries.value).toHaveLength(1)
    expect(entries.value[0]?.id).toBe(ENTRY.id)
    expect(entries.value[0]?.verified).toBe(true)
    expect(stale.value).toBe(false)
    expect(fetchedAt.value).toBe('2026-09-28T00:00:00Z')
  })

  it('loadIndex() sets fetchError and clears entries when IPC throws', async () => {
    vi.mocked(invokeTauri).mockRejectedValueOnce(new Error('network down'))
    const { entries, stale, fetchError, loadIndex } = useMarketplace()
    await loadIndex()
    expect(entries.value).toEqual([])
    expect(fetchError.value).toBe('network down')
    expect(stale.value).toBe(false)
  })

  it('loadIndex() keeps cached entries and flags stale when IPC returns stale:true', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce({
      index: { schema_version: 1, commit: 'abc1234', entries: [ENTRY] },
      stale: true,
      fetchedAt: '2026-09-27T12:00:00Z',
      error: 'index fetch failed: network down',
    })
    const { entries, stale, fetchedAt, fetchError, loadIndex } = useMarketplace()
    await loadIndex()
    expect(entries.value).toHaveLength(1)
    expect(entries.value[0]?.verified).toBe(true)
    expect(stale.value).toBe(true)
    expect(fetchedAt.value).toBe('2026-09-27T12:00:00Z')
    expect(fetchError.value).toBe('index fetch failed: network down')
  })

  it('tracks offline/online window events and reloads the index when back online', async () => {
    vi.mocked(invokeTauri).mockResolvedValue(INDEX_PAYLOAD)
    const { online, loadIndex } = useMarketplace()
    await loadIndex()
    expect(online.value).toBe(true)
    const baseCalls = vi.mocked(invokeTauri).mock.calls.length

    window.dispatchEvent(new Event('offline'))
    expect(online.value).toBe(false)

    window.dispatchEvent(new Event('online'))
    expect(online.value).toBe(true)
    // 復帰時の自動再取得で marketplace_fetch_index がもう 1 回呼ばれる。
    await new Promise((r) => setTimeout(r, 0))
    expect(
      vi.mocked(invokeTauri).mock.calls.filter((c) => c[0] === 'marketplace_fetch_index').length,
    ).toBeGreaterThan(baseCalls)
  })

  it('__resetForTests() restores the initial singleton state', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce({
      index: { schema_version: 1, commit: 'abc1234', entries: [ENTRY] },
      stale: true,
      fetchedAt: '2026-09-27T12:00:00Z',
      error: 'net down',
    })
    const api = useMarketplace()
    await api.loadIndex()
    window.dispatchEvent(new Event('offline'))
    expect(api.stale.value).toBe(true)
    expect(api.online.value).toBe(false)

    api.__resetForTests()
    expect(api.entries.value).toEqual([])
    expect(api.fetchError.value).toBeNull()
    expect(api.stale.value).toBe(false)
    expect(api.fetchedAt.value).toBeNull()
    expect(api.online.value).toBe(true)
  })

  it('installEntry(id) calls marketplace_install with the cached entry payload', async () => {
    vi.mocked(invokeTauri)
      .mockResolvedValueOnce(INDEX_PAYLOAD) // loadIndex
      .mockResolvedValueOnce(null) // marketplace_install
    const { entries, loadIndex, installEntry } = useMarketplace()
    await loadIndex()
    await installEntry(ENTRY.id)
    expect(invokeTauri).toHaveBeenCalledTimes(2)
    expect(invokeTauri).toHaveBeenLastCalledWith('marketplace_install', {
      req: {
        downloadUrl: ENTRY.downloadUrl,
        sha256: ENTRY.sha256,
        signature: ENTRY.signature,
        authorGithub: ENTRY.authorGithub,
        authorPubkeyId: ENTRY.authorPubkeyId,
      },
    })
    expect(entries.value).toHaveLength(1)
  })

  it('installEntry(id) throws when marketplace_install IPC rejects (caller handles toast)', async () => {
    vi.mocked(invokeTauri)
      .mockResolvedValueOnce(INDEX_PAYLOAD) // loadIndex
      .mockRejectedValueOnce(new Error('signature invalid')) // marketplace_install
    const { loadIndex, installEntry } = useMarketplace()
    await loadIndex()
    await expect(installEntry(ENTRY.id)).rejects.toThrow('signature invalid')
  })
})
