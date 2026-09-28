/**
 * useAppErrorState.ts の契約 (P11c):
 * - toCapturedError() が Error / string / NuxtError 風オブジェクトを正規化する
 * - capture() は最初のエラーのみ保持する (連鎖エラーで上書きしない)
 * - clear() (__resetForTests) で初期状態に戻る
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'

import { toCapturedError, useAppErrorState } from '../useAppErrorState'

describe('toCapturedError', () => {
  it('normalizes an Error with message and stack', () => {
    const cap = toCapturedError(new Error('boom'), 'render', 'setup()')
    expect(cap.message).toBe('boom')
    expect(cap.stack).toContain('boom')
    expect(cap.source).toBe('render')
    expect(cap.info).toBe('setup()')
    expect(typeof cap.at).toBe('string')
  })

  it('normalizes a plain string without stack', () => {
    const cap = toCapturedError('plain failure', 'window.error')
    expect(cap.message).toBe('plain failure')
    expect(cap.stack).toBeNull()
    expect(cap.info).toBeNull()
  })

  it('picks message/stack from NuxtError-like plain objects', () => {
    const cap = toCapturedError(
      { message: 'Page not found', statusCode: 404, stack: 'at server' },
      'nuxt',
      '404',
    )
    expect(cap.message).toBe('Page not found')
    expect(cap.stack).toBe('at server')
    expect(cap.info).toBe('404')
  })

  it('truncates overlong message and stack', () => {
    const cap = toCapturedError({ message: 'x'.repeat(3000), stack: 'y'.repeat(9000) }, 'render')
    expect(cap.message.length).toBeLessThanOrEqual(2001)
    expect(cap.stack?.length).toBeLessThanOrEqual(8001)
  })
})

describe('useAppErrorState', () => {
  beforeEach(() => {
    useAppErrorState().__resetForTests()
    vi.restoreAllMocks()
  })

  it('capture() keeps only the first error', () => {
    const { error, capture } = useAppErrorState()
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    capture(new Error('first'), 'render')
    capture(new Error('second'), 'vue:error')
    expect(error.value?.message).toBe('first')
    expect(spy).toHaveBeenCalled()
  })

  it('clear() resets to the initial state', () => {
    const api = useAppErrorState()
    vi.spyOn(console, 'error').mockImplementation(() => {})
    api.capture(new Error('x'), 'render')
    expect(api.error.value).not.toBeNull()
    api.clear()
    expect(api.error.value).toBeNull()
  })
})
