/**
 * AppErrorBoundary.vue (P11c):
 * - slot 内の子が throw すると AppErrorFallback に置き換わる (伝播停止)
 * - reload で @tauri-apps/plugin-process の relaunch() が呼ばれる
 * - dismiss (続行を試す) で slot が再マウントされる
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h } from 'vue'

vi.mock('~/composables/useI18n', async () => {
  const ja = (await import('~/locales/ja')).default
  function t(key: string): string {
    const v = key
      .split('.')
      .reduce<unknown>(
        (o, p) =>
          typeof o === 'object' && o !== null ? (o as Record<string, unknown>)[p] : undefined,
        ja,
      )
    return typeof v === 'string' ? v : key
  }
  return { useI18n: () => ({ t, locale: { value: 'ja' } }) }
})

vi.mock('~/composables/useAppInfo', () => ({
  useAppInfo: () => ({ info: { value: null }, load: vi.fn() }),
}))

const relaunchMock = vi.fn().mockResolvedValue(undefined)
vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: (...a: unknown[]) => relaunchMock(...a),
}))

import AppErrorBoundary from '../AppErrorBoundary.vue'
import { useAppErrorState } from '~/composables/useAppErrorState'

describe('AppErrorBoundary', () => {
  // マウント済みラッパは破棄する。破棄しないと singleton リセット時に
  // 旧テストの Thrower が再マウント → 再 throw → capture が古いエラーを保持し、
  // 後続テストの fallback 表示が汚染される。
  let mounted: VueWrapper[] = []
  afterEach(() => {
    for (const w of mounted) w.unmount()
    mounted = []
  })

  beforeEach(() => {
    useAppErrorState().__resetForTests()
    relaunchMock.mockReset()
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  it('renders the fallback when a slotted child throws', async () => {
    const Thrower = defineComponent({
      setup() {
        throw new Error('boom')
      },
      render: () => h('div'),
    })
    const w = mount(AppErrorBoundary, {
      slots: { default: Thrower },
    })
    mounted.push(w)
    await flushPromises()
    // フォールバック (errorBoundary.title) が描かれ、元の子は消える
    expect(w.text()).toContain('この画面で問題が発生しました')
    expect(w.text()).toContain('boom')
    expect(w.attributes('role')).toBeUndefined()
    expect(w.find('[role="alert"]').exists()).toBe(true)
    expect(console.error).toHaveBeenCalled()
  })

  it('reload calls Tauri relaunch()', async () => {
    const Thrower = defineComponent({
      setup() {
        throw new Error('boom')
      },
      render: () => h('div'),
    })
    const w = mount(AppErrorBoundary, {
      slots: { default: Thrower },
    })
    mounted.push(w)
    await flushPromises()
    const buttons = w.findAll('button')
    // [詳細をコピー] [アプリを再起動] [続行を試す] の順
    await buttons[1]?.trigger('click')
    await flushPromises()
    expect(relaunchMock).toHaveBeenCalledTimes(1)
  })

  it('dismiss remounts the slot content', async () => {
    let shouldThrow = true
    const Flaky = defineComponent({
      setup() {
        if (shouldThrow) {
          shouldThrow = false
          throw new Error('first boom')
        }
        return () => h('div', { 'data-test': 'recovered' }, 'recovered')
      },
    })
    const w = mount(AppErrorBoundary, {
      slots: { default: Flaky },
    })
    mounted.push(w)
    await flushPromises()
    expect(w.text()).toContain('first boom')

    const buttons = w.findAll('button')
    await buttons[2]?.trigger('click')
    await flushPromises()
    expect(w.find('[data-test="recovered"]').exists()).toBe(true)
    expect(w.find('[role="alert"]').exists()).toBe(false)
  })
})
