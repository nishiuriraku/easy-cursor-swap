/**
 * LibraryEmptyState コンポーネントテスト。
 *
 * テーマがゼロ件のときのヒーロー UI。3 つの導線 (新規作成 / インポート /
 * Marketplace) のうち、`open-import` のみクリックで emit される。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibraryEmptyState from '../LibraryEmptyState.vue'

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  NuxtLink: {
    props: ['to'],
    template: '<a :href="to" data-testid="nuxt-link"><slot /></a>',
  },
}

describe('LibraryEmptyState', () => {
  it('renders the empty hero UI', () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    expect(wrapper.find('.es-stage').exists()).toBe(true)
    expect(wrapper.find('.es-empty').exists()).toBe(true)
    expect(wrapper.text()).toContain('EMPTY · NO THEMES')
  })

  it('renders 2 NuxtLinks (creator + marketplace) and 1 import button', () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    const links = wrapper.findAll('[data-testid="nuxt-link"]')
    // /creator (primary) と /marketplace (ghost) の 2 個
    expect(links).toHaveLength(2)
    const hrefs = links.map((l) => l.attributes('href'))
    expect(hrefs).toContain('/creator')
    expect(hrefs).toContain('/marketplace')
  })

  it('emits open-import when import button clicked', async () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    // 「.cursorpack をインポート」ボタンを探す。NuxtLink でない button 要素はちょうど 1 つ。
    const buttons = wrapper.findAll('button')
    expect(buttons).toHaveLength(1)
    await buttons[0]!.trigger('click')
    expect(wrapper.emitted('open-import')).toHaveLength(1)
  })

  it('does not emit open-import on initial render', () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    expect(wrapper.emitted('open-import')).toBeUndefined()
  })

  it('renders drop hint section', () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    expect(wrapper.find('.es-drop').exists()).toBe(true)
    expect(wrapper.text()).toContain('.cursorpack')
  })
})
