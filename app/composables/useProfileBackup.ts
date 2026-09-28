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

export interface ProfileBackupDialogDeps {
  t: (key: string, params?: Record<string, string | number>) => string
}

/**
 * 設定画面のプロファイル入出力 UI 状態 (P08a Step 3 / S6)。
 *
 * dialog 選択 + 文言組み立てまで行い、import 成功時は「再読込が必要」を
 * 戻り値 boolean で返す (呼び出し側で `loadConfig + applyConfigToLocal`)。
 */
export function useProfileBackupDialog(deps: ProfileBackupDialogDeps) {
  const { t } = deps
  const { exportProfile: runExportProfile, importProfile: runImportProfile } = useProfileBackup()
  const busy = ref(false)
  const message = ref<string | null>(null)

  async function exportWithDialog() {
    busy.value = true
    message.value = null
    try {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const today = new Date().toISOString().slice(0, 10)
      const target = await save({
        defaultPath: `easycursorswap-${today}.cursorprofile`,
        filters: [{ name: 'EasyCursorSwap Profile', extensions: ['cursorprofile'] }],
      })
      if (!target) return
      await runExportProfile(target)
      message.value = t('settings.profileExportSuccess', { target })
    } catch (err) {
      message.value = t('settings.profileExportFail', {
        error: appErrorMessage(err),
      })
    } finally {
      busy.value = false
    }
  }

  async function importWithDialog(): Promise<boolean> {
    busy.value = true
    message.value = null
    try {
      const { open, ask } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'EasyCursorSwap Profile', extensions: ['cursorprofile'] }],
      })
      if (!selected || Array.isArray(selected)) return false
      const overwrite = await ask(t('settings.profileImportAskMsg'), {
        title: t('settings.profileImportAskTitle'),
        kind: 'warning',
      })
      await runImportProfile(selected, !overwrite)
      message.value = t('settings.profileImportSuccess', {
        target: selected,
      })
      return true
    } catch (err) {
      message.value = t('settings.profileImportFail', {
        error: appErrorMessage(err),
      })
      return false
    } finally {
      busy.value = false
    }
  }

  return { busy, message, exportWithDialog, importWithDialog }
}
