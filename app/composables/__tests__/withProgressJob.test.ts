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
        listenFn:
          opts.listenImpl ??
          (listenMock as <T>(
            event: string,
            cb: (e: { payload: T }) => void,
          ) => Promise<() => void>),
        invokeFn:
          opts.invokeImpl ??
          (invokeMock as <T>(cmd: string, args?: Record<string, unknown>) => Promise<T | null>),
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

  /**
   * Wave 2AB Task 7 I-1 (parked finding) リグレッション guard。
   *
   * `handle(buildInvoke?)` の per-call closure 受けが race-free であることを検証。
   * 旧実装は `options.buildInvoke` を listenProgress の await を跨いで後で評価するため、
   * 呼び出し側が mutable な closure state (例: `let pendingCommand` / `let pendingArgs`)
   * を読ませると、sibling call が state を上書きする window で他方の args を読み込んで
   * 誤 invoke が発火する race が起きる (Task 7 I-1 parked)。
   *
   * per-call closure なら caller 側のローカル変数を handle() 呼び出し時点でキャプチャ
   * するため、await を跨いでも安全。この test では:
   *  - listenImpl に 1 段 microtask yield を入れ、handle() 内の await listenProgress
   *    で実際に制御が一度戻るように仕掛ける (mockResolvedValue 同期だと race 顕在化
   *    しないので意味がない)
   *  - 2 つの handle() を並行発火、それぞれ別の cmd/args を持つ per-call closure を渡す
   *  - 最終的に invoke が各 closure の cmd/args で 1 回ずつ呼ばれていることを assert
   *
   * 旧 options.buildInvoke のみに依存する design だとここで両 invoke が 2 段目の
   * cmd/args に上書きされて fail。per-call closure なら pass。
   *
   * `runOpts.listenFn` / `invokeFn` を直接渡すことで vitest 4.x の dynamic-import
   * キャッシュによるリアル useTauri 到達 (`transformCallback` undefined) を回避。
   */
  it('handle(buildInvoke?) per-call closure で race-free になる', async () => {
    const listenImpl = vi.fn().mockImplementation(async () => {
      await Promise.resolve()
      return () => {}
    })
    const invokeImpl = vi
      .fn()
      .mockImplementation(async (cmd: string, _args?: Record<string, unknown>) => {
        return { ok: cmd } as { ok: string } | null
      })
    const runner = createProgressJobRunner(
      {
        jobIdPrefix: 'race',
        // per-call form のみで扱う想定。options.buildInvoke に到達したら fail-fast
        // にして、誤ったクロージャ state 読みを regression で検出する。
        buildInvoke: () => {
          throw new Error('per-call form expected — caller must pass buildInvoke to handle()')
        },
      },
      {
        listenFn: listenImpl as <T>(
          event: string,
          cb: (e: { payload: T }) => void,
        ) => Promise<() => void>,
        invokeFn: invokeImpl as <T>(
          cmd: string,
          args?: Record<string, unknown>,
        ) => Promise<T | null>,
        newJobId: (() => {
          let n = 0
          return () => `j-${++n}`
        })(),
      },
    )

    // 1 段目: cmdA / argsA を同期キャプチャした per-call closure
    const cmdA = 'cmd_a'
    const argsA: Record<string, unknown> = { req: 'A' }
    const buildInvokeA = (jobId: string) => ({
      command: cmdA,
      args: { ...argsA, jobId },
    })
    const p1 = runner.handle(buildInvokeA)

    // microtask flush: handle 1 が listenProgress の await に到達した状態にする。
    // ここで yield がないと 2 段目も同期に進行して race window が生まれない。
    await Promise.resolve()

    // 2 段目: cmdB / argsB を同期キャプチャした別の per-call closure
    const cmdB = 'cmd_b'
    const argsB: Record<string, unknown> = { req: 'B' }
    const buildInvokeB = (jobId: string) => ({
      command: cmdB,
      args: { ...argsB, jobId },
    })
    const p2 = runner.handle(buildInvokeB)

    // unhandled rejection 監視を回避するため両方 catch を付ける。
    p1.catch(() => {})
    p2.catch(() => {})
    await Promise.all([p1, p2])

    // listen / invoke それぞれの回数を sanity check
    expect(listenImpl).toHaveBeenCalledTimes(2)
    expect(invokeImpl).toHaveBeenCalledTimes(2)

    // 各 invoke が自分の closure の cmd/args を使っている (race が起きていれば
    // 両方 cmdB / argsB になり cmdA / argsA の呼び出しが消える)
    expect(invokeImpl).toHaveBeenCalledWith(cmdA, expect.objectContaining({ req: 'A' }))
    expect(invokeImpl).toHaveBeenCalledWith(cmdB, expect.objectContaining({ req: 'B' }))
  })
})
