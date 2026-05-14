/**
 * 公式インデックスのカーソル絵柄プレビューを取得 / キャッシュする singleton composable。
 *
 * MarketplaceDetailModal で 6 ロール分の PNG (Arrow / Help / AppStarting / Wait / Crosshair / IBeam)
 * を `marketplace_fetch_preview` IPC 経由で並列取得し、Blob URL にして Map にキャッシュする。
 * 同一エントリへの再要求は同じ in-flight Promise を共有する (useThemePreviews と同じパターン)。
 */
import { ref } from 'vue'
import { invokeTauri } from './useTauri'

/** 公式インデックスと約束された先頭 6 ロール (CURSOR_ROLES 正規順)。 */
const PREVIEW_ROLES = ['Arrow', 'Help', 'AppStarting', 'Wait', 'Crosshair', 'IBeam'] as const

const cache = new Map<string, Record<string, string>>()
const inflight = new Map<string, Promise<Record<string, string>>>()

async function fetchOne(previewBaseUrl: string, role: string): Promise<Uint8Array | null> {
  try {
    const bytes = await invokeTauri<number[] | ArrayBuffer>('marketplace_fetch_preview', {
      previewBaseUrl,
      role,
    })
    if (!bytes) return null
    // Tauri 2 の serde は Vec<u8> を ArrayBuffer または number[] で渡してくる。両対応。
    if (bytes instanceof ArrayBuffer) return new Uint8Array(bytes)
    return new Uint8Array(bytes as number[])
  } catch (e) {
    console.warn('[useMarketplacePreviews] failed', role, e)
    return null
  }
}

async function fetchPreviews(
  entryId: string,
  previewBaseUrl: string,
): Promise<Record<string, string>> {
  const cached = cache.get(entryId)
  if (cached) return cached
  const pending = inflight.get(entryId)
  if (pending) return pending

  const promise = (async () => {
    const settled = await Promise.all(PREVIEW_ROLES.map((r) => fetchOne(previewBaseUrl, r)))
    const urls: Record<string, string> = {}
    settled.forEach((bytes, i) => {
      if (bytes) {
        const blob = new Blob([bytes], { type: 'image/png' })
        urls[PREVIEW_ROLES[i]] = URL.createObjectURL(blob)
      }
    })
    cache.set(entryId, urls)
    return urls
  })()
  inflight.set(entryId, promise)
  try {
    return await promise
  } finally {
    inflight.delete(entryId)
  }
}

function invalidate(entryId: string) {
  const urls = cache.get(entryId)
  if (urls) {
    for (const u of Object.values(urls)) URL.revokeObjectURL(u)
    cache.delete(entryId)
  }
  inflight.delete(entryId)
}

function invalidateAll() {
  for (const urls of cache.values()) {
    for (const u of Object.values(urls)) URL.revokeObjectURL(u)
  }
  cache.clear()
  inflight.clear()
}

export function useMarketplacePreviews() {
  const lastError = ref<string | null>(null)
  async function getMap(entryId: string, previewBaseUrl: string): Promise<Record<string, string>> {
    if (!previewBaseUrl) return {}
    lastError.value = null
    return fetchPreviews(entryId, previewBaseUrl)
  }
  return { getMap, invalidate, invalidateAll, lastError }
}
