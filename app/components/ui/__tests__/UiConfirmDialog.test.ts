/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import UiConfirmDialog from '../UiConfirmDialog.vue'

const stubs = { UiIcon: { template: '<span></span>', props: ['name', 'size'] } }

const wrappers: VueWrapper[] = []
afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
  document.body.style.overflow = ''
})

function mountDialog(props: Record<string, unknown>) {
  const w = mount(UiConfirmDialog, { props, global: { stubs }, attachTo: document.body })
  wrappers.push(w)
  return w
}

describe('UiConfirmDialog', () => {
  it('renders title and message when open', async () => {
    const w = mountDialog({
      open: true,
      title: 'Sure?',
      message: 'This cannot be undone.',
      confirmLabel: 'Yes',
    })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page')
    expect(page?.textContent).toContain('Sure?')
    expect(page?.textContent).toContain('This cannot be undone.')
  })

  it('emits confirm when confirm button clicked', async () => {
    const w = mountDialog({ open: true, title: 't', message: 'm', confirmLabel: 'Go' })
    await w.vm.$nextTick()
    const btn = document.querySelector(
      '.modal-foot .actions button:last-child',
    ) as HTMLButtonElement
    btn.click()
    expect(w.emitted('confirm')).toHaveLength(1)
  })

  it('emits cancel when cancel button clicked', async () => {
    const w = mountDialog({ open: true, title: 't', message: 'm', confirmLabel: 'Go' })
    await w.vm.$nextTick()
    const btn = document.querySelector(
      '.modal-foot .actions button:first-child',
    ) as HTMLButtonElement
    btn.click()
    expect(w.emitted('cancel')).toHaveLength(1)
    expect(w.emitted('update:open')?.[0]).toEqual([false])
  })

  it('uses danger variant for tone=danger', async () => {
    const w = mountDialog({
      open: true,
      title: 't',
      message: 'm',
      confirmLabel: 'X',
      tone: 'danger',
    })
    await w.vm.$nextTick()
    const confirmBtn = document.querySelector('.modal-foot .actions button:last-child')
    expect(confirmBtn?.className).toContain('danger')
  })

  it('disables buttons and ignores Esc when busy', async () => {
    const w = mountDialog({ open: true, title: 't', message: 'm', confirmLabel: 'X', busy: true })
    await w.vm.$nextTick()
    const buttons = document.querySelectorAll('.modal-foot .actions button')
    buttons.forEach((b) => expect((b as HTMLButtonElement).disabled).toBe(true))
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('cancel')).toBeUndefined()
  })
})
