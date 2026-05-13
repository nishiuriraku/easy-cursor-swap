/**
 * BulkImportPreviewModal の ApplyPayload コントラクトを固定化する。
 *
 * 2026-05-13 の UI 簡略化で「すぐシステムに反映」トグル
 * (旧 `applyImmediately` フィールド) を削除した。本テストは
 * 「apply で emit される payload に applyImmediately キーが含まれない」
 * ことを契約として固定する — 将来の差し戻し防止用。
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

describe('BulkImportPreviewModal apply payload', () => {
  it('apply emit の payload に applyImmediately キーは含まない', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      global: { stubs },
    })
    await wrapper.find('button.primary').trigger('click')
    const events = wrapper.emitted('apply')
    expect(events).toHaveLength(1)
    const payload = events![0]![0] as Record<string, unknown>
    expect('applyImmediately' in payload).toBe(false)
    expect(payload).toHaveProperty('roleAssets')
    expect(payload).toHaveProperty('metadataChoice')
    expect(payload).toHaveProperty('metadata')
  })

  it('フッターに data-test="apply-immediately" を持つ要素は描画されない', () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      global: { stubs },
    })
    expect(wrapper.find('[data-test="apply-immediately"]').exists()).toBe(false)
  })
})
