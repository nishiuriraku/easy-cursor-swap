/**
 * Creator の「複数ファイル / .cursorpack 取り込み → プレビュー → 適用」フロー制御。
 *
 * 役割:
 *  - BulkImportPreviewModal の open/close 状態と入力データ (`bulkResolved` / `bulkCursorpack`) を保持
 *  - dispatchBulkPaths: 入力パス群を「単一 fast-path」「.cursorpack 解析」「複数 bulk_resolve」に振り分け
 *  - applyBulkImport: モーダル apply イベントを受けて creatorAssets / メタデータに反映
 *  - cancelBulkImport: モーダルを閉じる
 *
 * 依存: useBulkImport, useCreatorImport の単一 fast-path 関数, creatorAssets, meta refs, 各種 UI ref。
 */
import { ref, type Ref } from 'vue'
import type { useBulkImport, ResolvedAsset, ParsedCursorpack } from './useBulkImport'
import type { useCreatorAssets } from './useCreatorAssets'
import type { ApplyPayload } from '~/components/creator/BulkImportPreviewModal.vue'

type SaveModalMode = 'file' | 'library' | 'libraryAndApply'

export interface CreatorBulkImportFlowDeps {
  bulkImport: ReturnType<typeof useBulkImport>
  creatorAssets: ReturnType<typeof useCreatorAssets>
  filledRoles: Set<string>
  filledSizesByRole: Ref<Record<string, number[]>>
  sourceThemeId: Ref<string | null>
  /** メタデータ refs (cursorpack 取込時に上書き) */
  metaName: Ref<string>
  metaNameEn: Ref<string>
  metaAuthor: Ref<string>
  metaVersion: Ref<string>
  metaDescription: Ref<string>
  /** SaveDestinationModal の open/default 制御 (Apply Immediately フロー用) */
  saveModalOpen: Ref<boolean>
  saveModalDefault: Ref<SaveModalMode>
  /** インポート系の進捗・通知 (useCreatorImport が所有) */
  importBusy: Ref<boolean>
  importMessage: Ref<string | null>
  sanitizedRemovals: Ref<string[]>
  /** 単一ファイル fast-path 用 (useCreatorImport が提供) */
  pickRasterFromPath: (path: string, ext: string) => Promise<void>
  pickCursorFromPath: (path: string) => Promise<void>
}

export function useCreatorBulkImportFlow(deps: CreatorBulkImportFlowDeps) {
  const {
    bulkImport,
    creatorAssets,
    filledRoles,
    filledSizesByRole,
    sourceThemeId,
    metaName,
    metaNameEn,
    metaAuthor,
    metaVersion,
    metaDescription,
    saveModalOpen,
    saveModalDefault,
    importBusy,
    importMessage,
    sanitizedRemovals,
    pickRasterFromPath,
    pickCursorFromPath,
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
   *  - 単一ファイル (1 件) で `.cursorpack` 以外 (= png/svg/cur/ico) の場合は、
   *    bulk preview を経由せず **現在編集中のロールに直接代入** する fast-path に流す。
   *  - 単一 `.ani` は static fast-path には乗せず、bulk preview 経路に通す
   *    (アニメ再生 + ホットスポット調整 UI が必要なため)
   *  - `.cursorpack` は他のファイルと一緒に取り込めない (パッケージ単位の取込なので)
   *  - 複数 `.cursorpack` の同時取込もサポートしない
   */
  async function dispatchBulkPaths(paths: string[]) {
    // Fast-path: 1 件かつ非 cursorpack なら現在ロールに直接代入
    if (paths.length === 1) {
      const p = paths[0]!
      const ext = p.split('.').pop()?.toLowerCase() ?? ''
      if (ext === 'cur' || ext === 'ico') {
        importBusy.value = true
        importMessage.value = null
        try {
          await pickCursorFromPath(p)
        } catch (err) {
          importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
        } finally {
          importBusy.value = false
        }
        return
      }
      if (ext === 'png' || ext === 'svg') {
        importBusy.value = true
        importMessage.value = null
        sanitizedRemovals.value = []
        try {
          await pickRasterFromPath(p, ext)
        } catch (err) {
          importMessage.value = `失敗: ${err instanceof Error ? err.message : String(err)}`
        } finally {
          importBusy.value = false
        }
        return
      }
      // .cursorpack / .ani は下の bulk preview ロジックに fallthrough
    }

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
      // filledSizesByRole も更新 (UI バッジ用)
      const sizes = asset.sized ? Array.from(asset.sized.keys()) : [asset.primarySize]
      filledSizesByRole.value = { ...filledSizesByRole.value, [roleId]: sizes }
      filledRoles.add(roleId)
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

    // ✓ 「すぐシステムに反映」がチェックされていれば SaveDestinationModal を
    // Library+Apply 既定で開く。ユーザーは [保存] を押すだけで適用まで進める。
    if (payload.applyImmediately) {
      saveModalDefault.value = 'libraryAndApply'
      saveModalOpen.value = true
    }
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
