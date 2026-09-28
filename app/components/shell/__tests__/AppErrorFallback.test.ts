/**
 * AppErrorFallback.vue (P11c):
 * - role="alert" (UiAlert) で詳細 <pre> に message + stack を含める
 * - コピーで navigator.clipboard.writeText が呼ばれ、成功表示が出る
 * - reload / dismiss を emit する (showDismiss が無いとき dismiss ボタンは無い)
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

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
  useAppInfo: () => ({
    info: {
      value: { version: '0.0.8', cursors_dir: '', config_dir: '', os_version: '' },
    },
    load: vi.fn(),
  }),
}))

import AppErrorFallback from '../AppErrorFallback.vue'
import type { CapturedError } from '~/composables/useAppErrorState'

const ERROR: CapturedError = {
  message: 'boom happened',
  stack: 'Error: boom happened\n    at setup',
  source: 'render',
  info: 'setup()',
  at: '2026-09-28T00:00:00.000Z',
}

function buttons(w: ReturnType<typeof mount>) {
  return w.findAll('button')
}

describe('AppErrorFallback', () => {
  beforeEach(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    })
  })

  it('renders role="alert" with message, stack and version in the detail block', () => {
    const w = mount(AppErrorFallback, { props: { error: ERROR } })
    expect(w.attributes('role')).toBe('alert')
    const pre = w.find('pre')
    expect(pre.text()).toContain('boom happened')
    expect(pre.text()).toContain('at setup')
    expect(pre.text()).toContain('[EasyCursorSwap v0.0.8]')
    expect(pre.text()).toContain('render')
  })

  it('copies the detail text and shows the copied label', async () => {
    const w = mount(AppErrorFallback, { props: { error: ERROR } })
    const writeText = navigator.clipboard.writeText as unknown as ReturnType<typeof vi.fn>
    await buttons(w)[0]?.trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledTimes(1)
    const copied = String(writeText.mock.calls[0]?.[0] ?? '')
    expect(copied).toContain('boom happened')
    expect(copied).toContain('[EasyCursorSwap v0.0.8]')
    // `common.copied` (コピー完了) がボタンに表示される
    expect(buttons(w)[0]?.text()).toContain('コピー完了')
  })

  it('emits reload, and dismiss only when showDismiss', async () => {
    const w = mount(AppErrorFallback, { props: { error: ERROR } })
    expect(buttons(w)).toHaveLength(2)
    await buttons(w)[1]?.trigger('click')
    expect(w.emitted('reload')).toHaveLength(1)

    const w2 = mount(AppErrorFallback, { props: { error: ERROR, showDismiss: true } })
    expect(buttons(w2)).toHaveLength(3)
    await buttons(w2)[2]?.trigger('click')
    expect(w2.emitted('dismiss')).toHaveLength(1)
  })
})
