import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// 依存をモック化: load() で config を返す useAppSettings、check() を返す useUpdater、notify。
const configRef: { value: unknown } = { value: null }
const loadMock = vi.fn(async () => configRef.value)
vi.mock('../useAppSettings', () => ({
  useAppSettings: () => ({ load: loadMock, config: configRef }),
}))

const checkMock = vi.fn<() => Promise<unknown>>()
vi.mock('../useUpdater', () => ({
  useUpdater: () => ({ check: checkMock }),
}))

const notifyMock = vi.fn(async () => undefined)
vi.mock('../useNotify', () => ({
  notify: (...args: unknown[]) => notifyMock(...(args as [])),
}))

const invokeTauriMock = vi.fn<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>>()
vi.mock('../useTauri', () => ({
  invokeTauri: (cmd: string, args?: Record<string, unknown>) => invokeTauriMock(cmd, args),
}))

// i18n: 実 useI18n を差し替え、翻訳経路 (t) が使われたことを検証できるようにする (G18)。
// version を補間して返すので、既存の「body に version が含まれる」検証も通る。
const tMock = vi.fn((key: string, params?: Record<string, string | number>) => {
  if (key === 'updater.toastUpdateAvailable') {
    return `New version v${params?.version} is available.`
  }
  return key
})
vi.mock('../useI18n', () => ({
  useI18n: () => ({ t: tMock }),
}))

import { bootstrapUpdaterCheck } from '../useUpdaterBootstrap'

const LAST_CHECK_KEY = 'ecs.updater.last_check_at'

function mkConfig(auto_update: boolean) {
  return {
    schema_version: 1,
    general: {
      auto_start: true,
      auto_update,
      language: 'ja',
      active_theme_id: null,
      panic_hotkey: 'Ctrl+Alt+Shift+R',
      crash_reporting: false,
    },
    security: {
      max_pack_compressed_size: 50,
      max_pack_uncompressed_size: 200,
      max_image_file_size: 10,
      storage_warning_threshold: 1024,
    },
    logging: {
      level: 'INFO',
      retention_days: 14,
      max_total_size: 100,
    },
  }
}

/** マイクロタスクを 1 tick 進めて bootstrap 内部の `void run()` を待つ。 */
async function flush(): Promise<void> {
  for (let i = 0; i < 5; i++) {
    await Promise.resolve()
  }
}

