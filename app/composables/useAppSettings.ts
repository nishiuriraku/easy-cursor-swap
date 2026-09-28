/**
 * Rust 側の `AppConfig` を取得 / 更新する composable。
 * グローバルなリアクティブシングルトンで全画面が同じインスタンスを参照する。
 *
 * Wave 2B / Task 3: `update` は `AppConfig` 全体を Rust に送るのではなく、
 * 差分だけを `AppConfigPatch` に詰めて送る。`schema_version` /
 * `github_account` / セキュリティ閾値 / 履歴系フィールドはそもそも
 * `AppConfigPatch` に存在しないため、フロントから上書きできない。
 */
import type { AppConfig, GeneralConfig, LoggingConfig, SecurityConfig } from '~/types/config'
import type {
  AppConfigPatch,
  GeneralConfigPatch,
  LoggingConfigPatch,
  SecurityConfigPatch,
} from '~/types/generated'

const config = ref<AppConfig | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)

let inflight: Promise<AppConfig | null> | null = null

async function load(force = false): Promise<AppConfig | null> {
  if (config.value && !force) return config.value
  if (inflight) return inflight

  loading.value = true
  error.value = null
  inflight = (async () => {
    try {
      const result = await invokeTauri<AppConfig>('get_config')
      config.value = result ?? null
      return config.value
    } catch (err) {
      error.value = appErrorMessage(err)
      console.warn('[useAppSettings] get_config failed:', err)
      return null
    } finally {
      loading.value = false
      inflight = null
    }
  })()
  return inflight
}

/** 2 つの値を JSON 比較して「同じなら undefined、違ったら next 側を返す」ヘルパ。 */
function diff<T>(prev: T, next: T): T | undefined {
  return JSON.stringify(prev) === JSON.stringify(next) ? undefined : next
}

/** `undefined` キーを取り除き、残りが空なら undefined を返す。 */
function stripUndefined<T extends Record<string, unknown>>(obj: T): Partial<T> | undefined {
  const out: Partial<T> = {}
  let hasAny = false
  for (const k of Object.keys(obj) as (keyof T)[]) {
    const v = obj[k]
    if (v !== undefined) {
      out[k] = v
      hasAny = true
    }
  }
  return hasAny ? out : undefined
}

function diffGeneral(
  prev: GeneralConfig | null,
  next: GeneralConfig | null,
): GeneralConfigPatch | undefined {
  if (!prev || !next) return undefined
  return stripUndefined({
    autoStart: diff(prev.auto_start, next.auto_start),
    autoUpdate: diff(prev.auto_update, next.auto_update),
    language: diff(prev.language, next.language),
    crashReporting: diff(prev.crash_reporting, next.crash_reporting),
    showApplyToast: diff(prev.show_apply_toast, next.show_apply_toast),
    applyShadowControl: diff(prev.apply_shadow_control, next.apply_shadow_control),
    startMinimized: diff(prev.start_minimized, next.start_minimized),
    showStorageWarning: diff(prev.show_storage_warning, next.show_storage_warning),
    onboardingVersion: diff(prev.onboarding_version, next.onboarding_version),
  }) as GeneralConfigPatch
}

function diffSecurity(
  prev: SecurityConfig | null,
  next: SecurityConfig | null,
): SecurityConfigPatch | undefined {
  if (!prev || !next) return undefined
  return stripUndefined({
    requireSignedThemes: diff(prev.require_signed_themes, next.require_signed_themes),
    warnUnsignedImport: diff(prev.warn_unsigned_import, next.warn_unsigned_import),
  }) as SecurityConfigPatch
}

function diffLogging(
  prev: LoggingConfig | null,
  next: LoggingConfig | null,
): LoggingConfigPatch | undefined {
  if (!prev || !next) return undefined
  return stripUndefined({
    level: diff(prev.level, next.level),
  }) as LoggingConfigPatch
}

/**
 * `mutator` で config を変更し、Rust に **差分 patch だけ** を送る。
 * 失敗時はメモリ上の patch 適用を巻き戻し、元の `config.value` を維持する。
 *
 * `schema_version` / `github_account` / セキュリティ閾値 / 履歴系フィールドは
 * `AppConfigPatch` に存在しないため、`mutator` 内で書き換えても Rust には届かない
 * (= フロントから保護される)。
 */
async function update(mutator: (c: AppConfig) => void): Promise<AppConfig | null> {
  const current = config.value
  if (!current) return null

  const draft: AppConfig = JSON.parse(JSON.stringify(current))
  mutator(draft)

  // 差分だけを patch に詰める
  const patch: AppConfigPatch = {
    general: diffGeneral(current.general, draft.general),
    security: diffSecurity(current.security, draft.security),
    logging: diffLogging(current.logging, draft.logging),
  }
  // どのセクションも差分が無ければ IPC 自体を呼ばない (= Rust 側の
  // atomic write も発生しない)。rust 側の round-trip を抑止したい場面向け。
  if (!patch.general && !patch.security && !patch.logging) {
    return current
  }

  try {
    const updated = await invokeTauri<AppConfig>('update_config', { updates: patch })
    if (updated) config.value = updated
    return updated
  } catch (err) {
    error.value = appErrorMessage(err)
    console.error('[useAppSettings] update_config failed:', err)
    return null
  }
}

export function useAppSettings() {
  return { config, loading, error, load, update, __resetForTests }
}

/**
 * テスト専用: モジュールスコープの singleton state (`config`, `inflight`) を破棄する。
 * 通常アプリコードから呼び出してはならない。
 */
function __resetForTests() {
  config.value = null
  inflight = null
  loading.value = false
  error.value = null
}
