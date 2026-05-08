/**
 * StartupSection の v-model バインディングテスト。
 *
 * 親が `v-model:auto-start` / `v-model:start-minimized` で渡す値が
 * 子から emit される `update:autoStart` / `update:startMinimized` で
 * 双方向に伝わることを確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import StartupSection from '../StartupSection.vue'

// SettingsToggle は実装が複雑なので、emit を再現する簡易 stub に置き換える。
const stubs = {
  UiIcon: { template: '<span></span>' },
  SettingsRow: {
    props: ['label', 'desc'],
    template: '<div :data-label="label"><slot /></div>',
  },
  SettingsToggle: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template:
      '<button class="toggle-stub" :data-on="modelValue" @click="$emit(\'update:modelValue\', !modelValue)">{{ modelValue ? "on" : "off" }}</button>',
  },
}

describe('StartupSection', () => {
  it('renders 2 toggles bound to props', () => {
    const wrapper = mount(StartupSection, {
      props: { autoStart: true, startMinimized: false },
      global: { stubs },
    })
    const toggles = wrapper.findAll('.toggle-stub')
    expect(toggles).toHaveLength(2)
    expect(toggles[0]!.attributes('data-on')).toBe('true')
    expect(toggles[1]!.attributes('data-on')).toBe('false')
  })

  it('emits update:autoStart when first toggle clicked', async () => {
    const wrapper = mount(StartupSection, {
      props: { autoStart: true, startMinimized: false },
      global: { stubs },
    })
    const toggles = wrapper.findAll('.toggle-stub')
    await toggles[0]!.trigger('click')
    // defineModel('autoStart') → update:autoStart イベント
    expect(wrapper.emitted('update:autoStart')).toEqual([[false]])
    // 反対側は emit されない
    expect(wrapper.emitted('update:startMinimized')).toBeUndefined()
  })

  it('emits update:startMinimized when second toggle clicked', async () => {
    const wrapper = mount(StartupSection, {
      props: { autoStart: false, startMinimized: false },
      global: { stubs },
    })
    const toggles = wrapper.findAll('.toggle-stub')
    await toggles[1]!.trigger('click')
    expect(wrapper.emitted('update:startMinimized')).toEqual([[true]])
    expect(wrapper.emitted('update:autoStart')).toBeUndefined()
  })

  it('reactively reflects external prop changes', async () => {
    const wrapper = mount(StartupSection, {
      props: { autoStart: false, startMinimized: false },
      global: { stubs },
    })
    let firstToggle = wrapper.findAll('.toggle-stub')[0]!
    expect(firstToggle.attributes('data-on')).toBe('false')
    await wrapper.setProps({ autoStart: true })
    firstToggle = wrapper.findAll('.toggle-stub')[0]!
    expect(firstToggle.attributes('data-on')).toBe('true')
  })

  it('renders i18n labels for both toggles', () => {
    const wrapper = mount(StartupSection, {
      props: { autoStart: false, startMinimized: false },
      global: { stubs },
    })
    const labels = wrapper.findAll('[data-label]').map((d) => d.attributes('data-label'))
    // 2 つのラベルが何かしら入る (ja か en の翻訳済み文字列)
    expect(labels).toHaveLength(2)
    for (const l of labels) {
      expect(l && l.length > 0).toBe(true)
    }
  })
})
