/**
 * BulkImportPreviewModal の割当ライフサイクル全体を担う state machine composable。
 *
 * 元の `BulkImportPreviewModal.vue` (579 行) 内で 200 行以上を占めていた
 * matches / unmatched の 2 配列管理 + watch による初期化 + ロール割当ての三方移動
 * (matches ↔ unmatched, swap) を分離する。
 *
 * 設計:
 *  - props は reactive なので、watch を発火させるため getter (() => value) で受ける。
 *  - previewUrl は per-asset で 1 つ作成し、matches / unmatched 間で「同じ url を引き継ぐ」
 *    (revoke しない)。modal を閉じたとき / unmount 時にまとめて revoke する。
 *  - apply 時の payload 組立 (cursorpack 経路 / 通常経路 + new-role hotspot 補正) は
 *    `buildApplyPayload()` に閉じ込めて、SFC 側はそれを emit するだけ。
 *
 * 注意: per-call インスタンス (singleton ではない)。BulkImportPreviewModal で 1 回だけ呼ぶ前提。
 */
import type { MatchCandidate } from '~/composables/useRoleMatcher'
import type { ResolvedAsset, ParsedCursorpack } from '~/composables/useBulkImport'
import type { RoleAsset, SizedAsset } from '~/composables/useCreatorAssets'

/**
 * 1 ロールに割当て済みのアセット。
 *
 * 確信度パーセントと採用/スキップトグルは廃止。`matches` に入っているもの = apply 対象。
 * 「適用したくない」場合は ✕ ボタンで未マッチプールへ戻すと、apply 時に無視される。
 */
export interface PendingMatch {
  role: string
  asset: ResolvedAsset
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  previewUrl: string
  /**
   * `.ani` 用の Uint8Array 化済みフレーム列。Vue 3 のテンプレートは Uint8Array を
   * グローバル名として認識しないため、`new Uint8Array(...)` をテンプレート式で書くと
   * 「Property "Uint8Array" was accessed during render but is not defined on instance」
   * の warn を出す。スクリプト側で 1 度だけ変換しておいて子コンポーネントへ渡す。
   */
  aniFramesU8: readonly Uint8Array[] | null
}

/**
 * まだロールに割当てられていないファイル (未マッチプール)。
 *
 * ドロップダウンでロール選択された瞬間に `matches` へ移動する eager モデルなので、
 * 「選択された未マッチ」という中間状態は持たない。
 */
export interface UnmatchedFile {
  asset: ResolvedAsset
  previewUrl: string
  aniFramesU8: readonly Uint8Array[] | null
}

/** SFC が emit する apply イベントの payload 型。 */
export interface ApplyPayload {
  /** 確定したロール → アセット。useCreatorAssets.setAsset() で書き込む。 */
  roleAssets: Array<{ roleId: string; asset: RoleAsset }>
  metadataChoice: 'keep' | 'overwrite' | 'name-only'
  metadata: ParsedCursorpack['metadata'] | null
}

interface Deps {
  open: () => boolean
  resolved: () => ResolvedAsset[] | null
  cursorpack: () => ParsedCursorpack | null
  existingRoles: () => Set<string>
}

function toAniFramesU8(asset: ResolvedAsset): readonly Uint8Array[] | null {
  return asset.ani ? asset.ani.framePngs.map((b) => new Uint8Array(b)) : null
}

