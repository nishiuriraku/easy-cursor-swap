/**
 * LibraryDropOverlay コンポーネントテスト。
 *
 * ドラッグ中のオーバーレイ表示制御を確認する。
 * UiIcon は Nuxt 自動インポート対象なので stub に差し替える。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibraryDropOverlay from '../LibraryDropOverlay.vue'

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  Transition: { template: '<div><slot /></div>' },
}

describe('LibraryDropOverlay', () => {
  it('renders nothing when show=false', () => {
    const wrapper = mount(LibraryDropOverlay, {
      props: { show: false },
      global: { stubs },
    })
    // Transition の中身 v-if=false なので drop クラスは存在しない
    expect(wrapper.find('.drop').exists()).toBe(false)
  })

  it('renders overlay when show=true', () => {
    const wrapper = mount(LibraryDropOverlay, {
      props: { show: true },
      global: { stubs },
    })
    expect(wrapper.find('.drop').exists()).toBe(true)
  })

  it('shows i18n title and sub text from library namespace', () => {
    const wrapper = mount(LibraryDropOverlay, {
      props: { show: true },
      global: { stubs },
    })
    // ja resource の値: '.cursorpack をドロップ' / 'テーマをライブラリにインポートします'
    const html = wrapper.html()
    expect(html).toContain('.cursorpack')
    // ja か en のどちらかの文言を含む
    expect(html).toMatch(/(ドロップ|Drop)/)
  })

  it('reactively switches between show states', async () => {
    const wrapper = mount(LibraryDropOverlay, {
      props: { show: false },
      global: { stubs },
    })
    expect(wrapper.find('.drop').exists()).toBe(false)
    await wrapper.setProps({ show: true })
    expect(wrapper.find('.drop').exists()).toBe(true)
    await wrapper.setProps({ show: false })
    expect(wrapper.find('.drop').exists()).toBe(false)
  })
})
