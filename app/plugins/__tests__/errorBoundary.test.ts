/**
 * errorBoundary.client.ts の window リスナ (P11c)。
 *
 * プラグイン本体は Nuxt 依存 (`defineNuxtPlugin`) のため、window 購読だけを純関数
 * `installGlobalErrorListeners(capture)` として export し、ここから直接検証する。
 */
import { describe, it, expect, vi, afterEach } from 'vitest'

// `defineNuxtPlugin` は vitest の unimport 対象外なので、モジュール評価前に stub する。
vi.stubGlobal('defineNuxtPlugin', (setup: unknown) => ({ __setup: setup }))

const { installGlobalErrorListeners } = await import('../errorBoundary.client')

describe('installGlobalErrorListeners', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('captures window error events with the error object', () => {
    const seen: Array<[unknown, string]> = []
    const uninstall = installGlobalErrorListeners((err, source) => {
      seen.push([err, source])
    })
    try {
      const err = new Error('e1')
      window.dispatchEvent(new ErrorEvent('error', { error: err }))
      expect(seen).toHaveLength(1)
      expect(seen[0]?.[0]).toBe(err)
      expect(seen[0]?.[1]).toBe('window.error')
    } finally {
      uninstall()
    }
  })

  it('captures unhandledrejection with the rejection reason', () => {
    const seen: Array<[unknown, string]> = []
    const uninstall = installGlobalErrorListeners((err, source) => {
      seen.push([err, source])
    })
    try {
      const ev = new Event('unhandledrejection') as Event & { reason?: unknown }
      ev.reason = 'nope'
      window.dispatchEvent(ev)
      expect(seen).toEqual([['nope', 'unhandledrejection']])
    } finally {
      uninstall()
    }
  })

  it('uninstall() removes both listeners', () => {
    const capture = vi.fn()
    const uninstall = installGlobalErrorListeners(capture)
    uninstall()
    window.dispatchEvent(new ErrorEvent('error', { error: new Error('late') }))
    const ev = new Event('unhandledrejection') as Event & { reason?: unknown }
    ev.reason = 'late'
    window.dispatchEvent(ev)
    expect(capture).not.toHaveBeenCalled()
  })
})
