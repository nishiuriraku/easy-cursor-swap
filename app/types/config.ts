/**
 * Rust 側の `AppConfig` (src-tauri/src/config.rs) と対応する型定義。
 * snake_case フィールド名はそのまま (Tauri の serde 既定)。
 */

export interface GeneralConfig {
  auto_start: boolean
  auto_update: boolean
  language: string
  active_theme_id: string | null
  panic_hotkey: string
  /** クラッシュレポート送信オプトイン (デフォルト false) */
  crash_reporting: boolean
}

/** `list_crash_reports` IPC が返すクラッシュレポート 1 件 */
export interface CrashReport {
  file_name: string
  timestamp_utc: string
  app_version: string
  os: string
  message: string
  location: string | null
}

export interface DarkModeConfig {
  enabled: boolean
  light_theme_id: string | null
  dark_theme_id: string | null
}

export interface SecurityConfig {
  max_pack_compressed_size: number
  max_pack_uncompressed_size: number
  max_image_file_size: number
  storage_warning_threshold: number
}

export interface LoggingConfig {
  level: string
  retention_days: number
  max_total_size: number
}

export interface AppConfig {
  schema_version: number
  general: GeneralConfig
  dark_mode: DarkModeConfig
  security: SecurityConfig
  logging: LoggingConfig
}
