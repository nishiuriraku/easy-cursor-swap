/**
 * GeneralSection コンポーネントテスト。
 *
 * 表示言語切替 + 通知トグル 2 つ + ConfigRecoveryPanel 連携。
 * UI 言語の v-model は string、トグルは boolean。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import GeneralSection from '../GeneralSection.vue'

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
  UiSelect: {
    props: ['modelValue', 'options'],
    emits: ['update:modelValue'],
    template:
      '<select class="select-stub" :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><option v-for="o in options" :key="o.value" :value="o.value">{{o.label}}</option></select>',
  },
  ConfigRecoveryPanel: {
    emits: ['restored'],
    template: '<div class="recovery-stub" @click="$emit(\'restored\')" />',
  },
}

const baseProps = {
  language: 'ja',
  showApplyToast: true,
  applyShadowControl: true,
}

describe('GeneralSection', () => {
  it('renders language select with options', () => {
    const wrapper = mount(GeneralSection, { props: baseProps, global: { stubs } })
    const select = wrapper.find('.select-stub')
    expect(select.exists()).toBe(true)
    const optionVals = wrapper.findAll('option').map((o) => o.attributes('value'))
    expect(optionVals).toEqual(['ja', 'en'])
  })

  it('emits update:language when select changes', async () => {
    const wrapper = mount(GeneralSection, { props: baseProps, global: { stubs } })
    const select = wrapper.find('select')
    await select.setValue('en')
    expect(wrapper.emitted('update:language')).toEqual([['en']])
  })

  it('emits update:showApplyToast when first toggle clicked', async () => {
    const wrapper = mount(GeneralSection, { props: baseProps, global: { stubs } })
    const toggles = wrapper.findAll('.toggle-stub')
    expect(toggles).toHaveLength(2)
    await toggles[0]!.trigger('click')
    expect(wrapper.emitted('update:showApplyToast')).toEqual([[false]])
    expect(wrapper.emitted('update:applyShadowControl')).toBeUndefined()
  })

  it('emits update:applyShadowControl when second toggle clicked', async () => {
    const wrapper = mount(GeneralSection, { props: baseProps, global: { stubs } })
    await wrapper.findAll('.toggle-stub')[1]!.trigger('click')
    expect(wrapper.emitted('update:applyShadowControl')).toEqual([[false]])
  })

  it('forwards ConfigRecoveryPanel restored event as config-restored', async () => {
    const wrapper = mount(GeneralSection, { props: baseProps, global: { stubs } })
    await wrapper.find('.recovery-stub').trigger('click')
    expect(wrapper.emitted('config-restored')).toHaveLength(1)
  })
})
