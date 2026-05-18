/**
 * 公式インデックスのカーソル絵柄プレビューを取得 / キャッシュする singleton composable。
 *
 * MarketplaceDetailModal で 6 ロール分の PNG (Arrow / Help / AppStarting / Wait / Crosshair / IBeam)
 * を `marketplace_fetch_preview` IPC 経由で並列取得し、Blob URL にして Map にキャッシュする。
 * 同一エントリへの再要求は同じ in-flight Promise を共有する。
 *
 * Map + inflight + invalidate のコア機構は `usePngBlobCache` に統一済み。本ファイルでは
 * Marketplace 固有の fetcher (6 ロール並列取得 + Uint8Array 正規化) と dispose
 * (Blob URL revoke) のみを定義する。
 */

/** 公式インデックスと約束された先頭 6 ロール (CURSOR_ROLES 正規順)。 */
const PREVIEW_ROLES = ['Arrow', 'Help', 'AppStarting', 'Wait', 'Crosshair', 'IBeam'] as const

/** マーケットプレース取得時の単一ロール raw bytes フェッチ。 */
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

/**
 * モジュールスコープの singleton キャッシュ。
 * キーは `${entryId}::${previewBaseUrl}` の合成だが、entryId 1 つに対し base URL は 1 つなので
 * 実質 entryId をキーにしている。
 */
const previewCache = usePngBlobCache<
  { entryId: string; previewBaseUrl: string },
  Record<string, string>
>({
  keyOf: (k) => k.entryId,
  async fetcher({ previewBaseUrl }) {
    const settled = await Promise.all(PREVIEW_ROLES.map((r) => fetchOne(previewBaseUrl, r)))
    const urls: Record<string, string> = {}
    settled.forEach((bytes, i) => {
      if (bytes) {
        const blob = new Blob([bytes], { type: 'image/png' })
        urls[PREVIEW_ROLES[i]] = URL.createObjectURL(blob)
      }
    })
    return urls
  },
  dispose(urls) {
    for (const u of Object.values(urls)) URL.revokeObjectURL(u)
  },
})

export function useMarketplacePreviews() {
  const lastError = ref<string | null>(null)
  async function getMap(entryId: string, previewBaseUrl: string): Promise<Record<string, string>> {
    if (!previewBaseUrl) return {}
    lastError.value = null
    const urls = await previewCache.get({ entryId, previewBaseUrl })
    return urls ?? {}
  }
  return {
    getMap,
    invalidate: (entryId: string) => previewCache.invalidate({ entryId, previewBaseUrl: '' }),
    invalidateAll: () => previewCache.invalidateAll(),
    lastError,
  }
}
