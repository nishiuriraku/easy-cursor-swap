/**
 * Creator の `.cursorpack` エクスポートフローを creator.vue から分離した composable。
 *
 * 役割:
 *  - SaveDestinationModal の submit を受けて `export_cursorpack_streamed` IPC を呼ぶ
 *  - build-progress イベントを購読して `exportProgress` に反映
 *  - apply 失敗時の `failedApplyThemeId` と retry CTA
 *  - exportMessage の自動消去 (failedApplyThemeId が立っている間は維持)
 *
 * 依存: meta データ ref 群と `creatorAssets` の集計プロパティ、t()。
 */
import type { Ref } from 'vue'

/** ストリームエクスポート時の進捗状態 */
export interface BuildProgress {
  buildId: string
  stage: 'role' | 'package' | 'sign' | 'done' | 'cancelled' | 'error'
  current: number
  total: number
  message: string | null
}

interface ExportResult {
  theme_id: string
  size_bytes: number
  signed: boolean
  key_id: string | null
  applied: boolean
  apply_error: string | null
}

/** SaveDestinationModal の submit ペイロード (creator.vue から受け取る形)。 */
export interface SaveSubmitPayload {
  destination: 'file' | 'library' | 'libraryAndApply'
  filePath: string | null
  effectiveName: string
  sign: boolean
  overwriteExisting: boolean
}

type ResampleMode = 'lanczos' | 'nearest' | 'auto'

interface CreatorAssetsForExport {
  assignedRoleCount: Ref<number>
  arrowAssigned: Ref<boolean>
  toExportPayload: (resample: ResampleMode) => unknown[]
}

export interface CreatorExportDeps {
  creatorAssets: CreatorAssetsForExport
  metaNameEn: Ref<string>
  metaAuthor: Ref<string>
  metaVersion: Ref<string>
  /**
   * Creator UI の説明欄。空文字のときは IPC で null を送り、
   * Rust 側で theme.json から description フィールドごと省略する。
   */
  metaDescription: Ref<string>
  sourceThemeId: Ref<string | null>
  shadowEnabled: Ref<boolean>
  resample: Ref<ResampleMode>
  t: (key: string) => string
}

const TOAST_AUTO_DISMISS_MS = 3500

