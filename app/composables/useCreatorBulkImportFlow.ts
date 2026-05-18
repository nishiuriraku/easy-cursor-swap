/**
 * Creator の「複数ファイル / .cursorpack 取り込み → プレビュー → 適用」フロー制御。
 *
 * 役割:
 *  - BulkImportPreviewModal の open/close 状態と入力データ (`bulkResolved` / `bulkCursorpack`) を保持
 *  - dispatchBulkPaths: 入力パス群を「.cursorpack 解析」または「bulk_resolve」に振り分け
 *  - applyBulkImport: モーダル apply イベントを受けて creatorAssets / メタデータに反映
 *  - cancelBulkImport: モーダルを閉じる
 *
 * 依存: useBulkImport, creatorAssets, meta refs, 各種 UI ref。
 */
import type { Ref } from 'vue'
import type { useBulkImport, ResolvedAsset, ParsedCursorpack } from './useBulkImport'
import type { useCreatorAssets } from './useCreatorAssets'
import type { ApplyPayload } from './useBulkImportPreviewState'

export interface CreatorBulkImportFlowDeps {
  bulkImport: ReturnType<typeof useBulkImport>
  creatorAssets: ReturnType<typeof useCreatorAssets>
  sourceThemeId: Ref<string | null>
  /** メタデータ refs (cursorpack 取込時に上書き) */
  metaName: Ref<string>
  metaNameEn: Ref<string>
  metaAuthor: Ref<string>
  metaVersion: Ref<string>
  metaDescription: Ref<string>
  /** インポート系の進捗・通知 (useCreatorImport が所有) */
  importBusy: Ref<boolean>
  importMessage: Ref<string | null>
  sanitizedRemovals: Ref<string[]>
}

export function useCreatorBulkImportFlow(deps: CreatorBulkImportFlowDeps) {
  const {
    bulkImport,
    creatorAssets,
    sourceThemeId,
    metaName,
    metaNameEn,
    metaAuthor,
    metaVersion,
    metaDescription,
    importMessage,
    sanitizedRemovals,
  } = deps
  const { setAsset } = creatorAssets

  const bulkModalOpen = ref(false)
  const bulkResolved = ref<ResolvedAsset[] | null>(null)
  const bulkCursorpack = ref<ParsedCursorpack | null>(null)
  const bulkSourceLabel = ref('')

  /**
   * 拡張子分岐の本体。`pickBulkAuto` から、または将来のドラッグ&ドロップから呼ばれる想定。
   *
   * 設計判断:
   *  - 「一括インポート」エントリは件数に関わらず **常に bulk preview を通す**。
   *    過去には 1 件選択時に現在ロールへ直接代入する fast-path が存在したが、
   *    ユーザーから見て「一括インポートを押したのにプレビューが開かず Arrow だけに
   *    上書きされる」という不可解な挙動になっていたため削除した。単一ファイルでも
   *    ロール推定 / ロール選択 UI / overwrite ガードを通すのが一貫した体験。
   *  - `.cursorpack` は他のファイルと一緒に取り込めない (パッケージ単位の取込なので)
   *  - 複数 `.cursorpack` の同時取込もサポートしない
   *  - `sanitizedRemovals` は単一 SVG fast-path のみが使っていたので、bulk 経路では
   *    そもそも参照しない (BulkImportPreviewModal が SVG sanitize メッセージを扱う)。
   */
  async function dispatchBulkPaths(paths: string[]) {
    // 単一ファイルでも fast-path は使わず、cursorpack 判定 → bulk_resolve に進む。
    // (`sanitizedRemovals` は単一 SVG 経路でのみ使っていたので、ここではクリアだけ行う)
    sanitizedRemovals.value = []

    const packs = paths.filter((p) => p.toLowerCase().endsWith('.cursorpack'))
    const others = paths.filter((p) => !p.toLowerCase().endsWith('.cursorpack'))

    if (packs.length === 1 && others.length === 0) {
      try {
        const parsed = await bulkImport.parseCursorpack(packs[0]!)
        bulkCursorpack.value = parsed
        bulkResolved.value = null
        bulkSourceLabel.value = `📦 ${packs[0]!.split(/[\\/]/).pop()}`
        bulkModalOpen.value = true
        // `.cursorpack` を取り込んだ瞬間、編集対象のソースが入れ替わる。
        // `?editPath` で引き継いだ UUID は無効になるのでクリア (SaveDestinationModal の
        // 誤 overwrite 提案を防ぐ)。
        sourceThemeId.value = null
      } catch (err) {
        importMessage.value = `cursorpack 取り込み失敗: ${err instanceof Error ? err.message : String(err)}`
      }
      return
    }

    if (packs.length >= 2) {
      importMessage.value = '.cursorpack は 1 つだけ選択してください'
      return
    }

    if (packs.length === 1 && others.length > 0) {
      importMessage.value =
        '.cursorpack は他のファイルと同時に取り込めません (.cursorpack 以外を取り込みました)'
    }

    if (others.length === 0) return
    await runBulkResolve(others, false, `${others.length} 個のファイル`)
  }

  async function runBulkResolve(paths: string[], recursive: boolean, label: string) {
    try {
      const r = await bulkImport.resolveAssets(paths, recursive)
      if (r.assets.length === 0) {
        importMessage.value = '対応ファイルが見つかりません'
        return
      }
      bulkResolved.value = r.assets
      bulkCursorpack.value = null
      bulkSourceLabel.value = label
      bulkModalOpen.value = true
      if (r.failures.length > 0) {
        importMessage.value = `${r.failures.length} 件のファイルをスキップしました`
      }
    } catch (err) {
      importMessage.value = `一括インポート失敗: ${err instanceof Error ? err.message : String(err)}`
    }
  }

  /** BulkImportPreviewModal の apply イベントを受けて creator state に反映する。 */
  function applyBulkImport(payload: ApplyPayload) {
    for (const { roleId, asset } of payload.roleAssets) {
      setAsset(roleId, asset)
    }

    // メタデータ反映 (.cursorpack のみ)
    if (payload.metadata && payload.metadataChoice !== 'keep') {
      metaName.value = payload.metadata.nameJa ?? metaName.value
      if (payload.metadataChoice === 'overwrite') {
        metaNameEn.value = payload.metadata.nameEn ?? metaNameEn.value
        metaAuthor.value = payload.metadata.author ?? metaAuthor.value
        metaVersion.value = payload.metadata.version ?? metaVersion.value
        metaDescription.value = payload.metadata.description ?? metaDescription.value
      }
    }

    bulkModalOpen.value = false
    importMessage.value = `${payload.roleAssets.length} 件のロールを適用しました`
  }

  function cancelBulkImport() {
    bulkModalOpen.value = false
    bulkResolved.value = null
    bulkCursorpack.value = null
  }

  return {
    bulkModalOpen,
    bulkResolved,
    bulkCursorpack,
    bulkSourceLabel,
    dispatchBulkPaths,
    runBulkResolve,
    applyBulkImport,
    cancelBulkImport,
  }
}
