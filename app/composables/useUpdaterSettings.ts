/**
 * 設定画面のアップデータ設定 (P08a Step 3 / S4)。
 *
 * `useUpdater()` を内包し、メッセージ組み立て・メジャー跨ぎ確認・
 * クールダウン表示をまとめる。`t` は deps で受ける。
 */
import { invokeTauri } from './useTauri'
import { useUpdater, classifyUpdaterError } from './useUpdater'
import { useAppInfo } from './useAppInfo'
import { LAST_UPDATE_CHECK_KEY, UPDATE_CHECK_COOLDOWN_MS } from './updaterConstants'

export interface UpdaterSettingsDeps {
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useUpdaterSettings(deps: UpdaterSettingsDeps) {
  const { t } = deps
  const {
    checking: updaterChecking,
    downloading: updaterDownloading,
    available: updaterAvailable,
    error: updaterError,
    progressBytes: updaterProgress,
    totalBytes: updaterTotal,
    check: checkForUpdate,
    downloadAndInstall: downloadUpdate,
    relaunch: relaunchApp,
  } = useUpdater()
  const updaterMessage = ref<string | null>(null)

  /**
   * Updater のエラーをカテゴリ分類して i18n キー経由で表示文字列にする。
   * `error.value` には生 message を残しているので、ここで毎回 classify する。
   */
  const updaterErrorDisplay = computed(() => {
    if (!updaterError.value) return null
    const { key, message } = classifyUpdaterError(updaterError.value)
    return t(key, { message })
  })

  /** 次回自動チェック (= 起動時 bootstrap が走るタイミング) までの残り時間を文字列で返す。 */
  const autoCheckHint = computed(() => {
    if (typeof localStorage === 'undefined') return t('settings.autoCheckHintReady')
    const raw = Number(localStorage.getItem(LAST_UPDATE_CHECK_KEY) ?? '0')
    if (!Number.isFinite(raw) || raw === 0) return t('settings.autoCheckHintReady')
    const remainingMs = raw + UPDATE_CHECK_COOLDOWN_MS - Date.now()
    if (remainingMs <= 0) return t('settings.autoCheckHintReady')
    const hours = Math.max(1, Math.ceil(remainingMs / (60 * 60 * 1000)))
    return t('settings.autoCheckHintHours', { hours })
  })

  /** クールダウンを破って次回起動時に再チェックさせる。即時 check はしない (UX 上シンプル化)。 */
  function onForceRecheck() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LAST_UPDATE_CHECK_KEY, '0')
    }
    updaterMessage.value = t('settings.autoCheckHintReady')
  }

  // 利用可能なアップデート情報 (メジャー跨ぎ判定に使用)
  const pendingUpdateVersion = ref<string | null>(null)

  async function onCheckUpdate() {
    updaterMessage.value = null
    pendingUpdateVersion.value = null
    const info = await checkForUpdate()
    if (info) {
      pendingUpdateVersion.value = info.version
      updaterMessage.value = t('settings.updateNewVersion', {
        version: info.version,
        current: info.currentVersion,
      })
    } else if (!updaterError.value) {
      // check() は失敗時も null を返すため、エラーが立っているケースを除外しないと
      // 「最新版」と「フェッチ失敗」が同時表示されてしまう。
      updaterMessage.value = t('settings.updateUpToDate')
    }
  }

  async function onDownloadUpdate() {
    updaterMessage.value = null

    // メジャーバージョン跨ぎ確認
    if (pendingUpdateVersion.value) {
      const appInfo = await useAppInfo().load()
      // get_app_info が取れない場合 (通常 Tauri 起動中は発生しない) は、空バージョンで
      // メジャー跨ぎを誤判定しないよう中断する。直接 invoke していた頃の throw→中断と
      // 同じ「現在バージョン不明ならダウンロードへ進まない」挙動を維持する。
      if (!appInfo) return
      const isMajorJump = await invokeTauri<boolean>('check_update_is_major_jump', {
        currentVersion: appInfo.version,
        newVersion: pendingUpdateVersion.value,
      })
      if (isMajorJump) {
        const { ask } = await import('@tauri-apps/plugin-dialog')
        const proceed = await ask(
          t('settings.updateMajorJumpWarning', {
            version: pendingUpdateVersion.value,
          }),
          { title: t('settings.updateMajorJumpTitle'), kind: 'warning' },
        )
        if (!proceed) return
      }
    }

    const ok = await downloadUpdate()
    if (ok) {
      updaterMessage.value = t('settings.updateDownloadComplete')
      const { ask } = await import('@tauri-apps/plugin-dialog')
      const restart = await ask(t('settings.updateRelaunchAsk'), {
        title: t('settings.updateRelaunchTitle'),
        kind: 'info',
      })
      if (restart) await relaunchApp()
    }
  }

  return {
    updaterChecking,
    updaterDownloading,
    updaterAvailable,
    updaterMessage,
    updaterErrorDisplay,
    updaterProgress,
    updaterTotal,
    autoCheckHint,
    onForceRecheck,
    onCheckUpdate,
    onDownloadUpdate,
  }
}
