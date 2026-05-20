/**
 * DiscardEditDialog テスト。
 *
 * - open=false で非表示
 * - mode='clear' / 'navigate' で文言とタイトルが切り替わる
 * - 確定ボタンで `confirm`、キャンセルボタン / バックドロップ / Esc で `cancel` を emit
 */
import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import DiscardEditDialog from '../DiscardEditDialog.vue'
import { useI18n } from '~/composables/useI18n'

const stubs = {
  UiIcon: { template: '<span></span>' },
}

beforeEach(() => {
  useI18n().setLocale('ja')
})

describe('DiscardEditDialog', () => {
  it('renders nothing when open=false', () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: false, mode: 'clear' },
      global: { stubs },
    })
    expect(wrapper.find('.modal-page').exists()).toBe(false)
  })

  it('shows clear-specific title when mode=clear', () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    expect(wrapper.text()).toContain('編集内容を破棄しますか')
  })

  it('shows navigate-specific title when mode=navigate', () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'navigate' },
      global: { stubs },
    })
    expect(wrapper.text()).toContain('クリエイターを離れますか')
  })

  it('emits confirm when the danger button is clicked', async () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    await wrapper.find('button.danger').trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
    expect(wrapper.emitted('cancel')).toBeUndefined()
  })

  it('emits cancel when the ghost button is clicked', async () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    await wrapper.find('button.ghost').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('emits cancel on backdrop click (event.target === currentTarget)', async () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    // backdrop=root .modal-page; click 自身 (currentTarget=target) のときに cancel
    await wrapper.find('.modal-page').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('does NOT emit cancel when clicking inside the modal body', async () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    // .modal は @click.stop で伝播停止するので backdrop ハンドラに届かない
    await wrapper.find('.modal').trigger('click')
    expect(wrapper.emitted('cancel')).toBeUndefined()
  })

  it('emits cancel on Escape key', async () => {
    const wrapper = mount(DiscardEditDialog, {
      props: { open: true, mode: 'clear' },
      global: { stubs },
    })
    await wrapper.find('.modal-page').trigger('keydown', { key: 'Escape' })
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })
})
