/**
 * Map ベースのキャッシュ + in-flight Promise 共有 + invalidate/dispose 機構を
 * 抽象化した汎用 composable。
 *
 * 経緯: `useThemePreviews` / `useMarketplacePreviews` / Creator の
 * roleBlobCache / sizeBlobCache などで「fetcher の戻り値 (主に PNG → Blob URL)
 * をキー単位でメモ化し、同時要求は in-flight Promise を共有し、削除時に
 * URL.revokeObjectURL を確実に呼ぶ」というほぼ同型のパターンが 3 箇所に
 * 散在していた。本ファイルでそれを共通化することで:
 *  - inflight 解放漏れ / dispose 漏れ等の subtle なバグの一元化
 *  - 各 composable の責務を「fetcher の組み立て」だけに矮小化
 *
 * 命名は audit ドキュメント (`usePngBlobCache<K>`) を踏襲しているが、実装は
 * 値型を generic にしてあり Blob URL 以外 (例: 寸法やホットスポット入りの
 * 複合オブジェクト) も格納できる。
 */

export interface PngBlobCacheOptions<K, V> {
  /**
   * キーから値を取得する非同期 fetcher。
   * `null` を返した場合はキャッシュに入れず、`inflight` だけを解放する
   * (取得失敗を毎回再試行可能にする)。
   */
  fetcher: (key: K) => Promise<V | null>
  /** Map キー化用の文字列変換。省略時は `String(key)`。 */
  keyOf?: (key: K) => string
  /**
   * `invalidate` / `invalidateAll` 時に呼ばれる cleanup フック。
   * 主に `URL.revokeObjectURL` を Blob URL に対して呼ぶ用途を想定。
   */
  dispose?: (value: V) => void
}

export interface PngBlobCache<K, V> {
  get: (key: K) => Promise<V | null>
  invalidate: (key: K) => void
  invalidateAll: () => void
  /** デバッグ用: 現在キャッシュされているキー数 */
  size: () => number
}

/**
 * fetcher + dispose を渡してキャッシュインスタンスを得る。
 *
 * **ライフタイム**: 戻り値のキャッシュはモジュールスコープではなく
 * 呼び出し側で保持する必要がある。singleton にしたい場合は
 * `const cache = usePngBlobCache({...})` を composable のモジュール
 * トップレベルで宣言すること (本リポジトリの `useThemePreviews` 等が該当)。
 */
export function usePngBlobCache<K, V>(opts: PngBlobCacheOptions<K, V>): PngBlobCache<K, V> {
  const keyOf = opts.keyOf ?? ((k: K) => String(k))
  const cache = new Map<string, V>()
  const inflight = new Map<string, Promise<V | null>>()

  async function get(key: K): Promise<V | null> {
    const k = keyOf(key)
    const cached = cache.get(k)
    if (cached !== undefined) return cached
    const pending = inflight.get(k)
    if (pending) return pending

    const promise = (async () => {
      try {
        const v = await opts.fetcher(key)
        if (v !== null) cache.set(k, v)
        return v
      } finally {
        // 成功 / 失敗 / null 戻り 問わず inflight は必ず解放する。
        // 失敗ケースで inflight に Promise が残り続けると次回以降
        // 永遠に rejected Promise を返してしまう。
        inflight.delete(k)
      }
    })()
    inflight.set(k, promise)
    return promise
  }

  function invalidate(key: K): void {
    const k = keyOf(key)
    const v = cache.get(k)
    if (v !== undefined && opts.dispose) opts.dispose(v)
    cache.delete(k)
    inflight.delete(k)
  }

  function invalidateAll(): void {
    if (opts.dispose) {
      for (const v of cache.values()) opts.dispose(v)
    }
    cache.clear()
    inflight.clear()
  }

  return { get, invalidate, invalidateAll, size: () => cache.size }
}
