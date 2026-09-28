/**
 * OnboardingStepStart のテスト (P10)。
 *
 * 3 ボタンが期待引数で navigateTo を呼び、`actionTaken` を emit することを検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import OnboardingStepStart from '../OnboardingStepStart.vue'

const navigateToMock = vi.fn()

const stubs = {
  UiIcon: { template: '<span></span>' },
}

beforeEach(() => {
  navigateToMock.mockReset()
  vi.stubGlobal('navigateTo', navigateToMock)
})

describe('OnboardingStepStart', () => {
  it('renders 3 CTA buttons', () => {
    const wrapper = mount(OnboardingStepStart, { global: { stubs } })
    expect(wrapper.findAll('.ob-card')).toHaveLength(3)
    wrapper.unmount()
  })

  it('import CTA navigates with openImport query and emits actionTaken', async () => {
    const wrapper = mount(OnboardingStepStart, { global: { stubs } })
    await wrapper.findAll('.ob-card')[0]!.trigger('click')
    expect(navigateToMock).toHaveBeenCalledWith({ path: '/', query: { openImport: '1' } })
    expect(wrapper.emitted('actionTaken')).toHaveLength(1)
    wrapper.unmount()
  })

  it('index and creator CTAs navigate to pages', async () => {
    const wrapper = mount(OnboardingStepStart, { global: { stubs } })
    await wrapper.findAll('.ob-card')[1]!.trigger('click')
    expect(navigateToMock).toHaveBeenCalledWith('/marketplace')
    await wrapper.findAll('.ob-card')[2]!.trigger('click')
    expect(navigateToMock).toHaveBeenCalledWith('/creator')
    wrapper.unmount()
  })
})
