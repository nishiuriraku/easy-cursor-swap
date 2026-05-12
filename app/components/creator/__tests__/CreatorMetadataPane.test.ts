/**
 * CreatorMetadataPane テスト。
 *
 * 6 つの v-model (metaName/En/Author/Version/Description/shadowEnabled) +
 * 5 つの read-only props (arrowAssigned/assignedRoleCount/exportMessage/
 * exportProgress/exportBusy) + 2 つの emit (dismiss-export-message /
 * cancel-export) の独立性と表示挙動を確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorMetadataPane from '../CreatorMetadataPane.vue'

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

const baseProps = {
  metaName: 'TestTheme',
  metaNameEn: 'TestThemeEn',
  metaAuthor: '@me',
  metaVersion: '1.0.0',
  metaDescription: 'desc',
  shadowEnabled: true,
  arrowAssigned: true,
  assignedRoleCount: 5,
  exportMessage: null as string | null,
  exportProgress: null,
  exportBusy: false,
  hotspot: { x: 0.125, y: 0.125 }, // 4 / 32 = 0.125
  primarySize: 32,
  perSizeHotspot: false,
  activeRoleJp: '通常の選択',
  showAdvancedResolutions: false,
}

describe('CreatorMetadataPane', () => {
  it('renders 4 text inputs + 1 textarea + 1 toggle', () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })
    const textInputs = wrapper.findAll('input[type="text"], input:not([type])')
    expect(textInputs).toHaveLength(4)
    expect(wrapper.findAll('textarea')).toHaveLength(1)
    expect(wrapper.findAll('.toggle-stub')).toHaveLength(1)
  })

  it('emits update:metaName independent of other models', async () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input')
    await inputs[0]!.setValue('NewName')
    expect(wrapper.emitted('update:metaName')).toEqual([['NewName']])
    expect(wrapper.emitted('update:metaNameEn')).toBeUndefined()
    expect(wrapper.emitted('update:metaAuthor')).toBeUndefined()
    expect(wrapper.emitted('update:metaVersion')).toBeUndefined()
  })

  it('each text input emits its own model only', async () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input')
    await inputs[1]!.setValue('NameEn')
    await inputs[2]!.setValue('Auth')
    await inputs[3]!.setValue('2.0.0')
    expect(wrapper.emitted('update:metaNameEn')).toEqual([['NameEn']])
    expect(wrapper.emitted('update:metaAuthor')).toEqual([['Auth']])
    expect(wrapper.emitted('update:metaVersion')).toEqual([['2.0.0']])
    expect(wrapper.emitted('update:metaName')).toBeUndefined()
  })

  it('textarea emits update:metaDescription', async () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })
    await wrapper.find('textarea').setValue('long description')
    expect(wrapper.emitted('update:metaDescription')).toEqual([['long description']])
  })

  it('toggle emits update:shadowEnabled', async () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })
    await wrapper.find('.toggle-stub').trigger('click')
    expect(wrapper.emitted('update:shadowEnabled')).toEqual([[false]])
  })

  it('shows assignedRoleCount/17 in tag', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: { ...baseProps, assignedRoleCount: 12 },
      global: { stubs },
    })
    expect(wrapper.text()).toContain('12 / 17')
  })

  it('marks Arrow tag ok when arrowAssigned=true', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: { ...baseProps, arrowAssigned: true },
      global: { stubs },
    })
    // 「割り当て済み」or "Assigned" メッセージ
    expect(wrapper.text()).toMatch(/(割り当て済み|Assigned)/)
  })

  it('shows export message banner when exportMessage set', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: { ...baseProps, exportMessage: 'エクスポート完了' },
      global: { stubs },
    })
    expect(wrapper.find('.import-banner').exists()).toBe(true)
    expect(wrapper.text()).toContain('エクスポート完了')
  })

  it('emits dismiss-export-message when X clicked', async () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: { ...baseProps, exportMessage: 'message' },
      global: { stubs },
    })
    const closeBtn = wrapper.find('.import-banner button')
    await closeBtn.trigger('click')
    expect(wrapper.emitted('dismiss-export-message')).toHaveLength(1)
  })

  it('shows export progress with current/total when stage=role', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: {
        ...baseProps,
        exportProgress: {
          buildId: 'b1',
          stage: 'role' as const,
          current: 3,
          total: 18,
          message: 'Arrow',
        },
      },
      global: { stubs },
    })
    expect(wrapper.find('.export-progress').exists()).toBe(true)
    expect(wrapper.text()).toContain('Arrow')
    expect(wrapper.text()).toContain('3/18')
  })

  it('hides progress when stage=done', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: {
        ...baseProps,
        exportProgress: {
          buildId: 'b1',
          stage: 'done' as const,
          current: 18,
          total: 18,
          message: null,
        },
      },
      global: { stubs },
    })
    expect(wrapper.find('.export-progress').exists()).toBe(false)
  })

  it('emits cancel-export when cancel button clicked during export', async () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: {
        ...baseProps,
        exportBusy: true,
        exportProgress: {
          buildId: 'b1',
          stage: 'role' as const,
          current: 3,
          total: 18,
          message: 'Arrow',
        },
      },
      global: { stubs },
    })
    const cancelBtn = wrapper.find('.export-progress button')
    expect(cancelBtn.exists()).toBe(true)
    await cancelBtn.trigger('click')
    expect(wrapper.emitted('cancel-export')).toHaveLength(1)
  })

  it('renders the Hotspot section with the active role name and emits updates', async () => {
    const wrapper = mount(CreatorMetadataPane, { props: baseProps, global: { stubs } })

    // Section title + role tag are visible (locale is environment-dependent)
    expect(wrapper.text()).toMatch(/(ホットスポット \(現在ロール\)|Hotspot \(current role\))/)
    expect(wrapper.text()).toContain('通常の選択')

    // 2 number inputs (X / Y) are mounted with the px-derived values (0.125 * 32 = 4)
    const numberInputs = wrapper.findAll<HTMLInputElement>('input[type="number"]')
    expect(numberInputs.length).toBe(2)
    expect(numberInputs[0]!.element.value).toBe('4')
    expect(numberInputs[1]!.element.value).toBe('4')

    // Updating X emits update:hotspot with ratio (defineModel pattern)
    await numberInputs[0]!.setValue(12)
    const emitted = wrapper.emitted('update:hotspot')
    expect(emitted).toBeTruthy()
    // ratio = 12 / 32 = 0.375
    expect((emitted![0]![0] as { x: number }).x).toBeCloseTo(0.375)

    // Per-size toggle is hidden when showAdvancedResolutions is false
    const toggleButtons = wrapper.findAll('.toggle-stub')
    // existing single-toggle baseline has 1 toggle (shadow); new perSize toggle is hidden
    expect(toggleButtons.length).toBe(1)
  })

  it('shows per-size toggle when showAdvancedResolutions is true', () => {
    const wrapper = mount(CreatorMetadataPane, {
      props: { ...baseProps, showAdvancedResolutions: true },
      global: { stubs },
    })

    // Now both shadow toggle AND per-size toggle should be present (= 2 toggles)
    expect(wrapper.findAll('.toggle-stub').length).toBe(2)
  })
})
