/**
 * Marketplace 自動提出フローのフロント側コーディネータ。
 *
 * `submit_theme_auto` IPC を呼び、進捗イベント `submit:progress` を購読して
 * リアクティブな stage を提供する。終了時 (成功/失敗どちらも) にリスナーを解除する。
 */
import { ref } from 'vue'
import { invokeTauri, listenTauri } from '~/composables/useTauri'
import type { SubmitResult, SubmitStage } from '~/types/githubAuth'

export function useMarketplaceSubmit() {
  const stage = ref<SubmitStage | null>(null)
  const errorMsg = ref<string | null>(null)
  const busy = ref(false)

  async function submit(themeId: string): Promise<SubmitResult> {
    busy.value = true
    stage.value = null
    errorMsg.value = null
    const unlisten = await listenTauri<string>('submit:progress', (e) => {
      stage.value = e.payload as SubmitStage
    })
    try {
      const result = await invokeTauri<SubmitResult>('submit_theme_auto', {
        themeId,
      })
      if (!result) {
        // Tauri 未接続環境 (dev だけ)
        throw new Error('Tauri ランタイム未接続')
      }
      return result
    } catch (e) {
      errorMsg.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      unlisten()
      busy.value = false
    }
  }

  return { stage, errorMsg, busy, submit }
}
