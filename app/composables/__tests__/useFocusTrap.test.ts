/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it } from 'vitest'
import { defineComponent, h, ref, type Ref } from 'vue'
import { mount, type VueWrapper } from '@vue/test-utils'
import { useFocusTrap } from '../useFocusTrap'

const wrappers: VueWrapper[] = []

afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
})

/**
 * テスト用ホスト: 3 つの button (a, b, c) を持つコンテナを描画し、
 * `active` ref に連動して useFocusTrap を有効化する。
 */
function mountHost(active: Ref<boolean>): {
  wrapper: VueWrapper
  container: HTMLElement
  buttons: HTMLButtonElement[]
} {
  const Host = defineComponent({
    setup() {
      const containerRef = ref<HTMLElement | null>(null)
      useFocusTrap(containerRef, active)
      return () =>
        h('div', { ref: containerRef, 'data-testid': 'container' }, [
          h('button', { id: 'a' }, 'a'),
          h('button', { id: 'b' }, 'b'),
          h('button', { id: 'c' }, 'c'),
        ])
    },
  })
  const wrapper = mount(Host, { attachTo: document.body })
  wrappers.push(wrapper)
  const container = wrapper.find<HTMLElement>('[data-testid="container"]').element
  const buttons = Array.from(container.querySelectorAll('button')) as HTMLButtonElement[]
  return { wrapper, container, buttons }
}

describe('useFocusTrap', () => {
  it('focuses the first focusable element when activated', async () => {
    const active = ref(false)
    const { wrapper, buttons } = mountHost(active)
    active.value = true
    await wrapper.vm.$nextTick()
    expect(document.activeElement).toBe(buttons[0])
  })

  it('restores focus to previously focused element on deactivation', async () => {
    const outside = document.createElement('button')
    outside.id = 'outside'
    document.body.appendChild(outside)
    outside.focus()
    expect(document.activeElement).toBe(outside)

    const active = ref(false)
    const { wrapper } = mountHost(active)
    active.value = true
    await wrapper.vm.$nextTick()

    active.value = false
    await wrapper.vm.$nextTick()
    expect(document.activeElement).toBe(outside)

    outside.remove()
  })

  it('wraps Tab from last to first focusable element', async () => {
    const active = ref(true)
    const { wrapper, buttons } = mountHost(active)
    await wrapper.vm.$nextTick()
    buttons[2].focus()
    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true })
    document.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(true)
    expect(document.activeElement).toBe(buttons[0])
  })

  it('wraps Shift+Tab from first to last focusable element', async () => {
    const active = ref(true)
    const { wrapper, buttons } = mountHost(active)
    await wrapper.vm.$nextTick()
    buttons[0].focus()
    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    })
    document.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(true)
    expect(document.activeElement).toBe(buttons[2])
  })

  it('does nothing when active is false', async () => {
    const active = ref(false)
    const { wrapper, buttons } = mountHost(active)
    await wrapper.vm.$nextTick()
    buttons[2].focus()
    const event = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true })
    document.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(false)
  })
})
