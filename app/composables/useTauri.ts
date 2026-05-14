/**
 * Tauri IPC ラッパー。
 * Web 開発時 (Tauri ランタイム未接続) は警告ログのみで失敗させず、
 * UI 開発を妨げないようフォールバックする。
 */

import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event'

let invokeFn: (<T>(cmd: string, args?: Record<string, unknown>) => Promise<T>) | null = null
let warnedNoTauri = false

async function getInvoke() {
  if (invokeFn) return invokeFn
  try {
    const mod = await import('@tauri-apps/api/core')
    invokeFn = mod.invoke as typeof invokeFn
    return invokeFn
  } catch {
    if (!warnedNoTauri) {
      console.warn('[Tauri] @tauri-apps/api 未利用環境。IPC 呼び出しはスキップされます')
      warnedNoTauri = true
    }
    return null
  }
}

export async function invokeTauri<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  const fn = await getInvoke()
  if (!fn) return null
  try {
    return await fn<T>(cmd, args)
  } catch (err) {
    console.error(`[Tauri] invoke '${cmd}' failed:`, err)
    throw err
  }
}

/**
 * 起動時または 2 重起動シグナル経由で `.cursorpack` パスが Rust 側に積まれていれば
 * 取り出す。なければ null。`useCursorpackOpener` から使う。
 */
export async function takePendingCursorpack(): Promise<string | null> {
  const result = await invokeTauri<string | null>('take_pending_cursorpack')
  return result ?? null
}

let listenFn: (<T>(event: string, cb: EventCallback<T>) => Promise<UnlistenFn>) | null = null

async function getListen() {
  if (listenFn) return listenFn
  try {
    const mod = await import('@tauri-apps/api/event')
    listenFn = mod.listen as typeof listenFn
    return listenFn
  } catch {
    // Tauri 未接続環境: 何もしない unlisten を返すフォールバック
    return null
  }
}

/**
 * Tauri Event を listen する。Tauri 未接続時は no-op unlisten を返す。
 */
export async function listenTauri<T>(event: string, cb: EventCallback<T>): Promise<UnlistenFn> {
  const fn = await getListen()
  if (!fn) {
    return (() => {}) as UnlistenFn
  }
  return await fn<T>(event, cb)
}
