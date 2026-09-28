/**
 * 境界の外 (レイアウト / シェル) で起きた例外の集約 (P11c)。
 *
 * `clickOutside.client.ts` と同形の `defineNuxtPlugin`。`nuxtApp.hook('vue:error')` が
 * 非同期・イベントハンドラ由来の例外を、`window` の `error` / `unhandledrejection`
 * がそれ以外の到達漏れを `useAppErrorState` に集約する。
 * `AppErrorBoundary` が `return false` で止めた例外は `vue:error` に届かないので
 * 二重登録にならない。
 *
 * window リスナ本体はテスト可能な純関数 `installGlobalErrorListeners` として export
 * (`errorBoundary.plugin.test.ts` から直接検証する)。
 */
import type { CapturedErrorSource } from '~/composables/useAppErrorState'

export type GlobalErrorSource = Extract<
  CapturedErrorSource,
  'window.error' | 'unhandledrejection' | 'vue:error'
>

export function installGlobalErrorListeners(
  capture: (err: unknown, source: GlobalErrorSource, info?: string) => void,
): () => void {
  const onError = (e: ErrorEvent) => capture(e.error ?? e.message, 'window.error')
  const onRejection = (e: PromiseRejectionEvent) => capture(e.reason, 'unhandledrejection')
  window.addEventListener('error', onError)
  window.addEventListener('unhandledrejection', onRejection)
  return () => {
    window.removeEventListener('error', onError)
    window.removeEventListener('unhandledrejection', onRejection)
  }
}

export default defineNuxtPlugin((nuxtApp) => {
  const { capture } = useAppErrorState()
  nuxtApp.hook('vue:error', (err, _target, info) => capture(err, 'vue:error', String(info)))
  installGlobalErrorListeners(capture)
})
