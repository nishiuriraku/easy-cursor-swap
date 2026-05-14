import { describe, it, expect, vi, beforeEach } from 'vitest'

const invokeTauriMock = vi.fn()

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: invokeTauriMock,
}))

// Blob URL モック (happy-dom には標準で URL.createObjectURL が無い)
beforeEach(() => {
  invokeTauriMock.mockReset()
  let id = 0
  global.URL.createObjectURL = vi.fn(() => `blob:fake#${++id}`)
  global.URL.revokeObjectURL = vi.fn()
})

describe('useMarketplacePreviews', () => {
  it('fetches the 6 canonical roles in parallel and returns blob URLs', async () => {
    invokeTauriMock.mockImplementation(async (_, { role }: { role: string }) => {
      return new TextEncoder().encode(`png-${role}`).buffer
    })
    const { useMarketplacePreviews } = await import('../useMarketplacePreviews')
    const { getMap } = useMarketplacePreviews()
    const map = await getMap('entry-1', 'https://example.test/previews/x')
    expect(Object.keys(map).sort()).toEqual(
      ['AppStarting', 'Arrow', 'Crosshair', 'Help', 'IBeam', 'Wait'].sort(),
    )
    expect(invokeTauriMock).toHaveBeenCalledTimes(6)
  })

  it('caches results across multiple calls with same id', async () => {
    invokeTauriMock.mockResolvedValue(new TextEncoder().encode('png').buffer)
    const { useMarketplacePreviews } = await import('../useMarketplacePreviews')
    const { getMap } = useMarketplacePreviews()
    await getMap('entry-2', 'https://example.test/previews/x')
    invokeTauriMock.mockClear()
    await getMap('entry-2', 'https://example.test/previews/x')
    expect(invokeTauriMock).not.toHaveBeenCalled()
  })

  it('invalidate() revokes blob URLs', async () => {
    invokeTauriMock.mockResolvedValue(new TextEncoder().encode('png').buffer)
    const { useMarketplacePreviews } = await import('../useMarketplacePreviews')
    const { getMap, invalidate } = useMarketplacePreviews()
    await getMap('entry-3', 'https://example.test/previews/x')
    invalidate('entry-3')
    expect(global.URL.revokeObjectURL).toHaveBeenCalled()
  })

  it('returns empty map when all fetches fail', async () => {
    invokeTauriMock.mockRejectedValue(new Error('network'))
    const { useMarketplacePreviews } = await import('../useMarketplacePreviews')
    const { getMap } = useMarketplacePreviews()
    const map = await getMap('entry-4', 'https://example.test/previews/x')
    expect(map).toEqual({})
  })
})
