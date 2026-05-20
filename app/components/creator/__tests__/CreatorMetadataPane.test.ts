/**
 * CreatorMetadataPane テスト。
 *
 * 6 つの v-model (metaName/En/Author/Version/Description/shadowEnabled) +
 * 4 つの read-only props (arrowAssigned/assignedRoleCount/exportProgress/exportBusy) +
 * 1 つの emit (cancel-export) の独立性と表示挙動を確認する。
 *
 * NOTE: 保存トースト (exportMessage/failedApplyThemeId/dismiss-export-message/retry-apply)
 * は activeTab に依存せず表示するため creator.vue page-level に移管された。
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
  exportProgress: null,
  exportBusy: false,
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

  // NOTE: exportMessage banner と dismiss-export-message emit は creator.vue 側へ
  // 移管されたため、当該 2 ケースのテストは削除した (詳細はファイル冒頭の NOTE 参照)。

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
})
