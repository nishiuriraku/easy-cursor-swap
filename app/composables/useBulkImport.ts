import { ref } from 'vue'
import { invokeTauri } from '~/composables/useTauri'

export interface AniAssetData {
  framePngs: number[][]
  sequence: number[]
  perStepDurationsMs: number[]
  isLegacyRawDib: boolean
}

export interface ResolvedAsset {
  sourceFile: string
  sourcePath: string
  kind: 'png' | 'svg' | 'cur' | 'ico' | 'ani'
  pngBytes: number[]
  svgText: string | null
  nativeSize: number
  hotspot: { x: number; y: number } // ratio
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

export interface ParsedRole {
  primarySize: number
  primaryPngBytes: number[]
  hotspot: { x: number; y: number } // ratio
  sizedPngBytes: Record<string, number[]> // 当面この形は維持 (Task 5 で見直し可能)
  /** `.ani` ロールのフレームデータ。`.cur`/`.ico` ロールでは null。 */
  ani: AniAssetData | null
  /** `.ani` ロールの展開先絶対パス。`.cur`/`.ico` ロールでは null。
   *  export 時に Rust 側が `rewrite_ani_with_hotspot` のソースとして使う。 */
  aniSourcePath: string | null
}

export interface ParsedCursorpack {
  metadata: {
    nameJa: string | null
    nameEn: string | null
    author: string | null
    version: string | null
    description: string | null
  }
  roles: Record<string, ParsedRole>
}

interface BulkImportProgress {
  jobId: string
  stage: 'scan' | 'parse' | 'extract' | 'done' | 'error'
  current: number
  total: number
  message: string | null
}

export function useBulkImport() {
  const busy = ref(false)
  const progress = ref<BulkImportProgress | null>(null)
  const currentJobId = ref<string | null>(null)

  function newJobId(prefix: string) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  }

  async function subscribeProgress(jobId: string): Promise<() => void> {
    try {
      const { listen } = await import('@tauri-apps/api/event')
      const un = await listen<BulkImportProgress>('bulk-import-progress', (e) => {
        if (e.payload.jobId === jobId) progress.value = e.payload
      })
      return un
    } catch {
      return () => {}
    }
  }

  async function resolveAssets(paths: string[], recursive: boolean): Promise<BulkResolveResult> {
    busy.value = true
    progress.value = null
    const jobId = newJobId('bulk')
    currentJobId.value = jobId
    const unlisten = await subscribeProgress(jobId)
    try {
      const r = await invokeTauri<BulkResolveResult>('bulk_resolve_assets', {
        req: { paths, recursive, jobId },
      })
      return r ?? { assets: [], failures: [] }
    } finally {
      unlisten()
      currentJobId.value = null
      busy.value = false
    }
  }

  async function parseCursorpack(path: string): Promise<ParsedCursorpack> {
    busy.value = true
    progress.value = null
    const jobId = newJobId('cpack')
    currentJobId.value = jobId
    const unlisten = await subscribeProgress(jobId)
    try {
      const r = await invokeTauri<ParsedCursorpack>('parse_cursorpack_for_creator', {
        req: { path, jobId },
      })
      if (!r) throw new Error('cursorpack parse returned empty')
      return r
    } finally {
      unlisten()
      currentJobId.value = null
      busy.value = false
    }
  }

  async function cancel() {
    if (!currentJobId.value) return
    try {
      await invokeTauri('cancel_bulk_import', { jobId: currentJobId.value })
    } catch {
      // ignore
    }
  }

  return { busy, progress, resolveAssets, parseCursorpack, cancel }
}
