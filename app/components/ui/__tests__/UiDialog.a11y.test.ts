/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import UiAlert from '../UiAlert.vue'
import UiConfirmDialog from '../UiConfirmDialog.vue'
import UiModal from '../UiModal.vue'
import UiProgress from '../UiProgress.vue'

const stubs = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
  UiButton: { template: '<button><slot /></button>' },
}

describe('ui dialog/feedback a11y', () => {
  it('UiAlert has no axe violations', async () => {
    const w = mount(UiAlert, {
      props: { tone: 'info', title: 'Note' },
      slots: { default: 'message' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('UiModal exposes dialog role with label', async () => {
    const w = mount(UiModal, {
      props: { open: true, title: 'Confirm' },
      slots: { default: 'body' },
      global: { stubs },
      attachTo: document.body,
    })
    // Teleport のため中身は body 直下。wrapper.element はアンカーなので body 全体を検査。
    expect(await axe(document.body)).toHaveNoViolations()
    w.unmount()
  })

  it('UiConfirmDialog has no axe violations', async () => {
    const w = mount(UiConfirmDialog, {
      props: { open: true, title: 'Delete?', message: 'Sure?' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(document.body)).toHaveNoViolations()
    w.unmount()
  })

  it('UiProgress exposes progressbar role with values', async () => {
    const w = mount(UiProgress, {
      props: { value: 40, label: 'Loading' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
