/**
 * Tauri Updater 連携 composable (Phase 8-4)
 *
 * 仕様書「§5 自動アップデート」より:
 *  - 自動アップデートは設定で OFF 可能 (常駐型のため強制せず)
 *  - メジャーバージョン跨ぎ (v1 → v2) は自動更新しない (本 composable では未実装)
 *  - 3 回連続起動失敗で旧バイナリへロールバック (将来実装)
 *
 * 本 composable は手動チェック / ダウンロード / インストールを担当。
 * `dialog: false` で標準ダイアログを抑制し、UI 側で進捗表示する。
 */
import { ref } from 'vue'

export interface UpdateInfo {
  version: string
  currentVersion: string
  date?: string
  body?: string
}

const checking = ref(false)
const downloading = ref(false)
const installed = ref(false)
const available = ref<UpdateInfo | null>(null)
const error = ref<string | null>(null)
const progressBytes = ref(0)
const totalBytes = ref(0)

async function getUpdaterApi() {
  try {
    return await import('@tauri-apps/plugin-updater')
  } catch (e) {
    console.warn('[useUpdater] plugin not available:', e)
    return null
  }
}

/** 手動で更新を確認する。 */
async function check(): Promise<UpdateInfo | null> {
  const api = await getUpdaterApi()
  if (!api) return null
  checking.value = true
  error.value = null
  try {
    const result = await api.check()
    if (result?.available) {
      const info: UpdateInfo = {
        version: result.version,
        currentVersion: result.currentVersion,
        date: result.date ?? undefined,
        body: result.body ?? undefined,
      }
      available.value = info
      return info
    }
    available.value = null
    return null
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    return null
  } finally {
    checking.value = false
  }
}

/** 利用可能な更新をダウンロード + インストール。完了後の再起動はユーザー判断に委ねる。 */
async function downloadAndInstall(): Promise<boolean> {
  const api = await getUpdaterApi()
  if (!api) return false
  downloading.value = true
  error.value = null
  progressBytes.value = 0
  totalBytes.value = 0
  try {
    const update = await api.check()
    if (!update?.available) return false

    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          totalBytes.value = event.data.contentLength ?? 0
          break
        case 'Progress':
          progressBytes.value += event.data.chunkLength ?? 0
          break
        case 'Finished':
          installed.value = true
          break
      }
    })
    return true
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    return false
  } finally {
    downloading.value = false
  }
}

/** 再起動して新バージョンを反映。 */
async function relaunch(): Promise<void> {
  try {
    const { relaunch: tauriRelaunch } = await import('@tauri-apps/plugin-process')
    await tauriRelaunch()
  } catch {
    // process プラグインがない場合は黙って諦める
  }
}

export function useUpdater() {
  return {
    checking,
    downloading,
    installed,
    available,
    error,
    progressBytes,
    totalBytes,
    check,
    downloadAndInstall,
    relaunch,
  }
}
