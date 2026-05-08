/**
 * SecuritySection の v-model テスト。
 *
 * 署名検証関連の 2 トグル (requireSignedThemes / warnUnsignedImport) が正しく
 * 子から親へ伝播することを確認。これらは未署名テーマ実行を防ぐ重要な設定で、
 * v-model のミスバインドによる無効化は実害が大きいので網羅する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import SecuritySection from '../SecuritySection.vue'

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
      '<button class="toggle-stub" :data-on="modelValue" @click="$emit(\'update:modelValue\', !modelValue)" />',
  },
}

describe('SecuritySection', () => {
  it('renders 2 toggles', () => {
    const wrapper = mount(SecuritySection, {
      props: { requireSignedThemes: true, warnUnsignedImport: true },
      global: { stubs },
    })
    expect(wrapper.findAll('.toggle-stub')).toHaveLength(2)
  })

  it('first toggle emits update:requireSignedThemes', async () => {
    const wrapper = mount(SecuritySection, {
      props: { requireSignedThemes: true, warnUnsignedImport: true },
      global: { stubs },
    })
    await wrapper.findAll('.toggle-stub')[0]!.trigger('click')
    expect(wrapper.emitted('update:requireSignedThemes')).toEqual([[false]])
    expect(wrapper.emitted('update:warnUnsignedImport')).toBeUndefined()
  })

  it('second toggle emits update:warnUnsignedImport', async () => {
    const wrapper = mount(SecuritySection, {
      props: { requireSignedThemes: false, warnUnsignedImport: false },
      global: { stubs },
    })
    await wrapper.findAll('.toggle-stub')[1]!.trigger('click')
    expect(wrapper.emitted('update:warnUnsignedImport')).toEqual([[true]])
    expect(wrapper.emitted('update:requireSignedThemes')).toBeUndefined()
  })

  it('toggles do not cross-wire (regression guard)', async () => {
    // 2 つの defineModel が混線しないことを確認 (同じ親で両方 false → トグル両方押下)
    const wrapper = mount(SecuritySection, {
      props: { requireSignedThemes: false, warnUnsignedImport: false },
      global: { stubs },
    })
    const [first, second] = wrapper.findAll('.toggle-stub')
    await first!.trigger('click')
    await second!.trigger('click')
    // 各々が 1 回ずつ true で emit されているはず
    expect(wrapper.emitted('update:requireSignedThemes')).toEqual([[true]])
    expect(wrapper.emitted('update:warnUnsignedImport')).toEqual([[true]])
  })
})
