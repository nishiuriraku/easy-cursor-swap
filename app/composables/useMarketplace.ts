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
 * 単一画面を超えて参照される singleton 状態なので Pinia 不使用のシンプル
 * composable で実装 (useThemes と同じパターン)。
 */
import type { Ref } from 'vue'
import type { MarketplaceEntry } from '~/types/marketplace'

// Rust 側 IPC payload は `verified` フィールドを含まない (掲載 = CI 検証済 の
// フロント側での固定含意)。entries はそのままの生形で保持せず、`withVerified`
// で薄い wrapper を被せて `MarketplaceEntry` 形に正規化する。
interface RustMarketplaceIndex {
  schema_version: number
  commit?: string
  entries: Omit<MarketplaceEntry, 'verified'>[]
}

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
let inflight: Promise<void> | null = null

async function loadIndex(): Promise<void> {
  if (inflight) return inflight
  isLoading.value = true
  fetchError.value = null
  inflight = (async () => {
    try {
      const idx = await invokeTauri<RustMarketplaceIndex>('marketplace_fetch_index')
      if (!idx) {
        throw new Error('empty response')
      }
      entries.value = idx.entries.map(withVerified)
    } catch (e) {
      entries.value = []
      fetchError.value = e instanceof Error ? e.message : String(e)
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

export function useMarketplace(): {
  entries: Readonly<Ref<MarketplaceEntry[]>>
  isLoading: Readonly<Ref<boolean>>
  fetchError: Readonly<Ref<string | null>>
  loadIndex: () => Promise<void>
  installEntry: (id: string) => Promise<void>
} {
  return { entries, isLoading, fetchError, loadIndex, installEntry }
}