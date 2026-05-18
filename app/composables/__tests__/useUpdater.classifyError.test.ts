import { describe, expect, it } from 'vitest'
import { classifyUpdaterError } from '../useUpdater'

describe('classifyUpdaterError', () => {
  it('reqwest の canonical なネット切断メッセージを network に分類する', () => {
    // Tauri Updater (reqwest) が Wi-Fi 切断時などに実際に投げる文字列。
    // 「error sending request for url (...)」が canonical なので、これを
    // 取りこぼすと UI が「予期せぬエラー」表示にフォールバックしてしまう。
    const err = new Error(
      'error sending request for url (https://github.com/nishiuriraku/easy-cursor-swap/releases/latest/download/latest.json)',
    )
    expect(classifyUpdaterError(err)).toEqual({
      key: 'settings.updaterErrNetwork',
      message: expect.stringContaining('error sending request'),
    })
  })

  it('一般的な network ワードでも分類できる', () => {
    expect(classifyUpdaterError(new Error('network error')).key).toBe('settings.updaterErrNetwork')
    expect(classifyUpdaterError(new Error('fetch failed')).key).toBe('settings.updaterErrNetwork')
    expect(classifyUpdaterError(new Error('request timeout')).key).toBe(
      'settings.updaterErrNetwork',
    )
    expect(classifyUpdaterError(new Error('ECONNREFUSED 127.0.0.1:80')).key).toBe(
      'settings.updaterErrNetwork',
    )
    expect(classifyUpdaterError(new Error('AbortError: signal aborted')).key).toBe(
      'settings.updaterErrNetwork',
    )
    expect(classifyUpdaterError(new Error('tcp connect error')).key).toBe(
      'settings.updaterErrNetwork',
    )
    expect(classifyUpdaterError(new Error('dns resolution failed')).key).toBe(
      'settings.updaterErrNetwork',
    )
  })

  it('signature 系を signature に分類する', () => {
    expect(classifyUpdaterError(new Error('signature verify failed')).key).toBe(
      'settings.updaterErrSignature',
    )
    expect(classifyUpdaterError(new Error('untrusted comment mismatch')).key).toBe(
      'settings.updaterErrSignature',
    )
    expect(classifyUpdaterError(new Error('minisign: bad pubkey')).key).toBe(
      'settings.updaterErrSignature',
    )
  })

  it('plugin 未利用環境を plugin に分類する', () => {
    expect(classifyUpdaterError(new Error('@tauri-apps/plugin-updater not available')).key).toBe(
      'settings.updaterErrPlugin',
    )
    expect(classifyUpdaterError(new Error('module not found')).key).toBe(
      'settings.updaterErrPlugin',
    )
  })

  it('不明なエラーは unknown にフォールバック', () => {
    expect(classifyUpdaterError(new Error('something else')).key).toBe('settings.updaterErrUnknown')
    expect(classifyUpdaterError('plain string error').key).toBe('settings.updaterErrUnknown')
    expect(classifyUpdaterError(null).key).toBe('settings.updaterErrUnknown')
  })

  it('Error 以外を string にして処理する', () => {
    const result = classifyUpdaterError({ toString: () => 'network down' })
    expect(result.key).toBe('settings.updaterErrNetwork')
    expect(result.message).toBe('network down')
  })
})
