/**
 * LibraryToolbar テスト。
 *
 * パンくず + 検索ボックス + Import/New ボタン。
 * v-model:searchQuery と open-import emit を検証。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h } from 'vue'
import LibraryToolbar from '../LibraryToolbar.vue'

// NuxtLink は本体側で `custom v-slot` 形式に切り替わったため、スロットに渡される
// `navigate` を含むスロット props をモックする。外側 <span> 包みで「NuxtLink がどの
// ルートを向いているか」を data-to で識別可能にする。
const stubs = {
  UiIcon: { template: '<span></span>' },
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

describe('LibraryToolbar', () => {
  it('renders breadcrumb with library title', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    expect(wrapper.find('.bcrumb').exists()).toBe(true)
    expect(wrapper.find('.crumb.active').exists()).toBe(true)
  })

  it('emits update:searchQuery when search input changes', async () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    await wrapper.find('input').setValue('mint')
    expect(wrapper.emitted('update:searchQuery')).toEqual([['mint']])
  })

  it('renders search input with current query value', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: 'existing-query' },
      global: { stubs },
    })
    const input = wrapper.find('input')
    expect((input.element as HTMLInputElement).value).toBe('existing-query')
  })

  it('emits open-import on Import button click', async () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    // Import ボタンは .tb-actions の直接の子 <button> (New は NuxtLink スタブ内)。
    const importBtn = wrapper.find<HTMLButtonElement>('.tb-actions > button')
    expect(importBtn.exists()).toBe(true)
    await importBtn.trigger('click')
    expect(wrapper.emitted('open-import')).toHaveLength(1)
  })

  it('renders New button as NuxtLink to /creator', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    const link = wrapper.find('[data-testid="nuxt-link"]')
    expect(link.exists()).toBe(true)
    // NuxtLink を custom v-slot 化したので `data-to` で参照先を確認する。
    expect(link.attributes('data-to')).toBe('/creator')
    // slot 内に <button> がレンダリングされていること
    expect(link.find('button').exists()).toBe(true)
  })

  it('does not render a keyboard shortcut hint (Ctrl+K is not wired up)', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    expect(wrapper.find('.kbd').exists()).toBe(false)
  })
})
