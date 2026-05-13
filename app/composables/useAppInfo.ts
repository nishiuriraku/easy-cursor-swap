/**
 * `get_app_info` IPC の結果 (version / cursors_dir / config_dir / os_version) を
 * モジュールスコープでキャッシュする小さな composable。
 *
 * 設計意図:
 *  - アプリ起動中に変わらない情報なので 1 回 fetch して以降はキャッシュを返す。
 *  - 同時に複数のコンポーネントが load() を呼んでも IPC は 1 回だけ走るよう
 *    in-flight Promise を共有する。
 *  - Tauri 未接続 (web preview) では IPC が失敗するので、その場合は null を返し
 *    呼び出し元は `info.value?.version ?? '—'` のように fallback すること。
 *
 * 既存呼び出し元 (settings.vue の onDownloadUpdate) は直接 invokeTauri しているが、
 * 呼び出しタイミングが異なる (起動時 vs ユーザー操作時) ので併存可能。
 */
import { ref } from 'vue'
import { invokeTauri } from './useTauri'

export interface AppInfo {
  version: string
  cursors_dir: string
  config_dir: string
  os_version: string
}

const cached = ref<AppInfo | null>(null)
let inFlight: Promise<AppInfo | null> | null = null

async function load(): Promise<AppInfo | null> {
  if (cached.value) return cached.value
  if (inFlight) return inFlight
  inFlight = (async () => {
    try {
      const info = await invokeTauri<AppInfo>('get_app_info')
      cached.value = info ?? null
      return cached.value
    } catch (err) {
      // Tauri 未接続 (web preview) では失敗するので握る。
      console.warn('[useAppInfo] get_app_info failed:', err)
      return null
    } finally {
      inFlight = null
    }
  })()
  return inFlight
}

export function useAppInfo() {
  return { info: cached, load }
}
