/**
 * withProgressJob: listen → invoke → unlisten ライフサイクルの単体テスト。
 *
 * 検証ポイント:
 *  - listen が invoke より **前** に張られる (race 回避)
 *  - 成功 / reject いずれの経路でも unlisten が 1 回だけ呼ばれる
 *  - busy / currentJobId が正しくリセットされる
 *  - cancel() 呼び出し後に invoke が reject した場合、cancelledJobId を見て
 *    BulkImportCancelledError に翻訳される (typed error)
 *  - onCancelled フックが呼ばれる
 *  - 進捗イベントは jobId フィルタされる (他ジョブの progress を無視)
 *  - 連続呼び出しで前ジョブの状態がリークしない
 *  - 進捗イベントがなくても invoke は成功する
 *  - BulkImportCancelledError がモジュールローカル identity を保つ
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { createProgressJobRunner, BulkImportCancelledError } from '../withProgressJob'

interface FakeProgress {
  jobId: string
  stage: 'scan' | 'parse' | 'extract' | 'done' | 'error' | 'cancelled'
  current: number
  total: number
  message: string | null
}

describe('withProgressJob (createProgressJobRunner)', () => {
  let listenMock: ReturnType<typeof vi.fn>
  let invokeMock: ReturnType<typeof vi.fn>

  beforeEach(() => {
    listenMock = vi.fn()
    invokeMock = vi.fn()
  })

  // 標準的な listen / invoke モックを差し込んで runner を生成するヘルパ。
  function makeRunner(opts: {
    listenImpl?: <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>
    invokeImpl?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T | null>
    buildInvoke?: (jobId: string) => { command: string; args: Record<string, unknown> }
    jobIdPrefix?: string
    newJobId?: () => string
    unwrap?: <T>(raw: T | null) => T | null
    onCancelled?: () => void
  }) {
    return createProgressJobRunner(
      {
        jobIdPrefix: opts.jobIdPrefix ?? 'bulk',
        buildInvoke: opts.buildInvoke ?? ((jobId) => ({ command: 'do_thing', args: { jobId } })),
        unwrap: opts.unwrap,
        onCancelled: opts.onCancelled,
      },
      {
        listenFn: opts.listenImpl ?? listenMock as <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>,
        invokeFn: opts.invokeImpl ?? invokeMock as <T>(cmd: string, args?: Record<string, unknown>) => Promise<T | null>,
        newJobId: opts.newJobId,
      },
    )
  }

  it('listen が invoke より前に張られる (race 回避)', async () => {
    const order: string[] = []
    listenMock.mockImplementation(
      async (_evt: string, cb: (e: { payload: FakeProgress }) => void) => {
        order.push('listen')
        cb({ payload: { jobId: 'j-1', stage: 'parse', current: 0, total: 1, message: null } })
        return () => order.push('unlisten')
      },
    )
    invokeMock.mockImplementation(async () => {
      order.push('invoke')
      return { ok: true }
    })

    const runner = makeRunner({ newJobId: () => 'j-1' })

    await runner.handle()

    expect(order[0]).toBe('listen')
    expect(order).toContain('invoke')
    expect(order[order.length - 1]).toBe('unlisten')
    expect(order.indexOf('listen')).toBeLessThan(order.indexOf('invoke'))
    expect(order.indexOf('invoke')).toBeLessThan(order.indexOf('unlisten'))
  })

  it('成功時に unlisten が 1 回だけ呼ばれる', async () => {
    const unlistenSpy = vi.fn()
    listenMock.mockResolvedValue(unlistenSpy)
    invokeMock.mockResolvedValue({ ok: true })

    const runner = makeRunner({})
    await runner.handle()
    expect(unlistenSpy).toHaveBeenCalledTimes(1)
  })

  it('reject 時も unlisten が 1 回だけ呼ばれる', async () => {
    const unlistenSpy = vi.fn()
    listenMock.mockResolvedValue(unlistenSpy)
    invokeMock.mockRejectedValue(new Error('boom'))

    const runner = makeRunner({})
    await expect(runner.handle()).rejects.toThrow('boom')
    expect(unlistenSpy).toHaveBeenCalledTimes(1)
  })

  it('完了後に busy / currentJobId がリセットされる', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValue({ ok: true })

    const runner = makeRunner({ newJobId: () => 'j-busy' })

    expect(runner.busy.value).toBe(false)
    expect(runner.currentJobId.value).toBe(null)

    const p = runner.handle()
    expect(runner.busy.value).toBe(true)
    expect(runner.currentJobId.value).toBe('j-busy')
    await p

    expect(runner.busy.value).toBe(false)
    expect(runner.currentJobId.value).toBe(null)
  })

  it('cancel() 後の reject は BulkImportCancelledError に翻訳される', async () => {
    // 実フローの再現: handle 開始 → listen 完了 → invoke 中に cancel() が
    // currentJobId → cancelledJobId に書き込む → invoke reject → handle 内 catch で
    // cancelledJobId を見て翻訳。
    //
    // cancel() は内部で invokeCommand('cancel_bulk_import', ...) を呼ぶため、
    // invokeMock 側で cancel_bulk_import を no-op として扱う必要がある。
    // さらに capture 順序: invokeMock が先に do_thing で入り、その中で
    // runner.cancel() を呼んで cancel_bulk_import を解決してから reject する。
    const onCancelled = vi.fn()
    let capturedRunner: ReturnType<typeof makeRunner> | null = null
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'cancel_bulk_import') return null
      // do_thing: まず cancel() を呼んで cancelledJobId を立てる → reject
      await capturedRunner!.cancel()
      throw new Error('cancel-from-rust')
    })
    listenMock.mockResolvedValue(() => {})

    capturedRunner = makeRunner({ newJobId: () => 'j-cancel-flow', onCancelled })
    await expect(capturedRunner.handle()).rejects.toBeInstanceOf(BulkImportCancelledError)
    expect(onCancelled).toHaveBeenCalledTimes(1)
  })

  it('cancel していない invoke の reject はそのまま伝播する', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockRejectedValue(new Error('real-error'))

    const runner = makeRunner({})
    await expect(runner.handle()).rejects.toThrow('real-error')
  })

  it('onCancelled コールバックが cancelled 翻訳パスで発火する', async () => {
    let capturedRunner: ReturnType<typeof makeRunner> | null = null
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'cancel_bulk_import') return null
      await capturedRunner!.cancel()
      throw new Error('cancel-from-rust')
    })
    listenMock.mockResolvedValue(() => {})
    const onCancelled = vi.fn()
    capturedRunner = makeRunner({ newJobId: () => 'j-on-cancel', onCancelled })

    await expect(capturedRunner.handle()).rejects.toBeInstanceOf(BulkImportCancelledError)
    expect(onCancelled).toHaveBeenCalledTimes(1)
  })

  it('progress イベントは jobId フィルタされる (他ジョブは無視)', async () => {
    let captured: ((e: { payload: FakeProgress }) => void) | null = null
    listenMock.mockImplementation(
      async (_evt: string, cb: (e: { payload: FakeProgress }) => void) => {
        captured = cb
        return () => {}
      },
    )
    invokeMock.mockResolvedValue({ ok: true })

    const runner = makeRunner({ newJobId: () => 'mine' })

    const p = runner.handle()
    captured?.({
      payload: { jobId: 'someone-else', stage: 'parse', current: 1, total: 1, message: null },
    })
    captured?.({
      payload: { jobId: 'mine', stage: 'parse', current: 1, total: 1, message: null },
    })
    await p
    expect(runner.progress.value?.jobId).toBe('mine')
  })

  it('連続した 2 回の handle() 呼び出しで状態がリークしない', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValueOnce({ ok: 1 }).mockResolvedValueOnce({ ok: 2 })

    let n = 0
    const runner = makeRunner({
      newJobId: () => `j-${++n}`,
      unwrap: (raw) => raw as { ok: number } | null,
    })

    const r1 = (await runner.handle()) as { ok: number } | null
    const r2 = (await runner.handle()) as { ok: number } | null

    expect(r1?.ok).toBe(1)
    expect(r2?.ok).toBe(2)
    expect(runner.busy.value).toBe(false)
    expect(runner.currentJobId.value).toBe(null)
  })

  it('進捗イベントが発火しなくても handle() は成功する (no-op progress path)', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValue({ ok: true })

    const runner = makeRunner({})
    const r = await runner.handle()
    expect((r as { ok: boolean } | null)?.ok).toBe(true)
    expect(runner.progress.value).toBe(null)
  })

  it('BulkImportCancelledError が同じ identity で re-export される', () => {
    const e = new BulkImportCancelledError()
    expect(e.name).toBe('BulkImportCancelledError')
    expect(e.message).toBe('bulk import cancelled')
    expect(e).toBeInstanceOf(BulkImportCancelledError)
    expect(e).toBeInstanceOf(Error)
  })

  it('unwrap が null を返した場合は handle() も null を返す', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValue(null)
    const runner = makeRunner({ unwrap: () => null })
    expect(await runner.handle()).toBe(null)
  })

  it('cancel() が currentJobId 未設定のときは何もしない', async () => {
    const runner = makeRunner({})
    await runner.cancel()
    expect(invokeMock).not.toHaveBeenCalled()
  })

  it('cancel() が invoke をキャンセルIPC へ委譲する', async () => {
    listenMock.mockResolvedValue(() => {})
    invokeMock.mockResolvedValue({ ok: true })

    const runner = makeRunner({ newJobId: () => 'j-cmd' })
    const p = runner.handle()
    await runner.cancel()
    expect(invokeMock).toHaveBeenCalledWith('cancel_bulk_import', { jobId: 'j-cmd' })
    await p
  })
})