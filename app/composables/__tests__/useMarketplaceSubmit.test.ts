import { describe, it, expect, vi, beforeEach } from 'vitest'

const invokeMock = vi.fn()
const listenMock = vi.fn()

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (...a: unknown[]) => invokeMock(...a),
  listenTauri: (...a: unknown[]) => listenMock(...a),
}))

import { useMarketplaceSubmit } from '../useMarketplaceSubmit'

describe('useMarketplaceSubmit', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    listenMock.mockReset()
  })

  it('submit() invokes submit_theme_auto with themeId and returns SubmitResult', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValueOnce({
      prUrl: 'https://github.com/x/y/pull/1',
      prNumber: 1,
    })

    const sub = useMarketplaceSubmit()
    const result = await sub.submit('abc-123')

    expect(invokeMock).toHaveBeenCalledWith('submit_theme_auto', { themeId: 'abc-123', tags: [] })
    expect(result.prUrl).toBe('https://github.com/x/y/pull/1')
    expect(result.prNumber).toBe(1)
  })

  it('listens to submit:progress and updates stage as events fire', async () => {
    let cb: ((e: { payload: string }) => void) | null = null
    listenMock.mockImplementation(async (_evt: string, fn: typeof cb) => {
      cb = fn
      return () => {}
    })
    invokeMock.mockImplementationOnce(async () => {
      // Simulate progress events during the IPC call
      cb?.({ payload: 'build' })
      cb?.({ payload: 'fork' })
      cb?.({ payload: 'open_pr' })
      return { prUrl: 'u', prNumber: 1 }
    })

    const sub = useMarketplaceSubmit()
    await sub.submit('abc')
    expect(sub.stage.value).toBe('open_pr')
  })

  it('sets busy true during submit and false after', async () => {
    listenMock.mockResolvedValue(() => {})
    let resolveInvoke!: (v: unknown) => void
    invokeMock.mockReturnValueOnce(
      new Promise((res) => {
        resolveInvoke = res
      }),
    )

    const sub = useMarketplaceSubmit()
    const promise = sub.submit('abc')
    // Allow microtask queue to settle so busy is set
    await Promise.resolve()
    expect(sub.busy.value).toBe(true)

    resolveInvoke({ prUrl: 'u', prNumber: 1 })
    await promise
    expect(sub.busy.value).toBe(false)
  })

  it('errorMsg is set when invoke throws', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockRejectedValueOnce(new Error('boom'))

    const sub = useMarketplaceSubmit()
    await expect(sub.submit('abc')).rejects.toThrow('boom')
    expect(sub.errorMsg.value).toContain('boom')
    expect(sub.busy.value).toBe(false)
  })
})
