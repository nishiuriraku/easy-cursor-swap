/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref, type Ref } from 'vue'
import { mount, type VueWrapper } from '@vue/test-utils'
import { useModalLifecycle } from '../useModalLifecycle'

// 各テストで生成した wrapper をここで一元管理し、afterEach で必ず unmount する。
// useModalLifecycle はモジュールスコープの lockCounter を持つので、unmount 漏れが
// あると次テストに lock 状態が leak する (counter が 0 にならず activate しても
// body.overflow が変わらないバグを誘発する)。
const wrappers: VueWrapper[] = []

afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
  document.body.style.overflow = ''
})

function mountHost(open: Ref<boolean>, onClose: () => void = () => {}): VueWrapper {
  const Host = defineComponent({
    setup() {
      useModalLifecycle({ open, onClose })
      return () => h('div', open.value ? 'open' : 'closed')
    },
  })
  const w = mount(Host)
  wrappers.push(w)
  return w
}

describe('useModalLifecycle', () => {
  it('locks body scroll when open and restores when closed', async () => {
    document.body.style.overflow = 'auto'
    const open = ref(false)
    const wrapper = mountHost(open)

    expect(document.body.style.overflow).toBe('auto')
    open.value = true
    await wrapper.vm.$nextTick()
    expect(document.body.style.overflow).toBe('hidden')
    open.value = false
    await wrapper.vm.$nextTick()
    expect(document.body.style.overflow).toBe('auto')
  })

  it('calls onClose on Escape keydown', async () => {
    const open = ref(true)
    const onClose = vi.fn()
    mountHost(open, onClose)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not call onClose on non-Escape keys', () => {
    const open = ref(true)
    const onClose = vi.fn()
    mountHost(open, onClose)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))
    expect(onClose).not.toHaveBeenCalled()
  })

  it('stacks scroll lock correctly across two modals', async () => {
    document.body.style.overflow = ''
    const aOpen = ref(false)
    const bOpen = ref(false)
    mountHost(aOpen)
    mountHost(bOpen)

    aOpen.value = true
    await Promise.resolve()
    expect(document.body.style.overflow).toBe('hidden')

    bOpen.value = true
    await Promise.resolve()
    expect(document.body.style.overflow).toBe('hidden')

    // B を先に閉じても、A がまだ開いているので lock は維持される
    bOpen.value = false
    await Promise.resolve()
    expect(document.body.style.overflow).toBe('hidden')

    aOpen.value = false
    await Promise.resolve()
    expect(document.body.style.overflow).toBe('')
  })

  it('unlocks body scroll on unmount even if still open', async () => {
    document.body.style.overflow = 'auto'
    const open = ref(true)
    const wrapper = mountHost(open)
    await wrapper.vm.$nextTick()
    expect(document.body.style.overflow).toBe('hidden')
    wrapper.unmount()
    // unmount 後は本テストの管理対象から外す (afterEach の重複 unmount 防止)
    const i = wrappers.indexOf(wrapper)
    if (i >= 0) wrappers.splice(i, 1)
    expect(document.body.style.overflow).toBe('auto')
  })
})
