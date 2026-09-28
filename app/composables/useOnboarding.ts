/**
 * 初回起動オンボーディングの表示判定と完了フラグ書込 (P10)。
 * 真実は Rust `AppConfig.general.onboarding_version` (`config.rs::ONBOARDING_CURRENT_VERSION`)。
 * シングルトン: layouts/default.vue が `evaluate()` を呼び、`OnboardingDialog` が `open` を購読する。
 */
import type { AppConfig } from '~/types/config'

/** Rust `config.rs::ONBOARDING_CURRENT_VERSION` と同値に保つ (テストで固定)。 */
export const ONBOARDING_VERSION = 1
export type OnboardingStep = 'welcome' | 'safety' | 'start'
export const ONBOARDING_STEPS: readonly OnboardingStep[] = ['welcome', 'safety', 'start']

const open = ref(false)
const step = ref<OnboardingStep>('welcome')
const saving = ref(false)
/** IPC 書込に失敗しても同セッションで再表示しないためのメモリフラグ (次回起動で Rust が真実)。 */
let dismissedThisSession = false

export function shouldShowOnboarding(config: AppConfig | null): boolean {
  if (!config) return false
  return (config.general.onboarding_version ?? 0) < ONBOARDING_VERSION
}

export function useOnboarding() {
  const settings = useAppSettings()

  /** config ロード後に 1 回呼ぶ。条件を満たせば open=true。 */
  function evaluate(): void {
    if (dismissedThisSession) return
    if (shouldShowOnboarding(settings.config.value)) {
      step.value = 'welcome'
      open.value = true
    }
  }
  function next(): void {
    const i = ONBOARDING_STEPS.indexOf(step.value)
    if (i < ONBOARDING_STEPS.length - 1) step.value = ONBOARDING_STEPS[i + 1]!
    else void complete()
  }
  function back(): void {
    const i = ONBOARDING_STEPS.indexOf(step.value)
    if (i > 0) step.value = ONBOARDING_STEPS[i - 1]!
  }
  /** 完了 / スキップ / Esc 共通。Rust へ version を書き、閉じる。 */
  async function complete(): Promise<void> {
    dismissedThisSession = true
    open.value = false
    saving.value = true
    try {
      await settings.update((c) => {
        c.general.onboarding_version = ONBOARDING_VERSION
      })
    } finally {
      saving.value = false
    }
  }
  /** 設定 → 一般 → 「もう一度見る」。0 に戻して即表示。 */
  async function replay(): Promise<void> {
    await settings.update((c) => {
      c.general.onboarding_version = 0
    })
    dismissedThisSession = false
    step.value = 'welcome'
    open.value = true
  }
  return {
    open: readonly(open),
    step: readonly(step),
    saving: readonly(saving),
    evaluate,
    next,
    back,
    complete,
    replay,
    __resetForTests,
  }
}
function __resetForTests() {
  open.value = false
  step.value = 'welcome'
  saving.value = false
  dismissedThisSession = false
}
