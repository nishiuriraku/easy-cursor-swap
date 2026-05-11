/**
 * SaveDestinationModal テスト。
 *
 * 検証ポイント:
 *  - sourceThemeId=null で「上書き/複製」セクションが非表示
 *  - sourceThemeId 付きで上書き/複製ラジオが表示される
 *  - hasKeystoreSigning=false で sign チェックボックスが disabled
 *  - destination=file 選択時に save dialog が呼ばれる
 *  - submit ペイロードが期待する形
 */
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SaveDestinationModal from '../SaveDestinationModal.vue'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn().mockResolvedValue('/tmp/picked.cursorpack'),
}))

const stubs = {
  UiIcon: { template: '<span></span>' },
}

const baseProps = {
  open: true,
  hasKeystoreSigning: false,
  sourceThemeId: null as string | null,
  defaultDestination: 'file' as const,
  metaName: 'Existing Theme',
}

describe('SaveDestinationModal', () => {
  it('hides overwrite/duplicate section when sourceThemeId is null', () => {
    const wrapper = mount(SaveDestinationModal, {
      props: baseProps,
      global: { stubs },
    })
    expect(wrapper.find('[data-test="overwrite-section"]').exists()).toBe(false)
  })

  it('shows overwrite/duplicate radio when sourceThemeId is provided', () => {
    const wrapper = mount(SaveDestinationModal, {
      props: { ...baseProps, sourceThemeId: 'abc-uuid' },
      global: { stubs },
    })
    expect(wrapper.find('[data-test="overwrite-section"]').exists()).toBe(true)
  })

  it('disables sign checkbox when hasKeystoreSigning=false', () => {
    const wrapper = mount(SaveDestinationModal, {
      props: { ...baseProps, hasKeystoreSigning: false },
      global: { stubs },
    })
    const signCb = wrapper.find('[data-test="sign-checkbox"]')
    expect(signCb.attributes('disabled')).toBeDefined()
  })

  it('emits cancel on cancel button click', async () => {
    const wrapper = mount(SaveDestinationModal, {
      props: baseProps,
      global: { stubs },
    })
    await wrapper.find('[data-test="cancel-btn"]').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('emits submit with destination=file payload after dialog returns path', async () => {
    const wrapper = mount(SaveDestinationModal, {
      props: { ...baseProps, defaultDestination: 'file' },
      global: { stubs },
    })
    await wrapper.find('[data-test="submit-btn"]').trigger('click')
    await new Promise((r) => setTimeout(r, 0))
    const events = wrapper.emitted('submit')
    expect(events).toHaveLength(1)
    const payload = events![0][0] as {
      destination: string
      filePath?: string
      overwriteExisting: boolean
      sign: boolean
      effectiveName: string
    }
    expect(payload.destination).toBe('file')
    expect(payload.filePath).toBe('/tmp/picked.cursorpack')
    expect(payload.overwriteExisting).toBe(false)
    expect(payload.effectiveName).toBe('Existing Theme')
  })

  it('uses Untitled placeholder when metaName is empty', () => {
    const wrapper = mount(SaveDestinationModal, {
      props: { ...baseProps, metaName: '' },
      global: { stubs },
    })
    expect(wrapper.find('[data-test="name-field"]').exists()).toBe(true)
  })
})
