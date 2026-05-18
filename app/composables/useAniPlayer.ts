/**
 * .ani アニメーション再生エンジン。
 *
 * 入力: `framePngs` (各フレームの PNG バイト)、`sequence` (再生順 = フレームインデックス列)、
 * `perStepDurationsMs` (各ステップの表示時間 ms)。
 *
 * 動作:
 *   - マウント時に各フレームを `URL.createObjectURL` で blob URL に変換 (キャッシュ)
 *   - chained setTimeout で step を進める。最終 step → 0 に戻って無限ループ
 *   - document.visibilityState='hidden' 中は setTimeout を抑止
 *   - onScopeDispose で blob URL を revoke
 *
 * `prefers-reduced-motion` は仕様により尊重しない (ユーザー要望: ずっと動いてる)。
 */
import type { ComputedRef, Ref } from 'vue'

export interface UseAniPlayerInput {
  framePngs: readonly Uint8Array[]
  sequence: readonly number[]
  perStepDurationsMs: readonly number[]
}

export interface UseAniPlayerReturn {
  currentImageUrl: ComputedRef<string>
  currentStep: Ref<number>
  isPlaying: Ref<boolean>
  play: () => void
  pause: () => void
  reset: () => void
}

export function useAniPlayer(input: UseAniPlayerInput): UseAniPlayerReturn {
  const frameUrls: string[] = input.framePngs.map((bytes) =>
    URL.createObjectURL(new Blob([bytes], { type: 'image/png' })),
  )

  const currentStep = ref(0)
  const isPlaying = ref(true)
  const documentHidden = ref(
    typeof document !== 'undefined' && document.visibilityState === 'hidden',
  )

  let timer: ReturnType<typeof setTimeout> | null = null

  function clearTimer() {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  function scheduleNext() {
    clearTimer()
    if (!isPlaying.value || documentHidden.value) return
    if (input.sequence.length === 0) return
    const stepIdx = currentStep.value
    const durations = input.perStepDurationsMs
    const ms = durations.length > 0 ? durations[stepIdx % durations.length] : 100
    timer = setTimeout(
      () => {
        currentStep.value = (currentStep.value + 1) % input.sequence.length
        scheduleNext()
      },
      Math.max(1, ms),
    )
  }

  function play() {
    if (isPlaying.value) return
    isPlaying.value = true
    scheduleNext()
  }

  function pause() {
    if (!isPlaying.value) return
    isPlaying.value = false
    clearTimer()
  }

  function reset() {
    clearTimer()
    currentStep.value = 0
    if (isPlaying.value) scheduleNext()
  }

  function onVisibilityChange() {
    if (typeof document === 'undefined') return
    documentHidden.value = document.visibilityState === 'hidden'
    if (documentHidden.value) {
      clearTimer()
    } else if (isPlaying.value) {
      scheduleNext()
    }
  }

  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', onVisibilityChange)
  }

  scheduleNext()

  onScopeDispose(() => {
    clearTimer()
    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }
    for (const url of frameUrls) URL.revokeObjectURL(url)
  })

  const currentImageUrl = computed(() => {
    const seq = input.sequence
    if (seq.length === 0) return frameUrls[0] ?? ''
    const frameIdx = seq[currentStep.value % seq.length] ?? 0
    return frameUrls[frameIdx] ?? frameUrls[0] ?? ''
  })

  return { currentImageUrl, currentStep, isPlaying, play, pause, reset }
}
