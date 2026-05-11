/**
 * BulkImportPreviewModal の applyImmediately フラグ追加分のテスト。
 * モーダル全体ではなく「apply 時に payload に applyImmediately が含まれるか」のみ確認。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import BulkImportPreviewModal from '../BulkImportPreviewModal.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  UiSelect: { template: '<select></select>' },
  BulkImportRoleRow: { template: '<div></div>' },
}

const baseProps = {
  open: true,
  resolved: [],
  cursorpack: null,
  existingRoles: new Set<string>(),
  sourceLabel: 'test',
}

describe('BulkImportPreviewModal applyImmediately', () => {
  it('emits apply payload with applyImmediately=false by default', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      global: { stubs },
    })
    await wrapper.find('button.primary').trigger('click')
    const events = wrapper.emitted('apply')
    expect(events).toHaveLength(1)
    const payload = events![0][0] as { applyImmediately: boolean }
    expect(payload.applyImmediately).toBe(false)
  })

  it('emits apply payload with applyImmediately=true after checkbox toggle', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      global: { stubs },
    })
    const cb = wrapper.find('input[data-test="apply-immediately"]')
    await cb.setValue(true)
    await wrapper.find('button.primary').trigger('click')
    const events = wrapper.emitted('apply')
    const payload = events![0][0] as { applyImmediately: boolean }
    expect(payload.applyImmediately).toBe(true)
  })
})
