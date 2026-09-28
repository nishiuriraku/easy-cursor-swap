/**
 * `app/utils/appError.ts` のテスト (P03 Step 9)。
 *
 * DTO → AppInvokeError 正規化、レガシー Error/文字列フォールバック、
 * `appErrorMessage` の i18n 解決・フォールバック、冪等性を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { AppInvokeError, appErrorMessage, toAppError } from '../appError'
import { useI18n } from '~/composables/useI18n'

beforeEach(() => {
  const { setLocale } = useI18n()
  setLocale('en')
})

describe('toAppError', () => {
  it('normalizes DTO objects preserving code/message/detail', () => {
    const e = toAppError({
      code: 'registry',
      message: 'レジストリエラー: x',
      detail: { path: 'C:\\a' },
    })
    expect(e).toBeInstanceOf(AppInvokeError)
    expect(e.code).toBe('registry')
    expect(e.message).toBe('レジストリエラー: x')
    expect(e.detail).toEqual({ path: 'C:\\a' })
    expect(Object.isFrozen(e.detail)).toBe(true)
  })

  it("maps Error to code 'other' with identical message", () => {
    const e = toAppError(new Error('crypto: DPAPI 失敗'))
    expect(e.code).toBe('other')
    expect(e.message).toBe('crypto: DPAPI 失敗')
  })

  it("maps strings to code 'other'", () => {
    expect(toAppError('boom').message).toBe('boom')
  })

  it('stringifies undefined/numbers', () => {
    expect(toAppError(undefined).message).toBe('undefined')
    expect(toAppError(42).message).toBe('42')
  })

  it('is idempotent', () => {
    const once = toAppError({ code: 'theme', message: 'm' })
    expect(toAppError(once)).toBe(once)
  })
})

describe('appErrorMessage', () => {
  it('interpolates errors.oversize_file in en', () => {
    const msg = appErrorMessage({
      code: 'oversize_file',
      message: 'x',
      detail: { path: '/a.p', size: '11' },
    })
    expect(msg).toContain('/a.p')
    expect(msg).toContain('11')
  })

  it('passes through message for errors.other', () => {
    expect(appErrorMessage(new Error('raw text'))).toBe('raw text')
  })

  it('falls back to message for unknown codes', () => {
    expect(appErrorMessage({ code: 'zzz', message: 'mystery' } as never)).toBe('mystery')
  })
})
