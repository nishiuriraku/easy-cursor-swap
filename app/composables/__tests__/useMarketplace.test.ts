/**
 * useMarketplace.ts の IPC 集約契約:
 * - loadIndex(): marketplace_fetch_index を呼んで entries を singleton state に格納
 *   (verified: true を付与 / 空レスポンス / エラーで fetchError セット)
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
  schema_version: 1,
  commit: 'abc1234',
  entries: [ENTRY],
}

describe('useMarketplace', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('loadIndex() populates entries with verified:true flag added', async () => {
    vi.mocked(invokeTauri).mockResolvedValueOnce(INDEX_PAYLOAD)
    const { entries, loadIndex } = useMarketplace()
    await loadIndex()
    expect(invokeTauri).toHaveBeenCalledWith('marketplace_fetch_index')
    expect(entries.value).toHaveLength(1)
    expect(entries.value[0]?.id).toBe(ENTRY.id)
    expect(entries.value[0]?.verified).toBe(true)
  })

  it('loadIndex() sets fetchError and clears entries when IPC throws', async () => {
    vi.mocked(invokeTauri).mockRejectedValueOnce(new Error('network down'))
    const { entries, fetchError, loadIndex } = useMarketplace()
    await loadIndex()
    expect(entries.value).toEqual([])
    expect(fetchError.value).toBe('network down')
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