describe('useUpdaterBootstrap', () => {
  beforeEach(() => {
    localStorage.clear()
    loadMock.mockClear()
    checkMock.mockReset()
    notifyMock.mockClear()
    invokeTauriMock.mockReset()
    tMock.mockClear()
    // デフォルトでは major bump check は false (= 通常更新) を返す。
    // 個別 it で必要なら invokeTauriMock.mockResolvedValue(true) で上書きする。
    invokeTauriMock.mockResolvedValue(false)
  })

  afterEach(() => {
    configRef.value = null
  })

  it('auto_update=false なら check しない', async () => {
    configRef.value = mkConfig(false)
    bootstrapUpdaterCheck()
    await flush()
    expect(checkMock).not.toHaveBeenCalled()
    expect(notifyMock).not.toHaveBeenCalled()
  })

  it('クールダウン中 (24h 未満) なら check しない', async () => {
    configRef.value = mkConfig(true)
    // 1 時間前にチェック済み
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - 60 * 60 * 1000))
    bootstrapUpdaterCheck()
    await flush()
    expect(checkMock).not.toHaveBeenCalled()
  })

  it('クールダウン経過後 (24h 超) なら check して、見つかれば notify', async () => {
    configRef.value = mkConfig(true)
    // 25 時間前
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - 25 * 60 * 60 * 1000))
    checkMock.mockResolvedValue({ version: '0.2.0', currentVersion: '0.1.0' })

    bootstrapUpdaterCheck()
    await flush()

    expect(checkMock).toHaveBeenCalled()
    expect(notifyMock).toHaveBeenCalledOnce()
    const arg = notifyMock.mock.calls[0]?.[0] as { body: string } | undefined
    expect(arg?.body).toContain('0.2.0')
    // 最新のタイムスタンプに更新される
    const newTs = Number(localStorage.getItem(LAST_CHECK_KEY))
    expect(newTs).toBeGreaterThan(Date.now() - 1000)
  })

  it('更新なしでも last_check_at は更新する (失敗時の再試行抑制)', async () => {
    configRef.value = mkConfig(true)
    localStorage.setItem(LAST_CHECK_KEY, '0')
    checkMock.mockResolvedValue(null)

    bootstrapUpdaterCheck()
    await flush()

    expect(checkMock).toHaveBeenCalled()
    expect(notifyMock).not.toHaveBeenCalled()
    expect(Number(localStorage.getItem(LAST_CHECK_KEY))).toBeGreaterThan(0)
  })

  it('メジャー跨ぎ更新が見つかっても notify は呼ばない', async () => {
    configRef.value = mkConfig(true)
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - 25 * 60 * 60 * 1000))
    checkMock.mockResolvedValue({ version: '2.0.0', currentVersion: '1.5.0' })
    invokeTauriMock.mockResolvedValue(true) // major bump = true

    bootstrapUpdaterCheck()
    await flush()

    expect(checkMock).toHaveBeenCalled()
    expect(invokeTauriMock).toHaveBeenCalledWith('check_update_is_major_jump', {
      currentVersion: '1.5.0',
      newVersion: '2.0.0',
    })
    expect(notifyMock).not.toHaveBeenCalled()
  })

  it('check_update_is_major_jump IPC が失敗したら安全側で notify する', async () => {
    configRef.value = mkConfig(true)
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - 25 * 60 * 60 * 1000))
    checkMock.mockResolvedValue({ version: '0.2.0', currentVersion: '0.1.0' })
    invokeTauriMock.mockRejectedValue(new Error('IPC failed'))

    bootstrapUpdaterCheck()
    await flush()

    expect(notifyMock).toHaveBeenCalledOnce()
  })

  it('通知本文は t(updater.toastUpdateAvailable, {version}) 経由で生成する (G18)', async () => {
    configRef.value = mkConfig(true)
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - 25 * 60 * 60 * 1000))
    checkMock.mockResolvedValue({ version: '0.2.0', currentVersion: '0.1.0' })

    bootstrapUpdaterCheck()
    await flush()

    // ハードコード日本語ではなく i18n キー経由であること
    expect(tMock).toHaveBeenCalledWith('updater.toastUpdateAvailable', { version: '0.2.0' })
    const arg = notifyMock.mock.calls[0]?.[0] as { body: string } | undefined
    expect(arg?.body).toBe('New version v0.2.0 is available.')
  })

  // ── Wave 2AB / Task 13: エッジケース補強 ──
  // 既存テストは「クールダウン経過後 + 正常発見」までは網羅していたが、
  // (a) check() 自体が throw するケース、(b) クールダウン境界 (= 24h ちょうど)、
  // (c) config が null (= load() が resolve したが c が null) のケースを取りこぼしていた。

  it('check() 自体が throw しても notify は呼ばない (= エラーは fire-and-forget)', async () => {
    configRef.value = mkConfig(true)
    localStorage.setItem(LAST_CHECK_KEY, '0')
    checkMock.mockRejectedValue(new Error('network: timeout'))

    bootstrapUpdaterCheck()
    await flush()

    // check は呼ばれているが失敗 → notify は出ない (bootstrap が startup をブロックしない contract)
    expect(checkMock).toHaveBeenCalled()
    expect(notifyMock).not.toHaveBeenCalled()
    // 失敗パスでも 24h cooldown を有効化するため、last_check_at を必ず進める
    // (= Tauri invoke 失敗が連続しても毎起動 retry しない)。
    const tsAfterThrow = Number(localStorage.getItem(LAST_CHECK_KEY))
    expect(Number.isFinite(tsAfterThrow) && tsAfterThrow > 0).toBe(true)
    expect(tsAfterThrow).toBeGreaterThan(Date.now() - 1000)
  })

  it('クールダウン境界: ちょうど 24h 経過した直後は skip せず check する', async () => {
    // 仕様: UPDATE_CHECK_COOLDOWN_MS (= 24h) 未満なら skip、それ以外は check。
    // 「未満」と「以上」の境界を 1ms ずらして検証する。
    configRef.value = mkConfig(true)
    const cooldownMs = 24 * 60 * 60 * 1000

    // 24h ぴったり (= 0 ms 差): 仕様上 >= なので check する
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - cooldownMs))
    bootstrapUpdaterCheck()
    await flush()
    expect(checkMock).toHaveBeenCalledTimes(1)

    // 1ms だけ余裕 (= 23h59m59s): < cooldown なので skip される
    localStorage.clear()
    localStorage.setItem(LAST_CHECK_KEY, String(Date.now() - cooldownMs + 1))
    checkMock.mockClear()
    bootstrapUpdaterCheck()
    await flush()
    expect(checkMock).not.toHaveBeenCalled()
  })

  it('config が null (load() が空を返したケース) なら check も notify もしない', async () => {
    configRef.value = null
    bootstrapUpdaterCheck()
    await flush()
    expect(checkMock).not.toHaveBeenCalled()
    expect(notifyMock).not.toHaveBeenCalled()
  })
})
