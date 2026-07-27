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
 * `BulkImportCancelledError` は canonical に `withProgressJob` で定義されており、
 * すべての呼び出し側は `~/composables/withProgressJob` から直接 import する
 * (instanceof identity を保つため)。
 */
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
 * 走らせない前提 (UI 上のアクションが排他) だが、`runner.handle()` 内部の
 * `await listenProgress(jobId)` で制御が一旦 yield するため、共有 mutable closure
 * を buildInvoke に読ませると race になる (Wave 2AB Task 7 I-1 parked)。
 * 代わりに per-call に `(jobId) => InvokeSpec` を引数で渡すことで、呼び出し時点の
 * ローカル変数を同期キャプチャして race を排除する。
 */
export function useBulkImport() {
  // `options.buildInvoke` は runner 固定のフォールバック。per-call では
  // `runner.handle((jobId) => ...)` 形式で毎回ローカル closure を渡す。
  // ここには到達しないはずだが、型シグネチャの充足と fail-fast のために残す。
  const runner = createProgressJobRunner({
    jobIdPrefix: 'bulk',
    buildInvoke: () => {
      throw new Error(
        'useBulkImport: per-call buildInvoke が必要です。runner.handle(buildInvoke) 形式で呼んでください',
      )
    },
    unwrap: (raw) => raw,
  })

  async function resolveAssets(paths: string[], recursive: boolean): Promise<BulkResolveResult> {
    const command = 'bulk_resolve_assets'
    const argsBase: Record<string, unknown> = { req: { paths, recursive } }
    const r = (await runner.handle((jobId) => ({
      command,
      args: mergeJobId(argsBase, jobId),
    }))) as BulkResolveResult | null
    return r ?? { assets: [], failures: [] }
  }

  async function parseCursorpack(path: string): Promise<ParsedCursorpack> {
    const command = 'parse_cursorpack_for_creator'
    const argsBase: Record<string, unknown> = { req: { path } }
    const r = await runner.handle((jobId) => ({
      command,
      args: mergeJobId(argsBase, jobId),
    }))
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
