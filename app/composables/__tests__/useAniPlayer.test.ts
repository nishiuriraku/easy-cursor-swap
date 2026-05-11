import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { effectScope, nextTick } from 'vue'
import { useAniPlayer } from '~/composables/useAniPlayer'

function makeFrames(n: number): Uint8Array[] {
  return Array.from({ length: n }, (_, i) => new Uint8Array([i]))
}

describe('useAniPlayer', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('loops indefinitely after final step', () => {
    const scope = effectScope()
    scope.run(() => {
      const p = useAniPlayer({
        framePngs: makeFrames(3),
        sequence: [0, 1, 2],
        perStepDurationsMs: [100, 100, 100],
      })
      expect(p.currentStep.value).toBe(0)
      vi.advanceTimersByTime(100)
      expect(p.currentStep.value).toBe(1)
      vi.advanceTimersByTime(100)
      expect(p.currentStep.value).toBe(2)
      vi.advanceTimersByTime(100)
      expect(p.currentStep.value).toBe(0)
      vi.advanceTimersByTime(100)
      expect(p.currentStep.value).toBe(1)
    })
    scope.stop()
  })

  it('respects per-step durations', () => {
    const scope = effectScope()
    scope.run(() => {
      const p = useAniPlayer({
        framePngs: makeFrames(3),
        sequence: [0, 1, 2],
        perStepDurationsMs: [100, 300, 200],
      })
      vi.advanceTimersByTime(99)
      expect(p.currentStep.value).toBe(0)
      vi.advanceTimersByTime(1)
      expect(p.currentStep.value).toBe(1)
      vi.advanceTimersByTime(299)
      expect(p.currentStep.value).toBe(1)
      vi.advanceTimersByTime(1)
      expect(p.currentStep.value).toBe(2)
    })
    scope.stop()
  })

  it('pauses when document becomes hidden', async () => {
    const scope = effectScope()
    let p!: ReturnType<typeof useAniPlayer>
    scope.run(() => {
      p = useAniPlayer({
        framePngs: makeFrames(2),
        sequence: [0, 1],
        perStepDurationsMs: [100, 100],
      })
    })

    vi.advanceTimersByTime(100)
    expect(p.currentStep.value).toBe(1)

    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => 'hidden',
    })
    document.dispatchEvent(new Event('visibilitychange'))
    await nextTick()

    const step = p.currentStep.value
    vi.advanceTimersByTime(500)
    expect(p.currentStep.value).toBe(step)

    scope.stop()
  })

  it('resumes when document becomes visible', async () => {
    const scope = effectScope()
    let p!: ReturnType<typeof useAniPlayer>
    scope.run(() => {
      p = useAniPlayer({
        framePngs: makeFrames(2),
        sequence: [0, 1],
        perStepDurationsMs: [100, 100],
      })
    })

    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => 'hidden',
    })
    document.dispatchEvent(new Event('visibilitychange'))
    await nextTick()
    vi.advanceTimersByTime(500)
    const stepBefore = p.currentStep.value

    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      get: () => 'visible',
    })
    document.dispatchEvent(new Event('visibilitychange'))
    await nextTick()
    vi.advanceTimersByTime(100)
    expect(p.currentStep.value).not.toBe(stepBefore)

    scope.stop()
  })

  it('revokes blob URLs on scope dispose', () => {
    const spy = vi.spyOn(URL, 'revokeObjectURL')
    const scope = effectScope()
    scope.run(() => {
      useAniPlayer({
        framePngs: makeFrames(2),
        sequence: [0, 1],
        perStepDurationsMs: [100, 100],
      })
    })
    scope.stop()
    expect(spy).toHaveBeenCalledTimes(2)
  })
})
