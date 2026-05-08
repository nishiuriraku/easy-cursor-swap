/**
 * LibraryToolbar テスト。
 *
 * パンくず + 検索ボックス + Import/New ボタン。
 * v-model:searchQuery と open-import emit を検証。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibraryToolbar from '../LibraryToolbar.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  NuxtLink: {
    props: ['to'],
    template: '<a :href="to" data-testid="nuxt-link"><slot /></a>',
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
    // ボタンは Import 1 つ (New は NuxtLink)
    const buttons = wrapper.findAll('button')
    expect(buttons).toHaveLength(1)
    await buttons[0]!.trigger('click')
    expect(wrapper.emitted('open-import')).toHaveLength(1)
  })

  it('renders New button as NuxtLink to /creator', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    const link = wrapper.find('[data-testid="nuxt-link"]')
    expect(link.exists()).toBe(true)
    expect(link.attributes('href')).toBe('/creator')
  })

  it('shows ⌘K keyboard hint badge', () => {
    const wrapper = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
    })
    expect(wrapper.find('.kbd').text()).toBe('⌘K')
  })
})
