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

interface PreviewCacheEntry {
  /** role → blob URL (ObjectURL) のマップ */
  urls: Record<string, string>
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
async function fetchPreviews(
  themeId: string,
  roles?: string[],
): Promise<PreviewCacheEntry | null> {
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
      const result = isWindowsScheme
        ? await invokeTauri<Record<string, number[]>>('get_windows_scheme_previews', {
            name: themeId.slice(WINDOWS_PREFIX.length),
          })
        : await invokeTauri<Record<string, number[]>>('get_theme_previews', {
            themeId,
            roles: roles ?? [],
          })
      if (!result) return null
      const urls: Record<string, string> = {}
      for (const [role, bytes] of Object.entries(result)) {
        const u8 = new Uint8Array(bytes)
        const blob = new Blob([u8], { type: 'image/png' })
        urls[role] = URL.createObjectURL(blob)
      }
      const entry: PreviewCacheEntry = { urls }
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

  return {
    getMap,
    invalidate,
    invalidateAll,
    lastError,
  }
}
