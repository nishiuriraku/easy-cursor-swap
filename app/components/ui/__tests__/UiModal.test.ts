/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import UiModal from '../UiModal.vue'

const stubs = { UiIcon: { template: '<span></span>', props: ['name', 'size'] } }

const wrappers: VueWrapper[] = []
afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
  document.body.style.overflow = ''
})

function mountModal(props: Record<string, unknown>, slots: Record<string, string> = {}) {
  const w = mount(UiModal, { props, slots, global: { stubs }, attachTo: document.body })
  wrappers.push(w)
  return w
}

describe('UiModal', () => {
  it('renders nothing when open=false', () => {
    const w = mountModal({ open: false, title: 't' })
    // Teleport to body: contents render to document, but inner .modal-page should not exist
    expect(document.querySelector('.modal-page')).toBeNull()
    void w
  })

  it('renders modal-page when open=true', async () => {
    const w = mountModal({ open: true, title: 'Hello', description: 'desc' })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page')
    expect(page).not.toBeNull()
    expect(page?.textContent).toContain('Hello')
    expect(page?.textContent).toContain('desc')
  })

  it('emits update:open=false and close on backdrop click', async () => {
    const w = mountModal({ open: true, title: 't' })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page') as HTMLElement
    page.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    // backdrop click only fires when event.target === event.currentTarget; our dispatch
    // targets the page itself, so it qualifies.
    expect(w.emitted('update:open')?.[0]).toEqual([false])
    expect(w.emitted('close')).toHaveLength(1)
  })

  it('does not close on backdrop click when busy=true', async () => {
    const w = mountModal({ open: true, title: 't', busy: true })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page') as HTMLElement
    page.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(w.emitted('update:open')).toBeUndefined()
  })

  it('does not close on backdrop click when closeOnBackdrop=false', async () => {
    const w = mountModal({ open: true, title: 't', closeOnBackdrop: false })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page') as HTMLElement
    page.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(w.emitted('update:open')).toBeUndefined()
  })

  it('emits close on Escape keydown', async () => {
    const w = mountModal({ open: true, title: 't' })
    await w.vm.$nextTick()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('close')).toHaveLength(1)
  })

  it('does not close on Escape when busy=true', async () => {
    const w = mountModal({ open: true, title: 't', busy: true })
    await w.vm.$nextTick()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('close')).toBeUndefined()
  })

  it('locks body scroll when open', async () => {
    document.body.style.overflow = 'auto'
    const w = mountModal({ open: true, title: 't' })
    await w.vm.$nextTick()
    expect(document.body.style.overflow).toBe('hidden')
  })

  it('renders default body slot', async () => {
    const w = mountModal({ open: true, title: 't' }, { default: '<p data-testid="body">Body</p>' })
    await w.vm.$nextTick()
    expect(document.querySelector('[data-testid="body"]')?.textContent).toBe('Body')
  })

  it('renders actions slot inside modal-foot', async () => {
    const w = mountModal(
      { open: true, title: 't' },
      { actions: '<button class="action-btn">Go</button>' },
    )
    await w.vm.$nextTick()
    expect(document.querySelector('.modal-foot .action-btn')).not.toBeNull()
  })

  it('does not render modal-foot when no actions/leftNote slots', async () => {
    const w = mountModal({ open: true, title: 't' })
    await w.vm.$nextTick()
    expect(document.querySelector('.modal-foot')).toBeNull()
  })
})
