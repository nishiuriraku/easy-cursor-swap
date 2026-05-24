/**
 * DiscardEditDialog テスト。
 *
 * - open=false で非表示
 * - mode='clear' / 'navigate' で文言とタイトルが切り替わる
 * - 確定ボタンで `confirm`、キャンセルボタン / バックドロップ / Esc で `cancel` を emit
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import DiscardEditDialog from '../DiscardEditDialog.vue'
import { useI18n } from '~/composables/useI18n'

const stubs = { UiIcon: { template: '<span></span>', props: ['name', 'size'] } }

const wrappers: VueWrapper[] = []
afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
  document.body.style.overflow = ''
})

beforeEach(() => {
  useI18n().setLocale('ja')
})

function mountDialog(props: { open: boolean; mode: 'clear' | 'navigate' }) {
  const w = mount(DiscardEditDialog, { props, global: { stubs }, attachTo: document.body })
  wrappers.push(w)
  return w
}

describe('DiscardEditDialog', () => {
  it('renders nothing when open=false', () => {
    mountDialog({ open: false, mode: 'clear' })
    expect(document.querySelector('.modal-page')).toBeNull()
  })

  it('shows clear-specific title when mode=clear', async () => {
    const w = mountDialog({ open: true, mode: 'clear' })
    await w.vm.$nextTick()
    expect(document.body.textContent).toContain('編集内容を破棄しますか')
  })

  it('shows navigate-specific title when mode=navigate', async () => {
    const w = mountDialog({ open: true, mode: 'navigate' })
    await w.vm.$nextTick()
    expect(document.body.textContent).toContain('クリエイターを離れますか')
  })

  it('emits confirm when the danger button is clicked', async () => {
    const w = mountDialog({ open: true, mode: 'clear' })
    await w.vm.$nextTick()
    const btn = document.querySelector('.modal-foot .actions button.danger') as HTMLButtonElement
    btn.click()
    expect(w.emitted('confirm')).toHaveLength(1)
  })

  it('emits cancel when the ghost button is clicked', async () => {
    const w = mountDialog({ open: true, mode: 'clear' })
    await w.vm.$nextTick()
    const btn = document.querySelector('.modal-foot .actions button.ghost') as HTMLButtonElement
    btn.click()
    expect(w.emitted('cancel')).toHaveLength(1)
  })

  it('emits cancel on backdrop click', async () => {
    const w = mountDialog({ open: true, mode: 'clear' })
    await w.vm.$nextTick()
    const page = document.querySelector('.modal-page') as HTMLElement
    page.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(w.emitted('cancel')).toHaveLength(1)
  })

  it('emits cancel on Escape keydown', async () => {
    const w = mountDialog({ open: true, mode: 'clear' })
    await w.vm.$nextTick()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('cancel')).toHaveLength(1)
  })
})
