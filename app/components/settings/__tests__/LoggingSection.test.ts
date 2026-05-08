/**
 * LoggingSection コンポーネントテスト。
 *
 * ログレベル選択 (5 値) + 数値入力 2 つ + フォルダオープンボタン (現状ダミー)。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LoggingSection from '../LoggingSection.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  SettingsRow: {
    props: ['label', 'desc'],
    template: '<div :data-label="label"><slot /></div>',
  },
  UiSelect: {
    props: ['modelValue', 'options'],
    emits: ['update:modelValue'],
    template:
      '<select class="select-stub" :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><option v-for="o in options" :key="o.value" :value="o.value">{{o.label}}</option></select>',
  },
}

const baseProps = {
  logLevel: 'INFO',
  retentionDays: 14,
  maxSizeMb: 100,
}

describe('LoggingSection', () => {
  it('renders log level select with 5 options', () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    const vals = wrapper.findAll('option').map((o) => o.attributes('value'))
    expect(vals).toEqual(['TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR'])
  })

  it('emits update:logLevel on select change', async () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    await wrapper.find('select').setValue('DEBUG')
    expect(wrapper.emitted('update:logLevel')).toEqual([['DEBUG']])
  })

  it('renders 2 number inputs for retention and maxSize', () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input[type="number"]')
    expect(inputs).toHaveLength(2)
    expect(inputs[0]!.attributes('min')).toBe('1')
    expect(inputs[0]!.attributes('max')).toBe('365')
    expect(inputs[1]!.attributes('min')).toBe('10')
    expect(inputs[1]!.attributes('max')).toBe('2048')
  })

  it('emits update:retentionDays as number when first input changes', async () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input[type="number"]')
    await inputs[0]!.setValue('30')
    // v-model.number → number として emit される
    const emits = wrapper.emitted('update:retentionDays')
    expect(emits).toBeTruthy()
    expect(emits![0]).toEqual([30])
    expect(typeof emits![0]![0]).toBe('number')
  })

  it('emits update:maxSizeMb as number when second input changes', async () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input[type="number"]')
    await inputs[1]!.setValue('500')
    const emits = wrapper.emitted('update:maxSizeMb')
    expect(emits![0]).toEqual([500])
  })

  it('renders open log folder button', () => {
    const wrapper = mount(LoggingSection, { props: baseProps, global: { stubs } })
    const buttons = wrapper.findAll('button')
    expect(buttons.length).toBeGreaterThanOrEqual(1)
  })
})
