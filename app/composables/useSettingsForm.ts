/**
 * 設定画面のフォーム状態 (P08a Step 3 / S1)。
 *
 * UI ローカル ref と `appConfig` (Rust 側) の双方向同期
 * (`applyConfigToLocal` / `flushLocalToConfig` / `save` / `discardChanges`) を
 * `settings.vue` から移動したもの。`useAppSettings()` を内包する。
 */
import type { GithubAccount } from '~/types/githubAuth'
import { useAppSettings } from './useAppSettings'

// バイト ⇄ GB / MB 変換ユーティリティ
export const BYTES_PER_GB = 1024 * 1024 * 1024
export const BYTES_PER_MB = 1024 * 1024

export function useSettingsForm() {
  const { config: appConfig, update: persistConfig, load: loadConfig } = useAppSettings()

  // UI 用ローカル ref。`appConfig` (Rust 側) との双方向同期を watch で実現する。
  const general = ref({
    language: 'ja' as 'ja' | 'en' | 'auto',
    applyShadowControl: true,
    showApplyToast: true,
    hideMainOnLaunch: false,
    crashReporting: false,
  })
  const startup = ref({
    autoStart: true,
    startMinimized: true,
  })
  const library = ref({
    totalLimitWarnGb: 1,
    storageWarnEnabled: true,
  })
  const security = ref({
    requireSignedThemes: false,
    warnUnsignedImport: true,
  })
  const githubAccount = ref<GithubAccount | null>(null)
  const logging = ref({
    logLevel: 'INFO' as 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR',
    retentionDays: 14,
    maxSizeMb: 100,
  })
  const updates = ref({
    autoUpdate: true,
  })

  const dirty = ref(false)
  const saving = ref(false)
  const saveError = ref<string | null>(null)

  // applyConfigToLocal 実行中は dirty watch の発火を抑制する
  let suppressDirty = false

  /** appConfig (Rust 側) → UI ローカル ref へ反映 */
  function applyConfigToLocal() {
    const c = appConfig.value
    if (!c) return
    suppressDirty = true
    general.value.language = (c.general.language as 'ja' | 'en' | 'auto') ?? 'auto'
    general.value.crashReporting = c.general.crash_reporting
    // Wave 1B-2: 適用トースト表示フラグを UI ローカル ref に反映。
    // undefined (V1 等の旧データ) の場合は V2 既定値 true を採用。
    general.value.showApplyToast = c.general.show_apply_toast ?? true
    // Wave 1B-3: カーソル影 ON/OFF 制御フラグ。V2 既定 true。
    general.value.applyShadowControl = c.general.apply_shadow_control ?? true
    startup.value.autoStart = c.general.auto_start
    // Wave 1B-4: --autostart 起動時のウィンドウ最小化。V2 既定 false。
    startup.value.startMinimized = c.general.start_minimized ?? false
    updates.value.autoUpdate = c.general.auto_update

    library.value.totalLimitWarnGb = c.security.storage_warning_threshold / BYTES_PER_GB
    // Wave 1B-6: ストレージ警告トースト表示フラグ。V2 既定 true。
    library.value.storageWarnEnabled = c.general.show_storage_warning ?? true

    // Wave 1B-5: 未署名インポート制御 2 フラグ。V2 既定 false / true。
    security.value.requireSignedThemes = c.security.require_signed_themes ?? false
    security.value.warnUnsignedImport = c.security.warn_unsigned_import ?? true

    logging.value.logLevel = (c.logging.level as typeof logging.value.logLevel) ?? 'INFO'
    logging.value.retentionDays = c.logging.retention_days
    logging.value.maxSizeMb = c.logging.max_total_size / BYTES_PER_MB

    githubAccount.value = c.github_account ?? null

    dirty.value = false
    // watch のマイクロタスク実行後にフラグを解除する
    nextTick(() => {
      suppressDirty = false
    })
  }

  /** UI ローカル ref → appConfig 形状にコピー */
  function flushLocalToConfig() {
    return persistConfig((draft) => {
      draft.general.language = general.value.language
      draft.general.crash_reporting = general.value.crashReporting
      // Wave 1B-2: 適用トースト表示フラグを draft に書き戻し。
      // (旧 V1 データで undefined の場合は V2 既定値 true を採用)
      draft.general.show_apply_toast = general.value.showApplyToast ?? true
      // Wave 1B-3: 影制御フラグを draft に書き戻し。V2 既定 true。
      draft.general.apply_shadow_control = general.value.applyShadowControl ?? true
      draft.general.auto_start = startup.value.autoStart
      // Wave 1B-4: --autostart 起動時ウィンドウ最小化。V2 既定 false。
      draft.general.start_minimized = startup.value.startMinimized ?? false
      draft.general.auto_update = updates.value.autoUpdate

      draft.security.storage_warning_threshold = Math.round(
        library.value.totalLimitWarnGb * BYTES_PER_GB,
      )
      // Wave 1B-5: 未署名インポート制御 2 フラグ。V2 既定 false / true。
      draft.security.require_signed_themes = security.value.requireSignedThemes ?? false
      draft.security.warn_unsigned_import = security.value.warnUnsignedImport ?? true

      // Wave 1B-6: ストレージ警告トースト表示フラグ。V2 既定 true。
      draft.general.show_storage_warning = library.value.storageWarnEnabled ?? true

      draft.logging.level = logging.value.logLevel
      draft.logging.retention_days = logging.value.retentionDays
      draft.logging.max_total_size = Math.round(logging.value.maxSizeMb * BYTES_PER_MB)
    })
  }

  async function save() {
    saving.value = true
    saveError.value = null
    try {
      await flushLocalToConfig()
      dirty.value = false
    } catch (err) {
      saveError.value = err instanceof Error ? err.message : String(err)
    } finally {
      saving.value = false
    }
  }

  function discardChanges() {
    applyConfigToLocal()
  }

  async function reloadFromBackend() {
    await loadConfig(true)
    applyConfigToLocal()
  }

  // 任意のローカル変更を dirty フラグ化 (applyConfigToLocal 実行中は除外)
  watch(
    [general, startup, library, security, logging, updates],
    () => {
      if (appConfig.value && !suppressDirty) dirty.value = true
    },
    { deep: true },
  )

  return {
    general,
    startup,
    library,
    security,
    logging,
    updates,
    githubAccount,
    dirty,
    saving,
    saveError,
    applyConfigToLocal,
    save,
    discardChanges,
    reloadFromBackend,
  }
}
