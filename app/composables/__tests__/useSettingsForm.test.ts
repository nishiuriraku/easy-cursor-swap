/**
 * useSettingsForm のテスト (P08a Step 3 / S1)。
 *
 * `useAppSettings` をモックし、applyConfigToLocal の写像 (snake→camel +
 * `?? true` 既定)、save() が snake_case で書くこと、dirty 抑制を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { nextTick } from 'vue'

const appConfig = { value: null as unknown }
const updateMock = vi.fn()
const loadMock = vi.fn()

vi.mock('../useAppSettings', () => ({
  useAppSettings: () => ({
    config: appConfig,
    update: (...args: unknown[]) => updateMock(...args),
    load: (...args: unknown[]) => loadMock(...args),
  }),
}))

import { useSettingsForm } from '../useSettingsForm'

const snakeConfig = () => ({
  general: {
    language: 'en',
    auto_start: true,
    auto_update: false,
    crash_reporting: true,
    show_apply_toast: undefined,
    apply_shadow_control: undefined,
    start_minimized: undefined,
    show_storage_warning: undefined,
  },
  security: {
    storage_warning_threshold: 2 * 1024 * 1024 * 1024,
    require_signed_themes: undefined,
    warn_unsigned_import: undefined,
  },
  logging: { level: 'DEBUG', retention_days: 7, max_total_size: 50 * 1024 * 1024 },
  github_account: null,
})

beforeEach(() => {
  updateMock.mockReset()
  loadMock.mockReset()
  appConfig.value = null
})

describe('useSettingsForm', () => {
  it('applyConfigToLocal maps snake_case to camelCase with defaults', async () => {
    appConfig.value = snakeConfig()
    const form = useSettingsForm()
    form.applyConfigToLocal()
    await nextTick()
    await nextTick()
    expect(form.general.value.language).toBe('en')
    expect(form.general.value.crashReporting).toBe(true)
    expect(form.general.value.showApplyToast).toBe(true)
    expect(form.startup.value.autoStart).toBe(true)
    expect(form.updates.value.autoUpdate).toBe(false)
    expect(form.library.value.totalLimitWarnGb).toBe(2)
    expect(form.security.value.requireSignedThemes).toBe(false)
    expect(form.logging.value.logLevel).toBe('DEBUG')
    expect(form.dirty.value).toBe(false)
  })

  it('save() writes snake_case via mutator', async () => {
    appConfig.value = snakeConfig()
    updateMock.mockResolvedValueOnce(appConfig.value)
    const form = useSettingsForm()
    form.applyConfigToLocal()
    await nextTick()
    await nextTick()
    form.general.value.language = 'ja'
    await form.save()
    expect(updateMock).toHaveBeenCalledTimes(1)
    const mutator = updateMock.mock.calls[0]![0] as (draft: never) => void
    const draft = JSON.parse(JSON.stringify(snakeConfig()))
    mutator(draft)
    expect(draft.general.language).toBe('ja')
    expect(draft.general.crash_reporting).toBe(true)
    expect(form.dirty.value).toBe(false)
  })

  it('does not mark dirty while suppressing', async () => {
    appConfig.value = snakeConfig()
    const form = useSettingsForm()
    form.applyConfigToLocal()
    // suppressDirty 解除前に変更しても dirty は立たない
    form.general.value.language = 'ja'
    await nextTick()
    expect(form.dirty.value).toBe(false)
  })
})
