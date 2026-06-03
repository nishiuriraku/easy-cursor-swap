/**
 * useBulkImport.resolveAssets のキャンセル翻訳を検証する。
 *
 * バックエンド (bulk_resolve_assets) はユーザーがキャンセルすると
 * AppError::BulkImportCancelled で reject する。resolveAssets はこの生 reject を
 * 「cancel() を呼んだジョブか」で判定し、typed な BulkImportCancelledError に
 * 翻訳しなければならない (呼び出し側が「失敗」表示と区別できるようにするため)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

const unlistenSpy = vi.fn()
vi.mock('@tauri-apps/api/event', () => ({
  listen: () => Promise.resolve(unlistenSpy),
}))

import { useBulkImport, BulkImportCancelledError } from '../useBulkImport'

describe('useBulkImport.resolveAssets cancellation', () => {
  beforeEach(() => {
    invoke.mockReset()
    unlistenSpy.mockReset()
  })

  it('cancel() 後に bulk_resolve_assets が reject すると BulkImportCancelledError を投げる', async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'cancel_bulk_import') return Promise.resolve()
      if (cmd === 'bulk_resolve_assets')
        return Promise.reject(new Error('一括インポートが中断されました'))
      return Promise.resolve()
    })

    const bulk = useBulkImport()
    const p = bulk.resolveAssets(['C:/x'], false)
    await bulk.cancel()
    await expect(p).rejects.toBeInstanceOf(BulkImportCancelledError)
  })

  it('cancel していないときの reject はそのまま伝播する', async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'bulk_resolve_assets') return Promise.reject(new Error('boom'))
      return Promise.resolve()
    })

    const bulk = useBulkImport()
    await expect(bulk.resolveAssets(['C:/x'], false)).rejects.toThrow('boom')
  })
})
