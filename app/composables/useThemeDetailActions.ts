/**
 * Library の詳細モーダル操作 (P08a Step 4 / L2)。
 *
 * `useThemes()` / `useWindowsSchemes()` / `useThemePreviews()` は内包。
 * `notify` は `useNotify` の auto-import をそのまま使う。
 */
import type { Ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'
import { useThemes } from '~/composables/useThemes'
import { useWindowsSchemes } from '~/composables/useWindowsSchemes'
import { useThemePreviews } from '~/composables/useThemePreviews'

export interface ThemeDetailActionsDeps {
  themes: Ref<ThemeCardData[]>
  reload: () => Promise<void>
  setError: (msg: string | null) => void
  requestApply: (id: string) => void
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useThemeDetailActions(deps: ThemeDetailActionsDeps) {
  const { themes, reload, setError, requestApply, t } = deps
  const {
    repackageTheme: repackageThemeIpc,
    duplicateTheme: duplicateThemeIpc,
    deleteTheme: deleteThemeIpc,
  } = useThemes()
  const { exportSchemeAsCursorpack } = useWindowsSchemes()
  const themePreviewCache = useThemePreviews()

  // 詳細モーダル制御。モーダルは画面に同時に 1 つしか出さない。
  const detailTheme = ref<ThemeCardData | null>(null)
  const detailPreviewMap = ref<Record<string, string> | null>(null)
  const detailPreviewDetails = ref<Record<string, RolePreviewDetail> | null>(null)
  // 詳細モーダルの二次アクション (edit/export/duplicate/delete) 実行中フラグ (LD8)。
  // 該当ボタンにスピナーを出し、実行中はグループを無効化する。
  const detailBusyAction = ref<'edit' | 'export' | 'duplicate' | 'delete' | null>(null)

  /**
   * カードのシェブロン押下で開く詳細モーダル。
   *
   * モーダルが共有されているのでプレビューマップは開いた瞬間にロードする。
   * `useThemePreviews` 側で IPC 結果がキャッシュされているので 2 回目以降は即時表示。
   */
  async function showDetails(id: string) {
    const found = themes.value.find((tt) => tt.id === id)
    if (!found) return
    detailTheme.value = found
    detailPreviewMap.value = null
    detailPreviewDetails.value = null
    // Windows システムスキームには ID が `windows:` プレフィックス付きでローカルテーマ
    // のキャッシュキーと衝突しないので、そのまま渡す。実体取得が無い場合は null のまま。
    // url 取得と詳細取得は同じキャッシュエントリを再利用する (in-flight 共有)。
    try {
      const [map, details] = await Promise.all([
        themePreviewCache.getMap(id),
        themePreviewCache.getDetails(id),
      ])
      detailPreviewMap.value = map
      detailPreviewDetails.value = details
    } catch (err) {
      console.warn('[Library] preview load for detail failed:', err)
    }
  }

  function closeDetails() {
    detailTheme.value = null
    detailPreviewMap.value = null
    detailPreviewDetails.value = null
  }

  /** 詳細モーダルから「適用」を選んだとき。確認モーダル経由で apply を実行する。 */
  function applyFromDetail(id: string) {
    closeDetails()
    requestApply(id)
  }

  /**
   * 詳細モーダルからの「Creator で編集」。テーマを再パッケージして一時ファイル化し、
   * Creator の bulk import 経路で開く。一時ファイルは Rust 側 (tempdir) ではなく
   * OS の TEMP に書き出し、Nuxt から `parse_cursorpack_for_creator` で読み込む。
   */
  async function editInCreator(id: string) {
    detailBusyAction.value = 'edit'
    try {
      const { tempDir, sep } = await import('@tauri-apps/api/path')
      const dir = await tempDir()
      const tempPath = `${dir}${sep()}_easycursorswap_edit_${Date.now()}.cursorpack`
      await repackageThemeIpc(id, tempPath)
      closeDetails()
      // Creator ページに遷移し、ロード対象のパスをクエリで渡す。Creator 側で
      // `editThemePath` クエリを拾って parse_cursorpack_for_creator を呼ぶ。
      await navigateTo({ path: '/creator', query: { editPath: tempPath } })
    } catch (err) {
      setError(
        t('library.errEditModeTransition', {
          detail: appErrorMessage(err),
        }),
      )
    } finally {
      detailBusyAction.value = null
    }
  }

  /** 詳細モーダルからの「複製」。`duplicate_theme` IPC で新 UUID を作りリロードする。 */
  async function duplicateTheme(id: string) {
    detailBusyAction.value = 'duplicate'
    try {
      await duplicateThemeIpc(id)
      closeDetails()
      await reload()
      void notify({
        title: 'EasyCursorSwap',
        body: t('library.notifyDuplicated', {
          name: themes.value.find((tt) => tt.id === id)?.name ?? t('library.fallbackThemeName'),
        }),
        level: 'success',
      })
    } catch (err) {
      setError(
        t('library.errDuplicate', {
          detail: appErrorMessage(err),
        }),
      )
    } finally {
      detailBusyAction.value = null
    }
  }

  /** 詳細モーダルからの「エクスポート」。`repackage_theme` で .cursorpack を保存する。 */
  async function exportTheme(id: string) {
    try {
      const target = themes.value.find((tt) => tt.id === id)
      if (!target) return
      const { save } = await import('@tauri-apps/plugin-dialog')
      const safeName = target.name.replace(/[^\p{L}\p{N}_-]+/gu, '_').slice(0, 64) || 'theme'
      const outputPath = await save({
        defaultPath: `${safeName}.cursorpack`,
        filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
      })
      if (!outputPath) return
      detailBusyAction.value = 'export'
      let bytes: number | null = null
      if (target.kind === 'system') {
        // Windows レジストリスキームはローカルテーマディレクトリを持たないので
        // 専用の export_windows_scheme_as_cursorpack を経由する。`%SystemRoot%`
        // 配下の .cur / .ani をそのまま zip 化する設計。
        bytes = await exportSchemeAsCursorpack(target.name, outputPath)
      } else {
        bytes = await repackageThemeIpc(id, outputPath)
      }
      void notify({
        title: 'EasyCursorSwap',
        body: t('library.notifyExported', {
          name: target.name,
          bytes: bytes ?? t('library.bytesUnknown'),
        }),
        level: 'success',
      })
    } catch (err) {
      setError(
        t('library.errExport', {
          detail: appErrorMessage(err),
        }),
      )
    } finally {
      detailBusyAction.value = null
    }
  }

  /** 詳細モーダルからの「削除」。確認ダイアログを挟んでから `delete_theme` を実行。 */
  async function deleteTheme(id: string) {
    const target = themes.value.find((tt) => tt.id === id)
    if (!target) return
    // UI で削除ボタンを disabled にしているが、IPC 直叩きや競合状態 (削除直前に
    // 別経路で apply されたケース) を防ぐため二重チェック。
    if (target.isActive) {
      setError(t('library.errDeleteActive'))
      return
    }
    // ネイティブ confirm はテストしづらいが Tauri WebView では機能するので暫定使用。
    // 将来的には専用の確認モーダルに置き換える。
    const ok = window.confirm(t('library.confirmDeleteMsg', { name: target.name }))
    if (!ok) return
    detailBusyAction.value = 'delete'
    try {
      await deleteThemeIpc(id)
      closeDetails()
      await reload()
      void notify({
        title: 'EasyCursorSwap',
        body: t('library.notifyDeleted', { name: target.name }),
        level: 'info',
      })
    } catch (err) {
      setError(
        t('library.errDelete', {
          detail: appErrorMessage(err),
        }),
      )
    } finally {
      detailBusyAction.value = null
    }
  }

  return {
    detailTheme,
    detailPreviewMap,
    detailPreviewDetails,
    detailBusyAction,
    showDetails,
    closeDetails,
    applyFromDetail,
    editInCreator,
    duplicateTheme,
    exportTheme,
    deleteTheme,
  }
}
