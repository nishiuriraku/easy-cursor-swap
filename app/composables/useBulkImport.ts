/**
 * Rust 側 theme::types::AniFrameData に対応する TS 型 (Phase 3a 以降)。
 * フィールド名は flatten 後の JSON とそのまま一致。
 */
export interface AniAssetData {
  framePngs: number[][]
  sequence: number[]
  perStepDurationsMs: number[]
  isLegacyRawDib: boolean
}

/**
 * Rust 側 bulk_import::ResolvedAsset に対応 (Phase 3a 以降)。
 * `CursorAssetDescriptor` (`pngBytes` / `width` / `height` / `hotspot`) が
 * `#[serde(flatten)]` で top-level に展開された JSON 形に一致。
 */
export interface ResolvedAsset {
  sourceFile: string
  sourcePath: string
  kind: 'png' | 'svg' | 'cur' | 'ico' | 'ani'
  pngBytes: number[]
  width: number
  height: number
  hotspot: { x: number; y: number } // ratio
  svgText: string | null
  availableSizes: number[]
  ani: AniAssetData | null
}

export interface ResolveFailure {
  sourcePath: string
  reason: string
}

export interface BulkResolveResult {
  assets: ResolvedAsset[]
  failures: ResolveFailure[]
}

/**
 * Rust 側 bulk_import::ParsedRole に対応 (Phase 3a 以降)。
 * `CursorAssetDescriptor` が flatten された JSON 形に一致。
 * 旧 `primarySize` / `primaryPngBytes` は廃止 (`width` / `pngBytes` に統一)。
 */
export interface ParsedRole {
  pngBytes: number[]
  width: number
  height: number
  hotspot: { x: number; y: number } // ratio
  sizedPngBytes: Record<string, number[]>
  /** `.ani` ロールのフレームデータ。`.cur`/`.ico` ロールでは null。 */
  ani: AniAssetData | null
  /** `.ani` ロールの展開先絶対パス。`.cur`/`.ico` ロールでは null。
   *  export 時に Rust 側が `rewrite_ani_with_hotspot` のソースとして使う。 */
  aniSourcePath: string | null
}

export interface ParsedCursorpack {
  metadata: {
    /** theme.json の UUID。`?editPath` 経由でロードしたとき creator.vue が
     *  `sourceThemeId` にそのまま代入し、SaveDestinationModal の「上書き / 複製」
     *  セクションを表示するトリガにする。Rust 側で必ず Some を返すが、古い
     *  cursorpack で省略されている可能性を考慮して nullable のままにしておく。 */
    id: string | null
    nameJa: string | null
    nameEn: string | null
    author: string | null
    version: string | null
    description: string | null
  }
  roles: Record<string, ParsedRole>
}

/**
 * `BulkImportCancelledError` は canonical に `withProgressJob` で定義する。
 * ここで re-export することで、`useBulkImport` 経由でも
 * `import { BulkImportCancelledError } from '~/composables/withProgressJob'`
 * 経由でも `instanceof` が同じ identity を保つ (vitest auto-import TDZ 回避)。
 */
export { BulkImportCancelledError } from './withProgressJob'

import { createProgressJobRunner } from './withProgressJob'

/**
 * Creator の一括インポート機能を束ねる composable。
 * `withProgressJob` のヘルパーに listen/invoke/unlisten ライフサイクルを委譲し、
 * resolveAssets と parseCursorpack は薄いラッパになる。
 *
 * `bulk_resolve_assets` はユーザーがキャンセルすると AppError::BulkImportCancelled
 * で reject するため、resolveAssets の typed 翻訳はヘルパー側に集約。
 *
 * resolveAssets と parseCursorpack は同じ runner インスタンスの state (busy /
 * progress / currentJobId / cancelledJobId / cancel) を共有する。両者を同時には
 * 走らせない前提 (UI 上のアクションが排他) なので、`buildInvoke` クロージャが
 * 呼び出しごとに command と args を組み立てて単一 runner を再利用する。
 */
export function useBulkImport() {
  // resolveAssets / parseCursorpack から書き換える「次に走らせるスペック」。
  // buildInvoke クロージャがこの変数を参照して invoke スペックを返す。
  let pendingCommand: string = 'bulk_resolve_assets'
  let pendingArgs: Record<string, unknown> = { req: { paths: [], recursive: false, jobId: '' } }

  const runner = createProgressJobRunner({
    jobIdPrefix: 'bulk',
    buildInvoke: (jobId) => ({
      command: pendingCommand,
      // pendingArgs 内の jobId を最新 jobId に差し替えて返す。
      args: mergeJobId(pendingArgs, jobId),
    }),
    unwrap: (raw) => raw,
  })

  async function resolveAssets(
    paths: string[],
    recursive: boolean,
  ): Promise<BulkResolveResult> {
    pendingCommand = 'bulk_resolve_assets'
    pendingArgs = { req: { paths, recursive } }
    const r = (await runner.handle()) as BulkResolveResult | null
    return r ?? { assets: [], failures: [] }
  }

  async function parseCursorpack(path: string): Promise<ParsedCursorpack> {
    pendingCommand = 'parse_cursorpack_for_creator'
    pendingArgs = { req: { path } }
    const r = await runner.handle()
    if (!r) throw new Error('cursorpack parse returned empty')
    return r as ParsedCursorpack
  }

  return {
    busy: runner.busy,
    progress: runner.progress,
    currentJobId: runner.currentJobId,
    cancelledJobId: runner.cancelledJobId,
    resolveAssets,
    parseCursorpack,
    cancel: runner.cancel,
  }
}

/**
 * args ツリーの `req.jobId` 位置にヘルパー生成の jobId を埋め込む。
 * Rust 側 IPC は `req` フィールド内に `{ path, jobId }` / `{ paths, recursive, jobId }`
 * を持つので、そこを上書きする。
 */
function mergeJobId(args: Record<string, unknown>, jobId: string): Record<string, unknown> {
  const req = (args.req as Record<string, unknown> | undefined) ?? {}
  return {
    ...args,
    req: { ...req, jobId },
  }
}