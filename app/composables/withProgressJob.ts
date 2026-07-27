/**
 * IPC ジョブの listen → invoke → unlisten ライフサイクルを共有するヘルパー。
 *
 * `bulk_resolve_assets` / `parse_cursorpack_for_creator` はいずれも
 * 1. job_id を採番
 * 2. `bulk-import-progress` イベントを listen 開始
 * 3. invoke を実行
 * 4. 完了 / 失敗 / キャンセルのいずれでも unlisten + busy リセット
 *
 * という同じライフサイクルを持つ。listen を invoke より **前** に張らないと
 * 「invoke 直後の最初の progress を取りこぼす」 race があるため順序を厳守する。
 *
 * `BulkImportCancelledError` は Canonical にここで定義し、`useBulkImport` から
 * `export { ... } from './withProgressJob'` で re-export する。vitest の
 * auto-import 境界で `instanceof` がグローバルとモジュールローカルで食い違う
 * TDZ 問題を避けるため、import 経路を単一化する (Wave 2B / Task 7)。
 */

import { ref, type Ref } from 'vue'

export class BulkImportCancelledError extends Error {
  constructor() {
    super('bulk import cancelled')
    this.name = 'BulkImportCancelledError'
  }
}

export interface BulkImportProgressLite {
  jobId: string
  stage: 'scan' | 'parse' | 'extract' | 'done' | 'error' | 'cancelled'
  current: number
  total: number
  message: string | null
}

/**
 * 呼び出し側ヘルパーで組み立てる invoke スペック。`buildInvoke` が `(jobId) => ...`
 * を返し、`createProgressJobRunner` がそれを handle() 内で使う。同じ runner を
 * 異なるコマンド (例: bulk_resolve_assets / parse_cursorpack_for_creator) で
 * 再利用したい場合に使う。
 */
export interface InvokeSpec {
  command: string
  args: Record<string, unknown>
}

export interface WithProgressJobOptions {
  /** 進捗イベント名 (既定 `'bulk-import-progress'`) */
  eventName?: string
  /** invoke のスペックを返す。jobId はヘルパーが渡す。 */
  buildInvoke: (jobId: string) => InvokeSpec
  /** invoke の戻り値を生の T に変換する。null を返すと空結果にフォールバック。 */
  unwrap?: <T>(raw: T | null) => T | null
  /** job_id プレフィックス (`bulk-` / `cpack-` 等)。テスト識別用にも使う。 */
  jobIdPrefix: string
  /** 進捗コールバック (jobId でフィルタ済) */
  onProgress?: (p: BulkImportProgressLite) => void
  /** キャンセル時に呼ばれる。BulkImportCancelledError throw 前の最終 hook。 */
  onCancelled?: () => void
}

export interface WithProgressJobRefs {
  busy: Ref<boolean>
  currentJobId: Ref<string | null>
  progress: Ref<BulkImportProgressLite | null>
  /** 直近に cancel() が要求された job_id。resolveAssets の reject 翻訳に使う。 */
  cancelledJobId: Ref<string | null>
  cancel: () => Promise<void>
}

export interface RunOptions {
  /** Tauri listen モック用 */
  listenFn?: <T>(event: string, cb: (e: { payload: T }) => void) => Promise<() => void>
  /** Tauri invoke モック用 */
  invokeFn?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T | null>
  /** テスト用: jobId の生成を上書きする (決定論的テスト用) */
  newJobId?: () => string
}

const DEFAULT_EVENT = 'bulk-import-progress'

/**
 * listen → invoke → unlisten のライフサイクルを一つに束ねるヘルパーを生成する。
 * 戻り値の `handle()` を呼ぶたびに新しい job_id を採番して invoke する。
 * `refs.busy` / `refs.currentJobId` / `refs.progress` は内部で自動更新される。
 *
 * 同じ runner インスタンスを複数回呼び出す場合、`busy` / `currentJobId` /
 * `cancelledJobId` は直近の呼び出しのものを保持する (UI から「いま走っている
 * ジョブ」を一元参照できる)。`handle()` 実行中は `cancelledJobId` がクリアされ、
 * `cancel()` でジョブ中断要求が記録される。
 *
 * `RunOptions` を介して Tauri 依存を差し替えられるので vitest から listen/invoke
 * を mock できる (`app/composables/__tests__/withProgressJob.test.ts`)。
 */
export function createProgressJobRunner(
  options: WithProgressJobOptions,
  runOpts: RunOptions = {},
): { handle: () => Promise<unknown | null> } & WithProgressJobRefs {
  const busy = ref(false)
  const currentJobId = ref<string | null>(null)
  const progress = ref<BulkImportProgressLite | null>(null)
  const cancelledJobId = ref<string | null>(null)

  const eventName = options.eventName ?? DEFAULT_EVENT

  function defaultNewJobId(): string {
    return `${options.jobIdPrefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  }
  const newJobId = runOpts.newJobId ?? defaultNewJobId

  async function listenProgress(jobId: string): Promise<() => void> {
    if (runOpts.listenFn) {
      return await runOpts.listenFn<BulkImportProgressLite>(eventName, (e) => {
        if (e.payload.jobId === jobId) {
          progress.value = e.payload
          options.onProgress?.(e.payload)
        }
      })
    }
    const { listenTauri } = await import('./useTauri')
    return await listenTauri<BulkImportProgressLite>(eventName, (e) => {
      if (e.payload.jobId === jobId) {
        progress.value = e.payload
        options.onProgress?.(e.payload)
      }
    })
  }

  async function invokeCommand<T>(cmd: string, args: Record<string, unknown>): Promise<T | null> {
    if (runOpts.invokeFn) {
      return await runOpts.invokeFn<T>(cmd, args)
    }
    const { invokeTauri } = await import('./useTauri')
    return await invokeTauri<T>(cmd, args)
  }

  async function handle(): Promise<unknown | null> {
    busy.value = true
    progress.value = null
    const jobId = newJobId()
    currentJobId.value = jobId
    cancelledJobId.value = null

    // listen を invoke より **前** に張る (race 回避: invoke 直後の最初の
    // progress を取りこぼさないため)。unlisten は必ず finally で 1 回だけ呼ぶ。
    const unlisten = await listenProgress(jobId)
    try {
      const spec = options.buildInvoke(jobId)
      const raw = await invokeCommand<unknown>(spec.command, spec.args)
      const result = options.unwrap ? options.unwrap(raw) : raw
      if (result === null || result === undefined) {
        return null
      }
      return result
    } catch (err) {
      // キャンセル要求があったかチェック。あれば typed な cancelled エラーに翻訳。
      if (cancelledJobId.value === jobId) {
        options.onCancelled?.()
        throw new BulkImportCancelledError()
      }
      throw err
    } finally {
      unlisten()
      currentJobId.value = null
      busy.value = false
    }
  }

  async function cancel(): Promise<void> {
    if (!currentJobId.value) return
    cancelledJobId.value = currentJobId.value
    try {
      await invokeCommand<unknown>('cancel_bulk_import', { jobId: currentJobId.value })
    } catch {
      // ignore: 既に完了 / 登録抹消済みでも cancel 自体は失敗しない設計
    }
  }

  return {
    handle,
    busy,
    currentJobId,
    progress,
    cancelledJobId,
    cancel,
  }
}