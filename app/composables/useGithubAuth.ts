/**
 * GitHub OAuth Device Flow のクライアント状態管理。
 *
 * `start()` で `start_device_flow` IPC を叩き、user_code を表示できる状態にする。
 * 内部で setInterval ポーリングし、`complete_device_flow` IPC の結果に応じて状態遷移する。
 *
 * Rust 側 (commands/marketplace_submit.rs) は単発 IPC のみ提供し、
 * interval / slow_down の制御はこの composable が行う。
 */
import type { Ref } from 'vue'
import type { StartFlowResult, CompleteFlowResult } from '~/types/githubAuth'

export type GithubAuthStatus = 'idle' | 'waiting' | 'ready' | 'denied' | 'expired' | 'error'

export function useGithubAuth() {
  const status: Ref<GithubAuthStatus> = ref('idle')
  const userCode = ref<string | null>(null)
  const verificationUri = ref<string | null>(null)
  const expiresAt = ref<number | null>(null)
  const login = ref<string | null>(null)
  const errorMsg = ref<string | null>(null)
  let timer: ReturnType<typeof setInterval> | null = null
  let intervalMs = 5000

  function stopTimer() {
    if (timer != null) {
      clearInterval(timer)
      timer = null
    }
  }

  async function start() {
    stopTimer()
    status.value = 'waiting'
    errorMsg.value = null
    login.value = null
    try {
      const r = await invokeTauri<StartFlowResult>('start_device_flow')
      if (!r) {
        status.value = 'error'
        errorMsg.value = 'Tauri ランタイム未接続'
        return
      }
      userCode.value = r.userCode
      verificationUri.value = r.verificationUri
      expiresAt.value = Date.now() + r.expiresIn * 1000
      intervalMs = Math.max(1000, r.interval * 1000)
      timer = setInterval(() => void poll(), intervalMs)
    } catch (e) {
      status.value = 'error'
      errorMsg.value = String(e)
    }
  }

  async function poll() {
    try {
      const r = await invokeTauri<CompleteFlowResult>('complete_device_flow')
      if (!r) return
      switch (r.status) {
        case 'pending':
          return
        case 'slow_down':
          stopTimer()
          intervalMs += 5000
          timer = setInterval(() => void poll(), intervalMs)
          return
        case 'expired':
          stopTimer()
          status.value = 'expired'
          return
        case 'denied':
          stopTimer()
          status.value = 'denied'
          return
        case 'ready':
          stopTimer()
          login.value = r.login
          status.value = 'ready'
          return
      }
    } catch (e) {
      stopTimer()
      status.value = 'error'
      errorMsg.value = String(e)
    }
  }

  async function cancel() {
    stopTimer()
    try {
      await invokeTauri<void>('cancel_device_flow')
    } catch {
      // 破棄成功扱い (バックエンドが既にクリアしていてもエラーにしない)
    }
    status.value = 'idle'
    userCode.value = null
    verificationUri.value = null
    expiresAt.value = null
  }

  // Component / effect scope の dispose 時に timer を必ず停止する。
  // 親ダイアログがモーダルを閉じ忘れた場合のクリーンアップ。
  if (getCurrentScope()) {
    onScopeDispose(() => stopTimer())
  }

  return { status, userCode, verificationUri, expiresAt, login, errorMsg, start, cancel }
}
