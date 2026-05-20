/**
 * LibraryEmptyState コンポーネントテスト。
 *
 * テーマがゼロ件のときのヒーロー UI。3 つの導線 (新規作成 / インポート /
 * Marketplace) のうち、`open-import` のみクリックで emit される。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h } from 'vue'
import LibraryEmptyState from '../LibraryEmptyState.vue'

// NuxtLink は本体側で `custom v-slot` 形式に切り替わったため、スロットに渡される
// `navigate` を含むスロット props をモックし、外側を <span data-testid="nuxt-link">
// で囲んで「どのルート向け NuxtLink か」をテスト側から判定できるようにする。
// data-to で `to` 値を取り出せる。中身はスロット内容 (= <button>) がそのまま入る。
const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  NuxtLink: {
    props: ['to', 'custom'],
    setup(props: { to: string; custom?: boolean }, { slots }: { slots: any }) {
      const navigate = () => {}
      return () =>
        h(
          'span',
          { 'data-testid': 'nuxt-link', 'data-to': props.to },
          slots.default?.({ navigate, isActive: false, isExactActive: false, href: props.to }),
        )
    },
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
    const targets = links.map((l) => l.attributes('data-to'))
    expect(targets).toContain('/creator')
    expect(targets).toContain('/marketplace')

    // NuxtLink の slot 内で <button> がレンダリングされているか確認
    // (custom v-slot に切り替えたあとも CTA 行のボタン数は 3 個のまま)
    const ctaButtons = wrapper.findAll('.es-cta-row button')
    expect(ctaButtons).toHaveLength(3)
  })

  it('emits open-import when import button clicked', async () => {
    const wrapper = mount(LibraryEmptyState, { global: { stubs } })
    // NuxtLink スタブの中ではない直接の子 <button> が import ボタン。
    const importBtn = wrapper.find<HTMLButtonElement>('.es-cta-row > button')
    expect(importBtn.exists()).toBe(true)
    await importBtn.trigger('click')
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
