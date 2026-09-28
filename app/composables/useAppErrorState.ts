/**
 * 描画エラーの singleton 集約 (P11c エラーバウンダリ)。
 *
 * 3 層で使う単一の真実:
 * - `AppErrorBoundary.vue` (`onErrorCaptured`, render 由来)
 * - `plugins/errorBoundary.client.ts` (`vue:error` / `window.error` / `unhandledrejection`)
 * - `app/error.vue` (Nuxt 致命エラー)
 *
 * 最初のエラーだけ保持する (連鎖エラーでフォールバック内容が上書きされないように)。
 * Rust の `crash.rs` (panic 保存) への転送はしない。新 IPC が必要になるため
 * task.md 起票 + P03 の typed error と一緒に検討 (P11 非スコープ)。
 */
import type { Ref } from 'vue'

export type CapturedErrorSource =
  | 'render'
  | 'vue:error'
  | 'window.error'
  | 'unhandledrejection'
  | 'nuxt'

export interface CapturedError {
  message: string
  stack: string | null
  source: CapturedErrorSource
  info: string | null
  at: string
}

const MESSAGE_LIMIT = 2000
const STACK_LIMIT = 8000

function truncate(s: string, limit: number): string {
  return s.length > limit ? `${s.slice(0, limit)}…` : s
}

/** `unknown` を表示用の `CapturedError` に正規化する。`message` / `stack` は上限で切る。 */
export function toCapturedError(
  err: unknown,
  source: CapturedErrorSource,
  info?: string,
): CapturedError {
  let message: string
  let stack: string | null = null
  if (err instanceof Error) {
    message = err.message || String(err)
    stack = err.stack ?? null
  } else if (typeof err === 'string') {
    message = err
  } else if (
    typeof err === 'object' &&
    err !== null &&
    typeof (err as { message?: unknown }).message === 'string'
  ) {
    // NuxtError (`#app`) は Error インスタンスではない plain object。
    message = (err as { message: string }).message
    const maybeStack = (err as { stack?: unknown }).stack
    stack = typeof maybeStack === 'string' ? maybeStack : null
  } else {
    try {
      message = JSON.stringify(err) ?? String(err)
    } catch {
      message = String(err)
    }
  }
  return {
    message: truncate(message, MESSAGE_LIMIT),
    stack: stack ? truncate(stack, STACK_LIMIT) : null,
    source,
    info: info ?? null,
    at: new Date().toISOString(),
  }
}

const current = ref<CapturedError | null>(null)

export function useAppErrorState(): {
  error: Readonly<Ref<CapturedError | null>>
  capture: (err: unknown, source: CapturedErrorSource, info?: string) => void
  clear: () => void
  __resetForTests: () => void
} {
  function capture(err: unknown, source: CapturedErrorSource, info?: string) {
    if (!current.value) current.value = toCapturedError(err, source, info)
    // eslint-disable-next-line no-console
    console.error('[error-boundary]', source, err)
  }
  function clear() {
    current.value = null
  }
  return { error: readonly(current), capture, clear, __resetForTests: clear }
}
