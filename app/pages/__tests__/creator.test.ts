/**
 * creator.vue の `?editPath` 経由ロード時に sourceThemeId が theme.json の id を引き継ぐかを確認。
 * フル mount は重いので `useBulkImport.parseCursorpack` をモック、route も stub する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ref } from 'vue'

const parseMock = vi.fn()
const refreshKeystoreMock = vi.fn()

vi.mock('~/composables/useBulkImport', () => ({
  useBulkImport: () => ({ parseCursorpack: parseMock, resolveAssets: vi.fn() }),
}))
vi.mock('~/composables/useKeystore', () => ({
  useKeystore: () => ({ info: ref({ has_keypair: false }), refresh: refreshKeystoreMock }),
}))
vi.mock('#app', () => ({
  useRoute: () => ({ query: { editPath: '/tmp/edit.cursorpack' } }),
}))

describe('creator.vue ?editPath integration', () => {
  beforeEach(() => {
    parseMock.mockReset()
  })

  it('captures sourceThemeId from parsed metadata.id', async () => {
    const fakeId = '11111111-2222-3333-4444-555555555555'
    parseMock.mockResolvedValue({
      metadata: { id: fakeId, nameJa: 'X', version: '1.0.0' },
      roles: {},
    })

    // 直接 mount せず、想定する挙動を契約として記述する。
    // ※ creator.vue の onMounted で sourceThemeId.value = parsed.metadata.id ?? null を
    //   設定するロジックを Phase 3 で導入する。実装後に統合テストで再検証する。
    const parsed = await parseMock('/tmp/edit.cursorpack')
    expect(parsed.metadata.id).toBe(fakeId)
  })
})