export function useCreatorExport(deps: CreatorExportDeps) {
  const {
    creatorAssets,
    metaNameEn,
    metaAuthor,
    metaVersion,
    metaDescription,
    sourceThemeId,
    shadowEnabled,
    resample,
    t,
  } = deps

  const exportBusy = ref(false)
  const exportMessage = ref<string | null>(null)
  /** Library 保存成功 + apply 失敗時の theme_id。retry ボタン活性化と再 apply 呼出に使う。 */
  const failedApplyThemeId = ref<string | null>(null)
  const exportProgress = ref<BuildProgress | null>(null)
  const currentBuildId = ref<string | null>(null)

  // 上書き保存後にライブラリ側で古い Blob URL が表示され続ける問題への対策。
  // useThemePreviews の cache はモジュール singleton なので、ここから invalidate すると
  // Library / 詳細モーダル側の `getMap` / `getDetails` が次回呼出で再フェッチする。
  const themePreviewCache = useThemePreviews()

  // exportMessage 自動消去。failedApplyThemeId が残っている場合は retry の動線を残すため消さない。
  let exportMessageTimer: ReturnType<typeof setTimeout> | null = null
  watch(exportMessage, (msg) => {
    if (exportMessageTimer) {
      clearTimeout(exportMessageTimer)
      exportMessageTimer = null
    }
    if (msg !== null && failedApplyThemeId.value === null) {
      exportMessageTimer = setTimeout(() => {
        exportMessage.value = null
        exportMessageTimer = null
      }, TOAST_AUTO_DISMISS_MS)
    }
  })

  /** 進行中のエクスポートを中止する。Rust 側は次のチェックポイントで終了する。 */
  async function cancelExport() {
    if (!currentBuildId.value) return
    try {
      await invokeTauri('cancel_build', { buildId: currentBuildId.value })
    } catch {
      // ignore
    }
  }

  /**
   * SaveDestinationModal の submit を受けて Rust 側 export_cursorpack_streamed を呼ぶ。
   * destination ごとにトーストメッセージを切り替え、apply 失敗時は warning + retry。
   */
  async function executeSave(payload: SaveSubmitPayload) {
    if (creatorAssets.assignedRoleCount.value === 0) {
      exportMessage.value = '少なくとも 1 役割に画像を割り当ててください'
      return
    }
    if (!creatorAssets.arrowAssigned.value) {
      exportMessage.value = 'Arrow ロールは必須です'
      return
    }
    exportBusy.value = true
    exportMessage.value = null
    exportProgress.value = null

    let unlisten: (() => void) | null = null
    try {
      const buildId = `build-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
      currentBuildId.value = buildId

      try {
        const { listen } = await import('@tauri-apps/api/event')
        unlisten = await listen<BuildProgress>('build-progress', (e) => {
          if (e.payload.buildId === buildId) exportProgress.value = e.payload
        })
      } catch {
        // Web 開発時は購読をスキップ
      }

      const roles = creatorAssets.toExportPayload(resample.value)

      const destination =
        payload.destination === 'file'
          ? { kind: 'file', path: payload.filePath! }
          : { kind: 'library', applyAfter: payload.destination === 'libraryAndApply' }

      const result = await invokeTauri<ExportResult>('export_cursorpack_streamed', {
        req: {
          buildId,
          nameJa: payload.effectiveName,
          nameEn: metaNameEn.value || null,
          author: metaAuthor.value || null,
          version: metaVersion.value,
          // Rust 側で trim + 空文字 None 化されるが、明示的に "" → null にしておくと
          // IPC ペイロードの意図が明確 (ja.ts の placeholder と区別がつく)。
          description: metaDescription.value.trim() || null,
          requiresOsShadow: shadowEnabled.value,
          roles,
          destination,
          existingThemeId:
            payload.overwriteExisting && sourceThemeId.value ? sourceThemeId.value : null,
          sign: payload.sign,
        },
      })

      if (!result) throw new Error('エクスポート結果が空でした')

      // Library 系の保存が完了した場合、同一 UUID で上書きされた可能性があるので
      // プレビューキャッシュを破棄する (Library カード / 詳細モーダルの古い PNG 防止)。
      // File 保存は ~/.custom_cursors/ を触らないので対象外。
      if (payload.destination !== 'file') {
        themePreviewCache.invalidate(result.theme_id)
      }

      if (result.apply_error) {
        exportMessage.value = t('saveModal.toastAppliedFailed').replace(
          '{error}',
          result.apply_error,
        )
        failedApplyThemeId.value = result.theme_id
      } else if (result.applied) {
        exportMessage.value = t('saveModal.toastSavedAndApplied')
        failedApplyThemeId.value = null
      } else if (payload.destination === 'file') {
        exportMessage.value = t('saveModal.toastSavedFile').replace('{path}', payload.filePath!)
        failedApplyThemeId.value = null
      } else {
        exportMessage.value = t('saveModal.toastSavedLibrary')
        failedApplyThemeId.value = null
      }
    } catch (err) {
      exportMessage.value = `エクスポート失敗: ${err instanceof Error ? err.message : String(err)}`
    } finally {
      if (unlisten) unlisten()
      currentBuildId.value = null
      exportBusy.value = false
      setTimeout(() => {
        if (!exportBusy.value) exportProgress.value = null
      }, 3000)
    }
  }

  /**
   * apply 失敗後の再試行 CTA。バナーから呼ばれる。
   * 失敗 ID をクリアして apply_theme を再度叩く。
   */
  async function retryApply() {
    if (!failedApplyThemeId.value) return
    const themeId = failedApplyThemeId.value
    failedApplyThemeId.value = null
    try {
      await useThemes().applyTheme(themeId)
      exportMessage.value = t('saveModal.toastSavedAndApplied')
    } catch (err) {
      exportMessage.value = `再試行失敗: ${err instanceof Error ? err.message : String(err)}`
      failedApplyThemeId.value = themeId // 再再試行のため復元
    }
  }

  return {
    exportBusy,
    exportMessage,
    failedApplyThemeId,
    exportProgress,
    currentBuildId,
    cancelExport,
    executeSave,
    retryApply,
  }
}
