/**
 * CreatorTabBar コンポーネントテスト (P08a Step 2 / V1)。
 *
 * 2 タブ描画・active クラス・click emit を確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorTabBar from '../CreatorTabBar.vue'

const tabs = [
  { id: 'assign', label: 'Assign', count: '3/17' },
  { id: 'metadata', label: 'Metadata' },
]

describe('CreatorTabBar', () => {
  it('renders tabs with labels and counts', () => {
    const wrapper = mount(CreatorTabBar, { props: { tabs, modelValue: 'assign' } })
    const buttons = wrapper.findAll('.tab')
    expect(buttons).toHaveLength(2)
    expect(buttons[0]!.text()).toContain('Assign')
    expect(buttons[0]!.find('.num').text()).toBe('3/17')
    expect(buttons[1]!.find('.num').exists()).toBe(false)
  })

  it('marks the active tab', () => {
    const wrapper = mount(CreatorTabBar, { props: { tabs, modelValue: 'metadata' } })
    const buttons = wrapper.findAll('.tab')
    expect(buttons[0]!.classes()).not.toContain('active')
    expect(buttons[1]!.classes()).toContain('active')
  })

  it('emits update:modelValue on click', async () => {
    const wrapper = mount(CreatorTabBar, { props: { tabs, modelValue: 'assign' } })
    await wrapper.findAll('.tab')[1]!.trigger('click')
    expect(wrapper.emitted('update:modelValue')).toEqual([['metadata']])
  })
})
