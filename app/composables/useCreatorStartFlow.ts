/**
 * Creator の新規作成 / 複製フロー (P08a Step 2 / C5)。
 *
 * `creator.vue` から「スタート画面の CTA + テーマピッカー + `?editPath` ロード」
 * を移動したもの。ページが所有する ref を deps で受け取り、ハンドラを返す。
 */
import type { Ref } from 'vue'
import type { ParsedCursorpack, ResolvedAsset, useBulkImport } from './useBulkImport'
import { useThemes } from './useThemes'

export type CreatorStage = 'start' | 'editing'

export interface CreatorBulkFlow {
  bulkModalOpen: Ref<boolean>
  bulkCursorpack: Ref<ParsedCursorpack | null>
  bulkResolved: Ref<ResolvedAsset[] | null>
  bulkSourceLabel: Ref<string>
  dispatchBulkPaths: (paths: string[]) => Promise<unknown>
}

export interface CreatorStartFlowDeps {
  stage: Ref<CreatorStage>
  newThemeModalOpen: Ref<boolean>
  themePickerOpen: Ref<boolean>
  themePickerSelected: Ref<string | null>
  sourceThemeId: Ref<string | null>
  saveModalDefault: Ref<'file' | 'library' | 'libraryAndApply'>
  bulkFlow: CreatorBulkFlow
  bulkImport: Pick<ReturnType<typeof useBulkImport>, 'parseCursorpack'>
  pickBulkAuto: () => Promise<unknown>
  pickBulkFolder: () => Promise<unknown>
  refreshPickerThemes: () => Promise<unknown>
  importMessage: Ref<string | null>
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useCreatorStartFlow(deps: CreatorStartFlowDeps) {
  const {
    stage,
    newThemeModalOpen,
    themePickerOpen,
    themePickerSelected,
    sourceThemeId,
    saveModalDefault,
    bulkFlow,
    bulkImport,
    importMessage,
    t,
  } = deps

  /**
   * ヒーロー画面の「新規作成」CTA ハンドラ。
   * モーダルを開いてベース画像を選ばせる (デザイン要件)。
   */
  function onStartNew() {
    newThemeModalOpen.value = true
  }

  /**
   * 「ファイル/パックから取り込む」CTA — bulkAuto を起動。
   * モーダルを閉じてから dispatch する。プレビューモーダルが開いたら editing へ遷移する。
   */
  async function onNewThemePickFiles() {
    newThemeModalOpen.value = false
    await deps.pickBulkAuto()
    if (bulkFlow.bulkModalOpen.value) {
      stage.value = 'editing'
    }
  }

  /** 「フォルダから取り込む」CTA。 */
  async function onNewThemePickFolder() {
    newThemeModalOpen.value = false
    await deps.pickBulkFolder()
    if (bulkFlow.bulkModalOpen.value) {
      stage.value = 'editing'
    }
  }

  /** モーダルから「画像なしで開始」を選んだ場合は従来通りの空エディタに遷移。 */
  function onNewThemeStartEmpty() {
    newThemeModalOpen.value = false
    stage.value = 'editing'
  }

  function onNewThemeCancel() {
    newThemeModalOpen.value = false
  }

  /**
   * 「既存テーマを複製して編集」CTA ハンドラ。
   *
   * 1. ライブラリのテーマ一覧をロードしてピッカーモーダルを開く
   * 2. 選択されたテーマを `repackage_theme` で一時 `.cursorpack` 化
   * 3. 既存の bulk preview modal 経路 (parseCursorpack) に流して editing へ遷移
   *
   * 詳細モーダルの `editInCreator` と同じ IPC を使うので、ロール衝突解決や
   * メタデータ反映の挙動はそちらと統一される。
   */
  async function onDuplicateExistingFromStart() {
    // 既存テーマの「複製」を起点にした新規作成セッション。`?editPath` で引き継いだ
    // ソース UUID は無効になるので、ピッカーを開く時点でクリアしておく
    // (SaveDestinationModal が誤って元テーマへの overwrite を提案するのを防ぐ)。
    sourceThemeId.value = null
    await deps.refreshPickerThemes()
    themePickerSelected.value = null
    themePickerOpen.value = true
  }

  async function onThemePickerSelect(id: string | null) {
    themePickerOpen.value = false
    if (!id) return
    try {
      const { tempDir, sep } = await import('@tauri-apps/api/path')
      const dir = await tempDir()
      const tempPath = `${dir}${sep()}_easycursorswap_dup_${Date.now()}.cursorpack`
      await useThemes().repackageTheme(id, tempPath)
      await bulkFlow.dispatchBulkPaths([tempPath])
      if (bulkFlow.bulkModalOpen.value) {
        stage.value = 'editing'
      }
    } catch (err) {
      importMessage.value = t('creator.errDuplicateThemeFailed', {
        detail: err instanceof Error ? err.message : String(err),
      })
      stage.value = 'editing'
    }
  }

  function onThemePickerCancel() {
    themePickerOpen.value = false
  }

  /**
   * `?editPath=...` で渡された `.cursorpack` を自動ロードして editing を開く。
   * ライブラリの「Creator で編集」からの遷移用。一時ファイルなので読み込み後に
   * 放置しても OS が TEMP を整理する (明示削除なし)。
   */
  async function loadFromEditPath(editPath: string) {
    try {
      const parsed = await bulkImport.parseCursorpack(editPath)
      bulkFlow.bulkCursorpack.value = parsed
      bulkFlow.bulkResolved.value = null
      bulkFlow.bulkSourceLabel.value = t('creator.bulkSourceEditing')
      bulkFlow.bulkModalOpen.value = true
      stage.value = 'editing'
      // `?editPath` 経由のみ元テーマ ID を保持。SaveDestinationModal が
      // 「上書き / 複製」セクションを出すトリガにも使う。
      sourceThemeId.value = parsed.metadata.id ?? null
      // `?editPath` 由来のテーマは「編集 → 再適用」が典型。デフォルトを Library+Apply に。
      saveModalDefault.value = 'libraryAndApply'
    } catch (err) {
      importMessage.value = t('creator.errEditLoadFailed', {
        detail: err instanceof Error ? err.message : String(err),
      })
      stage.value = 'editing'
    }
  }

  return {
    onStartNew,
    onNewThemePickFiles,
    onNewThemePickFolder,
    onNewThemeStartEmpty,
    onNewThemeCancel,
    onDuplicateExistingFromStart,
    onThemePickerSelect,
    onThemePickerCancel,
    loadFromEditPath,
  }
}
