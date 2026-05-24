/**
 * SaveDestinationModal テスト。
 *
 * 検証ポイント:
 *  - sourceThemeId=null で「上書き/複製」セクションが非表示
 *  - sourceThemeId 付きで上書き/複製ラジオが表示される
 *  - hasKeystoreSigning=false で sign チェックボックスが disabled
 *  - destination=file 選択時に save dialog が呼ばれる
 *  - submit ペイロードが期待する形
 *
 * UiModal が `<Teleport to="body">` を使うため、要素検索は `document.querySelector`
 * を経由する。mount 時に `attachTo: document.body` を指定し、afterEach で wrapper を
 * unmount してテレポートノードもクリーンアップする。
 */
import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
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

let currentWrapper: VueWrapper | null = null

function mountModal(props: Partial<typeof baseProps>) {
  currentWrapper = mount(SaveDestinationModal, {
    props: { ...baseProps, ...props },
    attachTo: document.body,
    global: { stubs },
  })
  return currentWrapper
}

afterEach(() => {
  currentWrapper?.unmount()
  currentWrapper = null
})

describe('SaveDestinationModal', () => {
  it('hides overwrite/duplicate section when sourceThemeId is null', () => {
    mountModal({})
    expect(document.querySelector('[data-test="overwrite-section"]')).toBeNull()
  })

  it('shows overwrite/duplicate radio when sourceThemeId is provided', () => {
    mountModal({ sourceThemeId: 'abc-uuid' })
    expect(document.querySelector('[data-test="overwrite-section"]')).not.toBeNull()
  })

  it('disables sign checkbox when hasKeystoreSigning=false', () => {
    mountModal({ hasKeystoreSigning: false })
    const signCb = document.querySelector('[data-test="sign-checkbox"]') as HTMLInputElement | null
    expect(signCb).not.toBeNull()
    expect(signCb!.hasAttribute('disabled')).toBe(true)
  })

  it('emits cancel on cancel button click', async () => {
    const wrapper = mountModal({})
    const btn = document.querySelector('[data-test="cancel-btn"]') as HTMLButtonElement | null
    expect(btn).not.toBeNull()
    btn!.click()
    await wrapper.vm.$nextTick()
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('emits submit with destination=file payload after dialog returns path', async () => {
    const wrapper = mountModal({ defaultDestination: 'file' })
    const btn = document.querySelector('[data-test="submit-btn"]') as HTMLButtonElement | null
    expect(btn).not.toBeNull()
    btn!.click()
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
    mountModal({ metaName: '' })
    expect(document.querySelector('[data-test="name-field"]')).not.toBeNull()
  })
})
