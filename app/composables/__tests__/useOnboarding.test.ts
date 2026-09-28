/**
 * useOnboarding のテスト (P10 Step 4)。
 *
 * 表示判定・遷移境界・complete/replay の書込を検証する。
 * `useAppSettings` をモックし、`__resetForTests` で singleton を初期化する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

const configValue = { value: null as unknown }
const updateMock = vi.fn()

vi.mock('../useAppSettings', () => ({
  useAppSettings: () => ({
    config: configValue,
    update: (...args: unknown[]) => updateMock(...args),
  }),
}))

import { ONBOARDING_VERSION, shouldShowOnboarding, useOnboarding } from '../useOnboarding'

const cfg = (v: number | undefined) => ({
  general: { onboarding_version: v },
})

beforeEach(() => {
  updateMock.mockReset()
  configValue.value = null
  useOnboarding().__resetForTests()
})

describe('shouldShowOnboarding', () => {
  it('null config shows nothing', () => {
    expect(shouldShowOnboarding(null)).toBe(false)
  })
  it('version 0 shows, 1+ hides', () => {
    expect(shouldShowOnboarding(cfg(0) as never)).toBe(true)
    expect(shouldShowOnboarding(cfg(1) as never)).toBe(false)
    expect(shouldShowOnboarding(cfg(99) as never)).toBe(false)
  })
  it('ONBOARDING_VERSION matches Rust constant', () => {
    expect(ONBOARDING_VERSION).toBe(1)
  })
})

describe('useOnboarding flow', () => {
  it('evaluate() opens on fresh config', () => {
    configValue.value = cfg(0)
    const { open, evaluate } = useOnboarding()
    evaluate()
    expect(open.value).toBe(true)
  })

  it('next/back respect boundaries (back no-op at head, next completes at tail)', async () => {
    updateMock.mockResolvedValueOnce(null)
    configValue.value = cfg(0)
    const ob = useOnboarding()
    ob.evaluate()
    ob.back()
    expect(ob.step.value).toBe('welcome')
    ob.next()
    expect(ob.step.value).toBe('safety')
    ob.next()
    expect(ob.step.value).toBe('start')
    await ob.next()
    expect(ob.open.value).toBe(false)
  })

  it('complete() writes version and stays closed on failure', async () => {
    updateMock.mockRejectedValueOnce(new Error('ipc down'))
    configValue.value = cfg(0)
    const ob = useOnboarding()
    ob.evaluate()
    await expect(ob.complete()).rejects.toThrow('ipc down')
    expect(updateMock).toHaveBeenCalledTimes(1)
    expect(ob.open.value).toBe(false)
    // 失敗しても同セッションでは再表示しない
    ob.evaluate()
    expect(ob.open.value).toBe(false)
  })

  it('replay() writes 0 and opens immediately', async () => {
    updateMock.mockResolvedValueOnce(null)
    configValue.value = cfg(1)
    const ob = useOnboarding()
    await ob.replay()
    expect(ob.open.value).toBe(true)
    expect(ob.step.value).toBe('welcome')
  })
})