export function useBulkImportPreviewState(deps: Deps) {
  const matches = ref<PendingMatch[]>([])
  const unmatched = ref<UnmatchedFile[]>([])
  const metadataChoice = ref<'keep' | 'overwrite' | 'name-only'>('overwrite')

  // 作成した Blob URL は per-asset 1 つで matches/unmatched を行き来する間も revoke しない。
  // modal を閉じる / unmount で一括 revoke する。
  const previewUrls: string[] = []

  function makePreview(asset: ResolvedAsset): string {
    if (asset.kind === 'svg' && asset.svgText) {
      const url = URL.createObjectURL(new Blob([asset.svgText], { type: 'image/svg+xml' }))
      previewUrls.push(url)
      return url
    }
    const url = URL.createObjectURL(
      new Blob([new Uint8Array(asset.pngBytes)], { type: 'image/png' }),
    )
    previewUrls.push(url)
    return url
  }

  function resetState() {
    for (const u of previewUrls) URL.revokeObjectURL(u)
    previewUrls.length = 0
    matches.value = []
    unmatched.value = []
  }

  // modal が開いたタイミングで初期マッチを行う。閉じたら state を消して URL を revoke。
  watch(
    () => deps.open(),
    (open) => {
      if (!open) {
        resetState()
        return
      }
      resetState()

      const cursorpack = deps.cursorpack()
      const resolved = deps.resolved()
      const existingRoles = deps.existingRoles()

      if (cursorpack) {
        // .cursorpack 経路: ロール ID は既に確定済み。全ロールを matches に詰める。
        for (const [role, parsed] of Object.entries(cursorpack.roles)) {
          const isAni = parsed.ani !== null
          const fakeAsset: ResolvedAsset = {
            sourceFile: isAni ? `${role}.ani` : `${role}.cur`,
            // `.ani` の場合は展開先絶対パスを sourcePath に入れる
            // (export 時に Rust が rewrite_ani_with_hotspot のソースとして使う)
            sourcePath: parsed.aniSourcePath ?? '',
            kind: isAni ? 'ani' : 'cur',
            pngBytes: parsed.pngBytes,
            width: parsed.width,
            height: parsed.height,
            hotspot: parsed.hotspot,
            svgText: null,
            availableSizes: isAni ? [parsed.width] : Object.keys(parsed.sizedPngBytes).map(Number),
            ani: parsed.ani,
          }
          const conflict = existingRoles.has(role) ? 'overwrite-existing' : 'none'
          matches.value.push({
            role,
            asset: fakeAsset,
            conflict,
            previewUrl: makePreview(fakeAsset),
            aniFramesU8: toAniFramesU8(fakeAsset),
          })
        }
        return
      }

      // 通常経路: ファイル名 + フォルダ名のファジーマッチ → 衝突解決 → 既存衝突判定。
      // sourcePath を渡すことで `arrow/64.png` `通常/256.png` のように
      // フォルダー名にロール名が含まれるケースもマッチさせる。
      const candidates: Array<MatchCandidate & { asset: ResolvedAsset }> = []
      for (const a of resolved ?? []) {
        const m = matchAssetWithContext(a.sourceFile, a.sourcePath)
        if (m) {
          candidates.push({ sourceFile: a.sourceFile, nativeSize: a.width, match: m, asset: a })
        } else {
          unmatched.value.push({
            asset: a,
            previewUrl: makePreview(a),
            aniFramesU8: toAniFramesU8(a),
          })
        }
      }
      const { winners, demoted } = resolveCollisions(candidates)
      for (const c of demoted as Array<(typeof candidates)[0]>) {
        unmatched.value.push({
          asset: c.asset,
          previewUrl: makePreview(c.asset),
          aniFramesU8: toAniFramesU8(c.asset),
        })
      }
      for (const w of winners as Array<(typeof candidates)[0]>) {
        const conflict = existingRoles.has(w.match.role) ? 'overwrite-existing' : 'none'
        matches.value.push({
          role: w.match.role,
          asset: w.asset,
          conflict,
          previewUrl: makePreview(w.asset),
          aniFramesU8: toAniFramesU8(w.asset),
        })
      }
    },
    { immediate: true },
  )

  /**
   * 割当解除: matches から該当ロールのファイルを未マッチプールへ戻す。
   * previewUrl / aniFramesU8 はそのまま再利用する (revoke しない)。
   */
  function unassignRole(roleId: string): void {
    const idx = matches.value.findIndex((m) => m.role === roleId)
    if (idx < 0) return
    const m = matches.value[idx]!
    matches.value.splice(idx, 1)
    unmatched.value.push({
      asset: m.asset,
      previewUrl: m.previewUrl,
      aniFramesU8: m.aniFramesU8,
    })
  }

  /**
   * 未マッチプールからロールへ割当: 既存の割当があれば未マッチプールへ追い出してから入替。
   * UiSelect の v-model 変更ハンドラから呼ぶ。
   */
  function pickRoleFromUnmatched(item: UnmatchedFile, roleId: string): void {
    const itemIdx = unmatched.value.indexOf(item)
    if (itemIdx < 0) return

    // 既存割当があれば先に未マッチへ戻す (順序: 自分を抜く → 既存を戻す → 新規割当)
    unmatched.value.splice(itemIdx, 1)
    const existingIdx = matches.value.findIndex((m) => m.role === roleId)
    if (existingIdx >= 0) {
      const existing = matches.value[existingIdx]!
      matches.value.splice(existingIdx, 1)
      unmatched.value.push({
        asset: existing.asset,
        previewUrl: existing.previewUrl,
        aniFramesU8: existing.aniFramesU8,
      })
    }

    const conflict = deps.existingRoles().has(roleId) ? 'overwrite-existing' : 'none'
    matches.value.push({
      role: roleId,
      asset: item.asset,
      conflict,
      previewUrl: item.previewUrl,
      aniFramesU8: item.aniFramesU8,
    })
  }

  /**
   * 全解除: 現在割当済の全ファイルを未マッチプールへ戻す。
   * 「ファジーマッチの結果をまるごとリセットして最初から手動で割り直したい」用途。
   */
  function unassignAll(): void {
    for (const m of matches.value) {
      unmatched.value.push({
        asset: m.asset,
        previewUrl: m.previewUrl,
        aniFramesU8: m.aniFramesU8,
      })
    }
    matches.value = []
  }

  /**
   * matches 内の 1 件 (cursorpack 経路以外) を `RoleAsset` に変換する。
   *
   * 「元ファイルに hotspot 情報がない」を kind で判定する。
   * - PNG / SVG は仕様上 hotspot を持たない (Rust 側 bulk_resolve は (0,0) を返す)。
   * - CUR / ICO は header に hotspot があるが、未指定の場合 (0,0) で来る — そのときも
   *   中央既定が望ましい。`(4, 4)` で判定すると Rust の sentinel (= 0) と噛み合わず
   *   中央化が一切発火しないバグになる。
   * - .ani は自前の埋め込みホットスポットを使う (中央既定を適用しない)。
   */
  function toRoleAsset(
    roleId: string,
    asset: ResolvedAsset,
    source: RoleAsset['source'],
    isNewRole: boolean,
  ): RoleAsset {
    const isAni = asset.kind === 'ani' && asset.ani !== null
    const noEmbeddedHotspot =
      asset.kind === 'png' ||
      asset.kind === 'svg' ||
      (asset.hotspot.x === 0 && asset.hotspot.y === 0)
    const finalHotspot =
      isNewRole && noEmbeddedHotspot && !isAni
        ? initialHotspotFor(roleId, asset.width)
        : asset.hotspot
    const base: RoleAsset = {
      primary: new Uint8Array(asset.pngBytes),
      primarySize: asset.width,
      hotspot: finalHotspot,
      sized: undefined,
      source,
    }
    if (isAni && asset.ani) {
      base.aniSourcePath = asset.sourcePath
      base.aniFrames = {
        framePngs: asset.ani.framePngs.map((b) => new Uint8Array(b)),
        sequence: asset.ani.sequence,
        perStepDurationsMs: asset.ani.perStepDurationsMs,
      }
    }
    return base
  }

  /**
   * 現在の matches / metadataChoice / cursorpack metadata から ApplyPayload を組み立てる。
   *
   * - cursorpack 経路: parsed.sizedPngBytes / parsed.ani をそのまま RoleAsset に転記。
   * - 通常経路: ファイル数 > 1 で bulk-folder、それ以外 bulk-file をソースタグに採用。
   *   既存ロールでない場合のみ hotspot を中央化する (toRoleAsset で判定)。
   * - 未マッチに残ったファイルは破棄される (UI 側でも未マッチ件数として可視化済み)。
   */
  function buildApplyPayload(): ApplyPayload {
    const roleAssets: Array<{ roleId: string; asset: RoleAsset }> = []
    const cursorpack = deps.cursorpack()
    const resolved = deps.resolved()
    const existingRoles = deps.existingRoles()

    if (cursorpack) {
      for (const m of matches.value) {
        const parsed = cursorpack.roles[m.role]
        if (!parsed) continue
        // SizedAsset 形式で構築 (per-size hotspot は cursorpack 内では未サポートなので undefined)
        const sized = new Map<number, SizedAsset>()
        for (const [k, v] of Object.entries(parsed.sizedPngBytes)) {
          sized.set(Number(k), { png: new Uint8Array(v) })
        }
        const asset: RoleAsset = {
          primary: new Uint8Array(parsed.pngBytes),
          primarySize: parsed.width,
          hotspot: parsed.hotspot,
          sized: sized.size > 0 ? sized : undefined,
          source: 'cursorpack',
        }
        // `.ani` ロールはアニメーション情報を保持して export 時に動的カーソルとして再構築する
        if (parsed.ani && parsed.aniSourcePath) {
          asset.aniSourcePath = parsed.aniSourcePath
          asset.aniFrames = {
            framePngs: parsed.ani.framePngs.map((b) => new Uint8Array(b)),
            sequence: parsed.ani.sequence,
            perStepDurationsMs: parsed.ani.perStepDurationsMs,
          }
        }
        roleAssets.push({ roleId: m.role, asset })
      }
    } else {
      const sourceTag: RoleAsset['source'] =
        (resolved?.length ?? 0) > 1 ? 'bulk-folder' : 'bulk-file'
      for (const m of matches.value) {
        roleAssets.push({
          roleId: m.role,
          asset: toRoleAsset(m.role, m.asset, sourceTag, !existingRoles.has(m.role)),
        })
      }
    }

    return {
      roleAssets,
      metadataChoice: metadataChoice.value,
      metadata: cursorpack?.metadata ?? null,
    }
  }

  onUnmounted(resetState)

  return {
    matches,
    unmatched,
    metadataChoice,
    unassignRole,
    pickRoleFromUnmatched,
    unassignAll,
    buildApplyPayload,
  }
}
