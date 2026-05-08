/**
 * LibrarySection コンポーネントテスト。
 *
 * ストレージ警告閾値 + .cursorprofile export/import の確認。
 * profileBusy で disabled 制御 / profileMessage の表示確認も。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibrarySection from '../LibrarySection.vue'

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
      '<select class="select-stub" :value="modelValue" @change="$emit(\'update:modelValue\', Number($event.target.value))"><option v-for="o in options" :key="o.value" :value="o.value">{{o.label}}</option></select>',
  },
}

const baseProps = {
  totalLimitWarnGb: 1,
  storageWarnEnabled: true,
  profileBusy: false,
  profileMessage: null,
}

describe('LibrarySection', () => {
  it('renders storage threshold select with 4 options', () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    const opts = wrapper.findAll('option')
    expect(opts).toHaveLength(4)
    const vals = opts.map((o) => o.attributes('value'))
    expect(vals).toEqual(['0.5', '1', '2', '5'])
  })

  it('emits update:totalLimitWarnGb on select change', async () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    await wrapper.find('select').setValue('2')
    expect(wrapper.emitted('update:totalLimitWarnGb')).toEqual([[2]])
  })

  it('emits update:storageWarnEnabled on toggle', async () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    await wrapper.find('.toggle-stub').trigger('click')
    expect(wrapper.emitted('update:storageWarnEnabled')).toEqual([[false]])
  })

  it('emits export-profile when export button clicked', async () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    const buttons = wrapper.findAll('button').filter((b) => !b.classes().includes('toggle-stub'))
    expect(buttons).toHaveLength(2)
    await buttons[0]!.trigger('click')
    expect(wrapper.emitted('export-profile')).toHaveLength(1)
    expect(wrapper.emitted('import-profile')).toBeUndefined()
  })

  it('emits import-profile when import button clicked', async () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    const buttons = wrapper.findAll('button').filter((b) => !b.classes().includes('toggle-stub'))
    await buttons[1]!.trigger('click')
    expect(wrapper.emitted('import-profile')).toHaveLength(1)
    expect(wrapper.emitted('export-profile')).toBeUndefined()
  })

  it('disables export/import buttons when profileBusy=true', () => {
    const wrapper = mount(LibrarySection, {
      props: { ...baseProps, profileBusy: true },
      global: { stubs },
    })
    const buttons = wrapper.findAll('button').filter((b) => !b.classes().includes('toggle-stub'))
    for (const b of buttons) {
      expect(b.attributes('disabled')).toBeDefined()
    }
  })

  it('shows profileMessage when present', () => {
    const wrapper = mount(LibrarySection, {
      props: { ...baseProps, profileMessage: 'エクスポート完了: foo.cursorprofile' },
      global: { stubs },
    })
    expect(wrapper.find('.profile-msg').exists()).toBe(true)
    expect(wrapper.text()).toContain('エクスポート完了')
  })

  it('hides profileMessage when null', () => {
    const wrapper = mount(LibrarySection, { props: baseProps, global: { stubs } })
    expect(wrapper.find('.profile-msg').exists()).toBe(false)
  })
})
