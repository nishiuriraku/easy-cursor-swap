/**
 * 設定画面のキーストア操作 (P08a Step 3 / S3)。
 *
 * `useKeystore()` を内包し、生成/再生成/削除/エクスポート/インポートと
 * パスフレーズプロンプトの状態をまとめる。`t` は deps で受ける。
 */
import { useKeystore } from './useKeystore'

export interface KeystoreSettingsDeps {
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useKeystoreSettingsActions(deps: KeystoreSettingsDeps) {
  const { t } = deps
  const {
    info: keystoreInfo,
    busy: keystoreBusy,
    lastError: keystoreError,
    refresh: refreshKeystore,
    generate: generateKeystore,
    remove: removeKeystore,
    exportPrivate: exportPrivateKey,
    importPrivate: importPrivateKey,
  } = useKeystore()

  // パスフレーズプロンプト制御
  const passphrasePrompt = ref<{ mode: 'export' | 'import'; open: boolean }>({
    mode: 'export',
    open: false,
  })
  const keystoreMessage = ref<string | null>(null)

  async function onKeystoreGenerate() {
    keystoreMessage.value = null
    await generateKeystore(false)
  }
  async function onKeystoreRegenerate() {
    // 既存鍵を上書き再生成。ユーザーには事前に dialog::ask で確認。
    const { ask } = await import('@tauri-apps/plugin-dialog')
    const proceed = await ask(t('settings.askRegenerateMsg'), {
      title: t('settings.askRegenerateTitle'),
      kind: 'warning',
    })
    if (!proceed) return
    keystoreMessage.value = null
    await generateKeystore(true)
  }
  async function onPassphraseConfirm(passphrase: string) {
    const mode = passphrasePrompt.value.mode
    keystoreMessage.value = null
    if (mode === 'export') {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const today = new Date().toISOString().slice(0, 10)
      const target = await save({
        defaultPath: `easycursorswap-key-${today}.cfkey`,
        filters: [{ name: 'EasyCursorSwap Key', extensions: ['cfkey'] }],
      })
      if (!target) return
      const written = await exportPrivateKey(passphrase, target)
      if (written !== null) {
        keystoreMessage.value = t('settings.keyExportSuccess', {
          size: written,
          target,
        })
      }
    } else {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({
        multiple: false,
        filters: [{ name: 'EasyCursorSwap Key', extensions: ['cfkey'] }],
      })
      if (!selected || Array.isArray(selected)) return
      const result = await importPrivateKey(passphrase, selected)
      if (result) {
        keystoreMessage.value = t('settings.keyImportSuccess', {
          keyId: result.key_id ?? '?',
        })
      }
    }
  }

  function onKeystoreExport() {
    passphrasePrompt.value = { mode: 'export', open: true }
  }

  function onKeystoreImport() {
    passphrasePrompt.value = { mode: 'import', open: true }
  }

  async function onKeystoreDelete() {
    const { ask } = await import('@tauri-apps/plugin-dialog')
    const proceed = await ask(t('settings.askDeleteMsg'), {
      title: t('settings.askDeleteTitle'),
      kind: 'warning',
    })
    if (!proceed) return
    keystoreMessage.value = null
    await removeKeystore()
  }

  return {
    keystoreInfo,
    keystoreBusy,
    keystoreError,
    keystoreMessage,
    passphrasePrompt,
    refreshKeystore,
    onKeystoreGenerate,
    onKeystoreRegenerate,
    onKeystoreExport,
    onKeystoreImport,
    onKeystoreDelete,
    onPassphraseConfirm,
  }
}
