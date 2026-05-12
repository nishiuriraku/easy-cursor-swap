/**
 * テーマカード/ApplyModal で「実物の絵」を表示するためのプレビューキャッシュ。
 *
 * Rust の `get_theme_previews` IPC で取得した PNG バイト列を Blob URL に変換し、
 * テーマ ID 単位でキャッシュする。同じテーマに対する重複リクエストは in-flight Promise を共有する。
 *
 * Demo / Marketplace のリモートテーマには使用しない (UUID 形式の ID のみ対応)。
 */
import { ref } from 'vue'
import { invokeTauri } from './useTauri'

/**
 * 個別ロールのプレビュー詳細。
 *
 * - `url`: PNG を Blob 化した ObjectURL (`<img>` の src に使う)
 * - `hotspot`: ratio 0.0-1.0 のホットスポット座標
 * - `width/height`: PNG のネイティブ寸法 (px)
 *
 * `width/height` が 0 のときはホットスポット位置を計算できないので、
 * フロント側はフォールバック (中央表示) する。
 */
export interface RolePreviewDetail {
  url: string
  hotspot: { x: number; y: number } // ratio 0.0-1.0
  width: number
  height: number
}

interface PreviewCacheEntry {
  /** role → blob URL (ObjectURL) のマップ。後方互換のため維持。 */
  urls: Record<string, string>
  /** role → 寸法 + ホットスポット詳細 */
  details: Record<string, RolePreviewDetail>
}

interface IpcRolePreview {
  png: number[]
  width: number
  height: number
  hotspot: { x: number; y: number }
}

const cache = new Map<string, PreviewCacheEntry>()
const inflight = new Map<string, Promise<PreviewCacheEntry | null>>()

/** UUID v4 風 (xxxxxxxx-xxxx-...) のテーマ ID か簡易判定。 */
function looksLikeUuid(id: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id)
}

const WINDOWS_PREFIX = 'windows:'

/**
 * 指定テーマの全ロール (または指定ロール) のプレビューを取得する。
 *
 * 結果は `cache` に保存され、`role → blob URL` のマップを返す。失敗時は null。
 *
 * - UUID 形式 → ローカルテーマ (`get_theme_previews`)
 * - `windows:<scheme name>` → Windows レジストリスキーム (`get_windows_scheme_previews`)
 */
async function fetchPreviews(themeId: string, roles?: string[]): Promise<PreviewCacheEntry | null> {
  const isWindowsScheme = themeId.startsWith(WINDOWS_PREFIX)
  if (!isWindowsScheme && !looksLikeUuid(themeId)) return null
  // 全ロール取得済みならキャッシュをそのまま返す。部分リクエストはキャッシュに合流させる。
  const existing = cache.get(themeId)
  if (existing) return existing

  const key = themeId
  const pending = inflight.get(key)
  if (pending) return pending

  const promise = (async () => {
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
            roles: roles ?? [],
          })
      if (!result) return null
      const urls: Record<string, string> = {}
      const details: Record<string, RolePreviewDetail> = {}
      for (const [role, info] of Object.entries(result)) {
        const u8 = new Uint8Array(info.png)
        const blob = new Blob([u8], { type: 'image/png' })
        const url = URL.createObjectURL(blob)
        urls[role] = url
        details[role] = {
          url,
          hotspot: info.hotspot,
          width: info.width,
          height: info.height,
        }
      }
      const entry: PreviewCacheEntry = { urls, details }
      cache.set(themeId, entry)
      return entry
    } catch (err) {
      console.warn('[useThemePreviews] preview fetch failed:', err)
      return null
    } finally {
      inflight.delete(key)
    }
  })()
  inflight.set(key, promise)
  return promise
}

/** キャッシュを破棄して Blob URL を revoke する (テーマ削除/再インポート時に呼ぶ)。 */
function invalidate(themeId: string) {
  const entry = cache.get(themeId)
  if (entry) {
    for (const url of Object.values(entry.urls)) URL.revokeObjectURL(url)
    cache.delete(themeId)
  }
  inflight.delete(themeId)
}

/** 全エントリを破棄する (アプリ終了/言語切替などで使用)。 */
function invalidateAll() {
  for (const entry of cache.values()) {
    for (const url of Object.values(entry.urls)) URL.revokeObjectURL(url)
  }
  cache.clear()
  inflight.clear()
}

export function useThemePreviews() {
  // 個別リアクティブな `previews` 参照が必要な利用側のために、ヘルパーも返す。
  const lastError = ref<string | null>(null)

  async function getMap(themeId: string, roles?: string[]): Promise<Record<string, string> | null> {
    const entry = await fetchPreviews(themeId, roles)
    return entry?.urls ?? null
  }

  /**
   * ホットスポット情報込みのプレビュー詳細を返す。
   * テーマ詳細ドロワーで `<img>` の上に hot ドットを正しい位置に置くために使う。
   */
  async function getDetails(
    themeId: string,
    roles?: string[],
  ): Promise<Record<string, RolePreviewDetail> | null> {
    const entry = await fetchPreviews(themeId, roles)
    return entry?.details ?? null
  }

  return {
    getMap,
    getDetails,
    invalidate,
    invalidateAll,
    lastError,
  }
}
