/**
 * useBulkImport.resolveAssets のキャンセル翻訳を検証する。
 *
 * バックエンド (bulk_resolve_assets) はユーザーがキャンセルすると
 * AppError::BulkImportCancelled で reject する。resolveAssets はこの生 reject を
 * 「cancel() を呼んだジョブか」で判定し、typed な BulkImportCancelledError に
 * 翻訳しなければならない (呼び出し側が「失敗」表示と区別できるようにするため)。
 *
 * `BulkImportCancelledError` は canonical に `withProgressJob` で定義されており、
 * ここでは re-export ではなく直接 canonical から import する。vitest の
 * auto-import がモジュールローカルとグローバル 2 つの identity を解決する経路で
 * TDZ になる事例があるため、テストの import 経路は単一に保つ (Wave 2B / Task 7)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
const listenTauriMock = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
  listenTauri: (...args: unknown[]) => listenTauriMock(...args),
}))

import { useBulkImport } from '../useBulkImport'
import { BulkImportCancelledError } from '../withProgressJob'

describe('useBulkImport.resolveAssets cancellation', () => {
  beforeEach(() => {
    invoke.mockReset()
    listenTauriMock.mockReset()
    // withProgressJob は listenTauri を await するので no-op unlisten を返す。
    listenTauriMock.mockResolvedValue(() => {})
  })

  it('cancel() 後に bulk_resolve_assets が reject すると BulkImportCancelledError を投げる', async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'cancel_bulk_import') return Promise.resolve(null)
      if (cmd === 'bulk_resolve_assets')
        return Promise.reject(new Error('一括インポートが中断されました'))
      return Promise.resolve(null)
    })

    const bulk = useBulkImport()
    const p = bulk.resolveAssets(['C:/x'], false)
    // p が reject するのを一旦受け止めて vitest の unhandled-rejection tracker が
    // 先に拾わないようにし、後段の expect で意図的に再アサートする。
    p.catch(() => {})
    await bulk.cancel()
    await expect(p).rejects.toBeInstanceOf(BulkImportCancelledError)
  })

  it('cancel していないときの reject はそのまま伝播する', async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'bulk_resolve_assets') return Promise.reject(new Error('boom'))
      return Promise.resolve(null)
    })

    const bulk = useBulkImport()
    await expect(bulk.resolveAssets(['C:/x'], false)).rejects.toThrow('boom')
  })
})
