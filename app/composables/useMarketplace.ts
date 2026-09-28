/**
 * 公式マーケットプレース index の singleton state + IPC 集約。
 *
 * 旧設計では `pages/marketplace.vue` と `layouts/default.vue` (sidebar バッジ用) が
 * それぞれ `marketplace_fetch_index` を直接 `invokeTauri` し、`marketplace_install` も
 * page から直接叩いていた (audit E 由来、Wave 2B の `useThemes` 集約と並んで page を
 * slim 化するため)。本 composable に集約することで:
 *   - HTTP 取得を 1 度だけ走らせ、layout / page / 他コンポーネントが同じ entries を参照
 *   - `verified` フラグ付与 (Rust 側 IPC payload には含まれない、CI 検証済の含意)
 *   - install の成否は composable では throw し、caller の toast UI と責務分離
 *
 * P11 (オフライン耐性): IPC は `MarketplaceIndexResult { index, stale, fetched_at, error }`
 * を返す。ネットワーク失敗時に Rust がディスクキャッシュ (`~/.custom_cursors/_
 * marketplace_index_cache.json`) を `stale: true` で返してきた場合も `entries` に
 * 入れて一覧表示し、`stale` / `fetchedAt` でバナー表示する。`navigator.onLine` と
 * `online` / `offline` イベントは表示補助 (`offlineHint`) と復帰時の自動再取得にのみ
 * 使い、stale 判定の正は Rust の取得結果とする (WebView2 の `navigator.onLine` は
 * NIC 状態しか見ないため)。
 *
 * 単一画面を超えて参照される singleton 状態なので Pinia 不使用のシンプル
 * composable で実装 (useThemes と同じパターン)。
 */
import type { Ref } from 'vue'
import type { MarketplaceEntry, MarketplaceIndexResult } from '~/types/marketplace'

// Rust 側 IPC payload の entries は `verified` フィールドを含まない (掲載 = CI 検証済 の
// フロント側での固定含意)。entries はそのままの生形で保持せず、`withVerified`
// で薄い wrapper を被せて `MarketplaceEntry` 形に正規化する。
function withVerified(e: Omit<MarketplaceEntry, 'verified'>): MarketplaceEntry {
  return { ...e, verified: true }
}

// ─────────────────────────────────────────────────────────────
// Singleton state (module スコープ)。
// `loadIndex()` を layout と page から呼んでも dedupe されるよう `inflight` で
// 並行呼び出しをガード。useThemes の refresh() と同じパターン。
// ─────────────────────────────────────────────────────────────

const entries = ref<MarketplaceEntry[]>([])
const isLoading = ref(false)
const fetchError = ref<string | null>(null)
/** true のとき entries はディスクキャッシュ由来 (前回成功時の stale 表示)。 */
const stale = ref(false)
/** 最終取得成功時刻 (Rust の RFC3339 文字列。stale 時はキャッシュ取得時刻)。 */
const fetchedAt = ref<string | null>(null)
/** `navigator.onLine` の追随値。表示補助と復帰時再取得トリガ専用。 */
const online = ref(typeof navigator === 'undefined' ? true : navigator.onLine)
let inflight: Promise<void> | null = null

let networkListenersInstalled = false
function installNetworkListeners() {
  if (networkListenersInstalled || typeof window === 'undefined') return
  networkListenersInstalled = true
  window.addEventListener('online', () => {
    online.value = true
    void loadIndex()
  })
  window.addEventListener('offline', () => {
    online.value = false
  })
}

async function loadIndex(): Promise<void> {
  if (inflight) return inflight
  isLoading.value = true
  fetchError.value = null
  inflight = (async () => {
    try {
      const res = await invokeTauri<MarketplaceIndexResult>('marketplace_fetch_index')
      if (!res || !res.index) {
        throw new Error('empty response')
      }
      entries.value = res.index.entries.map(withVerified)
      stale.value = res.stale
      fetchedAt.value = res.fetchedAt
      // stale (キャッシュ表示) でも entries は入れる。fetchError には失敗理由を残し、
      // page 側は entries がある場合は一覧 + stale バナー、空の場合のみエラー画面にする。
      fetchError.value = res.stale ? res.error : null
    } catch (e) {
      entries.value = []
      stale.value = false
      fetchError.value = appErrorMessage(e)
      // eslint-disable-next-line no-console
      console.warn('[useMarketplace] fetch_index failed:', e)
    } finally {
      isLoading.value = false
      inflight = null
    }
  })()
  return inflight
}

/**
 * `installEntry` は caller の toast UI と責務分離するため、失敗時は throw する。
 * caller は通常:
 *   ```ts
 *   try {
 *     await marketplace.installEntry(id)
 *     installStatus.value = { kind: 'ok', name: displayName }
 *   } catch (err) {
 *     installStatus.value = { kind: 'err', name, message: ... }
 *   }
 *   ```
 * のパターンで利用する。
 */
async function installEntry(id: string): Promise<void> {
  const e = entries.value.find((x) => x.id === id)
  if (!e) {
    throw new Error(`Marketplace entry ${id} not found in cache`)
  }
  await invokeTauri<string>('marketplace_install', {
    req: {
      downloadUrl: e.downloadUrl,
      sha256: e.sha256,
      signature: e.signature,
      authorGithub: e.authorGithub,
      authorPubkeyId: e.authorPubkeyId,
    },
  })
}

/**
 * テスト専用: モジュールスコープの singleton state を初期値に戻す。
 * 通常アプリコードから呼び出してはならない (useAppSettings と同じパターン)。
 * window の online/offline リスナーはアプリ存続期間の購読なので張り直さない。
 */
function __resetForTests() {
  entries.value = []
  isLoading.value = false
  fetchError.value = null
  stale.value = false
  fetchedAt.value = null
  online.value = typeof navigator === 'undefined' ? true : navigator.onLine
  inflight = null
}

export function useMarketplace(): {
  entries: Readonly<Ref<MarketplaceEntry[]>>
  isLoading: Readonly<Ref<boolean>>
  fetchError: Readonly<Ref<string | null>>
  stale: Readonly<Ref<boolean>>
  fetchedAt: Readonly<Ref<string | null>>
  online: Readonly<Ref<boolean>>
  loadIndex: () => Promise<void>
  installEntry: (id: string) => Promise<void>
  __resetForTests: () => void
} {
  installNetworkListeners()
  return {
    entries,
    isLoading,
    fetchError,
    stale,
    fetchedAt,
    online,
    loadIndex,
    installEntry,
    __resetForTests,
  }
}
