/**
 * テーマカード/ApplyModal で「実物の絵」を表示するためのプレビューキャッシュ。
 *
 * Rust の `get_theme_role_previews` IPC で取得した PNG バイト列を Blob URL に変換し、
 * テーマ ID 単位でキャッシュする。同じテーマに対する重複リクエストは in-flight Promise を共有する。
 *
 * Demo / Marketplace のリモートテーマには使用しない (UUID 形式の ID のみ対応)。
 *
 * Map + inflight + invalidate のコア機構は `usePngBlobCache` に統一済み。
 */
import { ref } from 'vue'
import { invokeTauri } from './useTauri'
import { usePngBlobCache } from './usePngBlobCache'

/**
 * 個別ロールのプレビュー詳細。
 *
 * - `url`: PNG を Blob 化した ObjectURL (`<img>` の src に使う)
 * - `hotspot`: ratio 0.0-1.0 のホットスポット座標
 * - `width/height`: PNG のネイティブ寸法 (px)
 * - `aniFrames`: 元ファイルが `.ani` のときのみセット。詳細ドロワーで
 *   `<CursorPreview kind="ani">` に渡してアニメーション再生する。
 *
 * `width/height` が 0 のときはホットスポット位置を計算できないので、
 * フロント側はフォールバック (中央表示) する。
 */
export interface RolePreviewDetail {
  url: string
  hotspot: { x: number; y: number } // ratio 0.0-1.0
  width: number
  height: number
  aniFrames?: {
    framePngs: Uint8Array[]
    sequence: number[]
    durations: number[]
    nativeSize: number
  }
}

interface PreviewCacheEntry {
  /** role → blob URL (ObjectURL) のマップ。後方互換のため維持。 */
  urls: Record<string, string>
  /** role → 寸法 + ホットスポット詳細 */
  details: Record<string, RolePreviewDetail>
}

interface IpcAniFrameData {
  framePngs: number[][]
  sequence: number[]
  perStepDurationsMs: number[]
  isLegacyRawDib: boolean
}

interface IpcRolePreview {
  pngBytes: number[]
  width: number
  height: number
  hotspot: { x: number; y: number }
  /** `.ani` ロールのときのみ存在。Rust の `RolePreview.frames` (Option) を反映。 */
  frames?: IpcAniFrameData
}

/** UUID v4 風 (xxxxxxxx-xxxx-...) のテーマ ID か簡易判定。 */
function looksLikeUuid(id: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id)
}

const WINDOWS_PREFIX = 'windows:'

/**
 * IPC 結果 (role → IpcRolePreview の Record) をフロント表現
 * (`urls` + `details`) に正規化する純粋関数。
 */
function normalize(result: Record<string, IpcRolePreview>): PreviewCacheEntry {
  const urls: Record<string, string> = {}
  const details: Record<string, RolePreviewDetail> = {}
  for (const [role, info] of Object.entries(result)) {
    const u8 = new Uint8Array(info.pngBytes)
    const blob = new Blob([u8], { type: 'image/png' })
    const url = URL.createObjectURL(blob)
    urls[role] = url
    const aniFrames = info.frames
      ? {
          framePngs: info.frames.framePngs.map((arr) => new Uint8Array(arr)),
          sequence: info.frames.sequence,
          durations: info.frames.perStepDurationsMs,
          // ANI フレームの内在ピクセル幅。ライブラリのプレビューでは正方画像前提なので
          // `width` のみ採用。
          nativeSize: info.width,
        }
      : undefined
    details[role] = {
      url,
      hotspot: info.hotspot,
      width: info.width,
      height: info.height,
      ...(aniFrames ? { aniFrames } : {}),
    }
  }
  return { urls, details }
}

/**
 * モジュールスコープの singleton キャッシュ。
 *
 * - UUID 形式 → ローカルテーマ (`get_theme_role_previews`)
 * - `windows:<scheme name>` → Windows レジストリスキーム (`get_windows_scheme_role_previews`)
 */
const previewCache = usePngBlobCache<string, PreviewCacheEntry>({
  async fetcher(themeId) {
    const isWindowsScheme = themeId.startsWith(WINDOWS_PREFIX)
    if (!isWindowsScheme && !looksLikeUuid(themeId)) return null
    try {
      // ホットスポット情報込みの新 IPC を優先利用。
      // 単一の往復で url + 寸法 + hotspot を確定させ、テーマ詳細ドロワーの
      // ホットスポット可視化と従来サムネ用 url の両方を 1 回でキャッシュする。
      const result = isWindowsScheme
        ? await invokeTauri<Record<string, IpcRolePreview>>('get_windows_scheme_role_previews', {
            name: themeId.slice(WINDOWS_PREFIX.length),
          })
        : await invokeTauri<Record<string, IpcRolePreview>>('get_theme_role_previews', {
            themeId,
            roles: [],
          })
      if (!result) return null
      return normalize(result)
    } catch (err) {
      console.warn('[useThemePreviews] preview fetch failed:', err)
      return null
    }
  },
  dispose(entry) {
    for (const url of Object.values(entry.urls)) URL.revokeObjectURL(url)
  },
})

export function useThemePreviews() {
  const lastError = ref<string | null>(null)

  async function getMap(themeId: string): Promise<Record<string, string> | null> {
    const entry = await previewCache.get(themeId)
    return entry?.urls ?? null
  }

  /**
   * ホットスポット情報込みのプレビュー詳細を返す。
   * テーマ詳細ドロワーで `<img>` の上に hot ドットを正しい位置に置くために使う。
   */
  async function getDetails(themeId: string): Promise<Record<string, RolePreviewDetail> | null> {
    const entry = await previewCache.get(themeId)
    return entry?.details ?? null
  }

  return {
    getMap,
    getDetails,
    invalidate: (themeId: string) => previewCache.invalidate(themeId),
    invalidateAll: () => previewCache.invalidateAll(),
    lastError,
  }
}
