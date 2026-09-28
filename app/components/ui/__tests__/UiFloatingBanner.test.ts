/**
 * UiFloatingBanner コンポーネントテスト (P08a Step 1)。
 *
 * tone クラス、slot 描画、dismiss emit、role 属性を確認する。
 * UiIcon は Nuxt 自動インポート対象なので stub に差し替える。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiFloatingBanner from '../UiFloatingBanner.vue'

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
}

describe('UiFloatingBanner', () => {
  it('applies tone-accent class for accent tone', () => {
    const wrapper = mount(UiFloatingBanner, {
      props: { tone: 'accent' },
      slots: { default: 'done' },
      global: { stubs },
    })
    expect(wrapper.find('.floating-banner.tone-accent').exists()).toBe(true)
  })

  it('applies tone-danger class and alert role for danger tone', () => {
    const wrapper = mount(UiFloatingBanner, {
      props: { tone: 'danger', role: 'alert' },
      slots: { default: 'failed' },
      global: { stubs },
    })
    const banner = wrapper.find('.floating-banner.tone-danger')
    expect(banner.exists()).toBe(true)
    expect(banner.attributes('role')).toBe('alert')
  })

  it('renders default and actions slots', () => {
    const wrapper = mount(UiFloatingBanner, {
      props: { tone: 'accent' },
      slots: { default: 'body text', actions: '<button>retry</button>' },
      global: { stubs },
    })
    expect(wrapper.find('.banner-body').text()).toContain('body text')
    expect(wrapper.find('.banner-actions').text()).toContain('retry')
  })

  it('emits dismiss when close button is clicked', async () => {
    const wrapper = mount(UiFloatingBanner, {
      props: { tone: 'danger' },
      slots: { default: 'x' },
      global: { stubs },
    })
    await wrapper.find('.banner-close').trigger('click')
    expect(wrapper.emitted('dismiss')).toHaveLength(1)
  })

  it('renders icon when icon prop is set', () => {
    const wrapper = mount(UiFloatingBanner, {
      props: { tone: 'accent', icon: 'Alert' },
      slots: { default: 'x' },
      global: { stubs },
    })
    // UiIcon stub が 2 件 (icon + 閉じるボタン)
    expect(wrapper.findAll('[data-testid="icon"]')).toHaveLength(2)
  })
})
