/**
 * useAppSettings の typed-patch 経路 (Wave 2B / Task 3) の回帰テスト。
 *
 * 検証対象:
 *   1. mutator 内で `schema_version` / `github_account` / セキュリティ閾値 /
 *      履歴系フィールドを書き換えても、IPC に送られる patch には含まれない
 *      (= フロントから保護される契約)
 *   2. 値に変更が無いセクションは patch に含まれない
 *   3. どのセクションも差分がない場合は invoke 自体が起こらない
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invokeTauriMock = vi.fn<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>>()

vi.mock('../useTauri', () => ({
  invokeTauri: (cmd: string, args?: Record<string, unknown>) => invokeTauriMock(cmd, args),
}))

import { useAppSettings } from '../useAppSettings'

const baseConfig = () => ({
  schema_version: 2,
  general: {
    auto_start: true,
    auto_update: true,
    language: 'ja',
    active_theme_id: null,
    panic_hotkey: 'Ctrl+Alt+Shift+R',
    crash_reporting: false,
    favorites: ['theme-uuid-1'],
    usage: { 'theme-uuid-1': { apply_count: 3, last_applied_at: null } },
    show_apply_toast: true,
    apply_shadow_control: true,
    start_minimized: false,
    show_storage_warning: true,
  },
  security: {
    max_pack_compressed_size: 50 * 1024 * 1024,
    max_pack_uncompressed_size: 200 * 1024 * 1024,
    max_image_file_size: 10 * 1024 * 1024,
    storage_warning_threshold: 1024 * 1024 * 1024,
    require_signed_themes: false,
    warn_unsigned_import: true,
  },
  logging: { level: 'INFO', retention_days: 14, max_total_size: 100 * 1024 * 1024 },
  github_account: { login: 'octocat', token_saved_at: '2026-07-01T00:00:00Z' },
})

beforeEach(() => {
  invokeTauriMock.mockReset()
  // 1回目: load() が返す値 (get_config)。2回目: update_config の戻り値。
  invokeTauriMock.mockResolvedValueOnce(baseConfig())
  // モジュール singleton を毎回リセット (テスト間の状態リーク防止)
  useAppSettings().__resetForTests()
})

describe('useAppSettings typed-patch contract (Wave 2B / Task 3)', () => {
  it('includes only the changed general fields in the patch', async () => {
    const { config, load, update } = useAppSettings()
    await load(true)
    // update_config の戻り値は mutation 反映後を想定
    const updatedConfig = { ...baseConfig() }
    updatedConfig.general = { ...baseConfig().general, crashReporting: true }
    invokeTauriMock.mockResolvedValueOnce(updatedConfig)

    const result = await update((c) => {
      c.general.crashReporting = true
    })

    expect(result).not.toBeNull()
    expect(invokeTauriMock).toHaveBeenCalledTimes(2) // get_config + update_config
    const [, args] = invokeTauriMock.mock.calls[1]!
    expect(args!.updates).toEqual({
      general: { crashReporting: true },
    })
    expect(config.value!.general.crashReporting).toBe(true)
  })

  it('never sends schema_version, github_account, security thresholds, favorites or usage', async () => {
    const { load, update } = useAppSettings()
    await load(true)
    invokeTauriMock.mockResolvedValueOnce({ ...baseConfig() })

    // mutator 内で forbidden fields を触ろうとしても patch に漏れない
    // (camelCase キーを直接書き換えても diff に入るものは AppConfigPatch 経由)
    await update((c) => {
      c.general.language = 'en'
      c.security.requireSignedThemes = true
      // forbidden attempts (type cast 経由でしか触れないので no-op):
      ;(c as { schema_version: number }).schema_version = 1
      ;(c as { github_account: unknown }).github_account = null
      c.security.max_pack_compressed_size = 1
      c.general.favorites = ['evil']
      c.general.usage = {}
    })

    const [, args] = invokeTauriMock.mock.calls[1]!
    expect(args!.updates).toEqual({
      general: { language: 'en' },
      security: { requireSignedThemes: true },
    })
    expect(JSON.stringify(args!.updates)).not.toContain('schema_version')
    expect(JSON.stringify(args!.updates)).not.toContain('github_account')
    expect(JSON.stringify(args!.updates)).not.toContain('favorites')
    expect(JSON.stringify(args!.updates)).not.toContain('usage')
    expect(JSON.stringify(args!.updates)).not.toContain('max_pack_compressed_size')
  })

  it('skips the IPC call entirely when no field actually changes', async () => {
    const { load, update } = useAppSettings()
    await load(true)
    const before = invokeTauriMock.mock.calls.length

    const result = await update(() => {
      /* no-op */
    })

    expect(result).not.toBeNull()
    expect(invokeTauriMock.mock.calls.length).toBe(before) // update_config なし
  })

  it('sends a logging patch when only logging.level changes', async () => {
    const { load, update } = useAppSettings()
    await load(true)
    invokeTauriMock.mockResolvedValueOnce({ ...baseConfig() })

    await update((c) => {
      c.logging.level = 'DEBUG'
    })

    const [, args] = invokeTauriMock.mock.calls[1]!
    expect(args!.updates).toEqual({ logging: { level: 'DEBUG' } })
  })
})
