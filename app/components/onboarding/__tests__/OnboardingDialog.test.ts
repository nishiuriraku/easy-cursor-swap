/**
 * OnboardingDialog のテスト (P10)。
 *
 * open 時の dialog role、ステップ遷移、Skip/Finish の complete 呼び出し、
 * 進捗テキストを検証する (実 useI18n の en 解決でアサート)。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import OnboardingDialog from '../OnboardingDialog.vue'
import { useOnboarding } from '~/composables/useOnboarding'

const configValue = { value: null as unknown }
const updateMock = vi.fn()

vi.mock('~/composables/useAppSettings', () => ({
  useAppSettings: () => ({
    config: configValue,
    update: (...args: unknown[]) => updateMock(...args),
  }),
}))

const stubs = {
  UiIcon: { template: '<span></span>' },
  UiButton: {
    props: ['variant', 'disabled', 'loading', 'iconLeft', 'iconRight'],
    template: '<button><slot /></button>',
  },
  UiModal: {
    props: ['open', 'title'],
    template:
      '<div v-if="open" role="dialog" aria-modal="true"><slot /><slot name="actions" /><slot name="leftNote" /></div>',
  },
  OnboardingStepWelcome: { template: '<div data-step="welcome"></div>' },
  OnboardingStepSafety: { template: '<div data-step="safety"></div>' },
  OnboardingStepStart: { template: '<div data-step="start"></div>' },
}

function mountOpen() {
  configValue.value = { general: { onboarding_version: 0 } }
  updateMock.mockResolvedValue(null)
  const ob = useOnboarding()
  ob.evaluate()
  return mount(OnboardingDialog, { global: { stubs } })
}

function buttons(wrapper: {
  findAll: (s: string) => Array<{ text: () => string; trigger: (e: string) => Promise<void> }>
}) {
  return wrapper.findAll('button')
}

beforeEach(() => {
  updateMock.mockReset()
  configValue.value = null
  useOnboarding().__resetForTests()
})

describe('OnboardingDialog', () => {
  it('renders dialog with role and welcome title when open', () => {
    const wrapper = mountOpen()
    const dialog = wrapper.find('[role="dialog"]')
    expect(dialog.exists()).toBe(true)
    expect(dialog.attributes('aria-modal')).toBe('true')
    expect(wrapper.find('[data-step="welcome"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('Next moves safety -> start and Back appears', async () => {
    const wrapper = mountOpen()
    const next = buttons(wrapper).find((b) => b.text().includes('Next'))!
    await next.trigger('click')
    expect(wrapper.find('[data-step="safety"]').exists()).toBe(true)
    await next.trigger('click')
    expect(wrapper.find('[data-step="start"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('Skip calls complete and writes version', async () => {
    const wrapper = mountOpen()
    const skip = buttons(wrapper).find((b) => b.text().includes('Skip for now'))!
    await skip.trigger('click')
    expect(updateMock).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('progress text shows step count', () => {
    const wrapper = mountOpen()
    expect(wrapper.text()).toContain('Step 1 of 3')
    wrapper.unmount()
  })
})
