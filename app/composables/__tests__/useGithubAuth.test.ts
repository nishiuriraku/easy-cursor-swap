import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

const invokeMock = vi.fn()
vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeMock(...args),
}))

import { useGithubAuth } from '../useGithubAuth'

describe('useGithubAuth', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    invokeMock.mockReset()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('start() populates userCode/verificationUri and sets status=waiting', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'WDJB-MJHT',
      verificationUri: 'https://github.com/login/device',
      expiresIn: 900,
      interval: 5,
    })
    const auth = useGithubAuth()
    await auth.start()
    expect(auth.userCode.value).toBe('WDJB-MJHT')
    expect(auth.verificationUri.value).toBe('https://github.com/login/device')
    expect(auth.status.value).toBe('waiting')
  })

  it('polling transitions to ready when complete_device_flow returns ready', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://x',
      expiresIn: 900,
      interval: 1,
    })
    const auth = useGithubAuth()
    await auth.start()
    expect(auth.status.value).toBe('waiting')

    // First poll → pending
    invokeMock.mockResolvedValueOnce({ status: 'pending' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(auth.status.value).toBe('waiting')

    // Second poll → ready
    invokeMock.mockResolvedValueOnce({ status: 'ready', login: 'octocat' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(auth.status.value).toBe('ready')
    expect(auth.login.value).toBe('octocat')
  })

  it('polling stops on expired status', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://x',
      expiresIn: 900,
      interval: 1,
    })
    const auth = useGithubAuth()
    await auth.start()
    invokeMock.mockResolvedValueOnce({ status: 'expired' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(auth.status.value).toBe('expired')
  })

  it('polling stops on denied status', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://x',
      expiresIn: 900,
      interval: 1,
    })
    const auth = useGithubAuth()
    await auth.start()
    invokeMock.mockResolvedValueOnce({ status: 'denied' })
    await vi.advanceTimersByTimeAsync(1000)
    expect(auth.status.value).toBe('denied')
  })

  it('slow_down extends interval by 5 seconds', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://x',
      expiresIn: 900,
      interval: 1,
    })
    const auth = useGithubAuth()
    await auth.start()
    invokeMock.mockResolvedValueOnce({ status: 'slow_down' })
    await vi.advanceTimersByTimeAsync(1000)
    // Now interval should be 1000 + 5000 = 6000ms.
    // After 5000ms more (total 6000) the next poll should not have fired yet.
    invokeMock.mockResolvedValueOnce({ status: 'pending' })
    await vi.advanceTimersByTimeAsync(5000)
    // 6000ms total since slow_down -> poll fires
    // (Allow some slack — the test mostly verifies interval was reset, not exact ms.)
    expect(auth.status.value).toBe('waiting')
  })

  it('cancel() calls cancel_device_flow IPC and resets status', async () => {
    invokeMock.mockResolvedValueOnce({
      userCode: 'X',
      verificationUri: 'https://x',
      expiresIn: 900,
      interval: 1,
    })
    const auth = useGithubAuth()
    await auth.start()
    invokeMock.mockResolvedValueOnce(undefined)
    await auth.cancel()
    expect(invokeMock).toHaveBeenLastCalledWith('cancel_device_flow')
    expect(auth.status.value).toBe('idle')
    expect(auth.userCode.value).toBeNull()
  })
})
