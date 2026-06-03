/**
 * `.cursorprofile` のエクスポート / インポート IPC を集約する薄いラッパ。
 * settings.vue が直接 invoke していたのをまとめる。
 * 対応 Rust: `src-tauri/src/commands/profile.rs` (`export_profile` / `import_profile`)。
 * import の戻り値 (ProfileEnvelope) は呼び出し側で未使用のため `unknown` 型のままにする
 * (AppConfig 全体の mirror は YAGNI)。
 */
export function useProfileBackup() {
  /** 現在の設定スナップショットを `.cursorprofile` として path に書き出す。 */
  function exportProfile(path: string): Promise<void | null> {
    return invokeTauri<void>('export_profile', { path })
  }
  /** `.cursorprofile` を読み込む。merge=true で既存設定にマージ、false で上書き。戻り値は未使用。 */
  function importProfile(path: string, merge: boolean): Promise<unknown> {
    return invokeTauri('import_profile', { path, merge })
  }
  return { exportProfile, importProfile }
}
