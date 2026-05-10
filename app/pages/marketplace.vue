<script setup lang="ts">
/**
 * 公式インデックス (Marketplace)
 *
 * design/screens.jsx の `CFMarketplace` を Vue/Nuxt に移植したもの。
 * Phase 5-6 に対応。
 *
 * - GitHub 上の公式メタデータインデックス (`index.json`) から取得
 * - Featured (3 件) + 一般グリッド表示
 * - Ed25519 署名検証済みのテーマのみ掲載 (CI 自動検証)
 * - インポートは Rust 側の `import_from_marketplace` (将来実装) に委譲
 */
import { computed, onMounted, ref } from 'vue'
import type { MarketplaceEntry, MarketplaceTag } from '~/types/marketplace'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const submitOpen = ref(false)

const entries = ref<MarketplaceEntry[]>([])
const isLoading = ref(true)
const filter = ref<MarketplaceTag>('all')
const searchQuery = ref('')
const installingId = ref<string | null>(null)
const lastSync = computed(() => t('marketplace.lastSyncJustNow'))

// --- IPC 経由で受け取る Rust 側スキーマ (snake_case) ---
interface RustMarketplaceEntry {
  id: string
  name: string
  author: string
  author_github: string
  author_pubkey_id: string
  sha256: string
  signature: string
  download_url: string
  version: string
  included_roles: string[]
  tags: string[]
  homepage?: string
  download_count: number
  highlight?: 'new' | 'popular' | null
}

interface RustMarketplaceIndex {
  schema_version: number
  commit?: string
  entries: RustMarketplaceEntry[]
}

function adaptEntry(e: RustMarketplaceEntry): MarketplaceEntry {
  return {
    id: e.id,
    name: e.name,
    author: e.author,
    authorGithub: e.author_github,
    homepage: e.homepage,
    sha256: e.sha256,
    signature: e.signature,
    authorPubkeyId: e.author_pubkey_id,
    downloadUrl: e.download_url,
    version: e.version,
    downloadCount: e.download_count,
    includedRoles: e.included_roles,
    tags: e.tags,
    highlight: (e.highlight ?? null) as MarketplaceEntry['highlight'],
    verified: true, // 公式インデックス掲載 = CI で署名検証済み
  }
}

const fetchError = ref<string | null>(null)

async function loadIndex() {
  isLoading.value = true
  fetchError.value = null
  try {
    const idx = await invokeTauri<RustMarketplaceIndex>('marketplace_fetch_index')
    if (!idx) {
      throw new Error('empty response')
    }
    entries.value = idx.entries.map(adaptEntry)
  } catch (e) {
    entries.value = []
    fetchError.value = e instanceof Error ? e.message : String(e)
    console.warn('[marketplace] fetch failed:', e)
  } finally {
    isLoading.value = false
  }
}

// --- 計算プロパティ ---
const featured = computed(() =>
  entries.value.filter((e) => e.highlight !== null && e.highlight !== undefined),
)

const filteredGrid = computed(() => {
  let result = [...entries.value]

  if (filter.value !== 'all') {
    result = result.filter((e) => e.tags.includes(filter.value))
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(
      (e) => e.name.toLowerCase().includes(q) || e.author.toLowerCase().includes(q),
    )
  }

  return result
})

const totalAuthors = computed(() => {
  const set = new Set(entries.value.map((e) => e.authorGithub))
  return set.size
})

// --- ハンドラ ---
async function installEntry(id: string) {
  const e = entries.value.find((x) => x.id === id)
  if (!e) return
  installingId.value = id
  try {
    // 将来: invoke('marketplace_install', { downloadUrl, sha256, signature, authorPubkeyId })
    await invokeTauri<void>('marketplace_install', {
      req: {
        downloadUrl: e.downloadUrl,
        sha256: e.sha256,
        signature: e.signature,
        authorGithub: e.authorGithub,
        authorPubkeyId: e.authorPubkeyId,
      },
    })
    console.info('[Marketplace] installed', e.name)
  } catch (err) {
    console.error('[Marketplace] install failed:', err)
  } finally {
    installingId.value = null
  }
}

function openGithub() {
  // 将来: invoke('open_external', { url: 'https://github.com/nishiuriraku/easy-cursor-swap-index' })
  if (typeof window !== 'undefined') {
    window.open(
      'https://github.com/nishiuriraku/easy-cursor-swap-index',
      '_blank',
      'noopener,noreferrer',
    )
  }
}

const filters: Array<{ id: MarketplaceTag; label: string }> = [
  { id: 'all', label: 'All' },
  { id: 'pixel', label: 'Pixel' },
  { id: 'minimal', label: 'Minimal' },
  { id: 'animated', label: 'Animated' },
  { id: 'dark', label: 'Dark' },
]

onMounted(loadIndex)
</script>

<template>
  <div class="marketplace-host">
    <!-- ツールバー -->
    <div class="toolbar">
      <div class="bcrumb">
        <span class="crumb">{{ t('marketplace.breadcrumbCategory') }}</span>
        <span class="sep">/</span>
        <span class="crumb active">{{ t('marketplace.breadcrumbCurrent') }}</span>
      </div>
      <div class="search">
        <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
        <input
          v-model="searchQuery"
          :placeholder="t('marketplace.searchPlaceholder', { count: entries.length })"
          :aria-label="t('common.search')"
        />
        <span class="kbd">⌘K</span>
      </div>
      <div class="tb-actions">
        <button class="btn ghost" @click="openGithub">
          <UiIcon name="Globe" :size="14" />{{ t('marketplace.openGithub') }}
        </button>
        <button class="btn ghost" @click="submitOpen = true">
          <UiIcon name="Upload" :size="14" />{{ t('marketplace.submitBtn') }}
        </button>
      </div>
    </div>

    <!-- コンテンツ -->
    <div class="content">
      <div class="page-head">
        <div>
          <h1>
            {{ t('marketplace.title') }}
            <span class="repo-link">github.com/nishiuriraku/easy-cursor-swap-index</span>
          </h1>
          <p>{{ t('marketplace.description', { count: entries.length }) }}</p>
        </div>
        <div class="right">
          <div class="chips">
            <button
              v-for="f in filters"
              :key="f.id"
              :class="['chip', { active: filter === f.id }]"
              @click="filter = f.id"
            >
              {{ f.label }}
            </button>
          </div>
        </div>
      </div>

      <!-- 取得失敗 -->
      <div v-if="fetchError" class="error-state">
        <UiIcon name="Alert" :size="32" />
        <p class="error-msg">{{ t('marketplace.fetchError') }}</p>
        <button class="btn primary" @click="loadIndex">
          {{ t('marketplace.fetchRetry') }}
        </button>
      </div>

      <!-- ローディング -->
      <div v-else-if="isLoading" class="grid">
        <div v-for="i in 6" :key="i" class="card skeleton-card" />
      </div>

      <template v-else>
        <!-- Featured ストリップ -->
        <div v-if="featured.length > 0" class="featured-strip">
          <FeaturedCard
            v-for="entry in featured"
            :key="entry.id"
            :entry="entry"
            @install="installEntry"
          />
        </div>

        <!-- 一般グリッド -->
        <div v-if="filteredGrid.length > 0" class="grid">
          <MarketplaceCard
            v-for="entry in filteredGrid"
            :key="entry.id"
            :entry="entry"
            @install="installEntry"
          />
        </div>

        <!-- 空状態 -->
        <div v-else class="empty-state">
          <UiIcon name="Search" :size="48" />
          <h3>{{ t('marketplace.emptyTitle') }}</h3>
          <p>{{ t('marketplace.emptyDesc') }}</p>
        </div>
      </template>
    </div>

    <!-- ステータスバー -->
    <AppStatusbar
      :items="[
        { dot: true, text: `Index synced · ${lastSync}` },
        { text: `${entries.length} themes · ${totalAuthors} authors` },
        { text: 'schema v3.2' },
      ]"
    />

    <!-- テーマ提出ダイアログ -->
    <SubmitThemeDialog v-model:open="submitOpen" />
  </div>
</template>

<style scoped>
.marketplace-host {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
}

.repo-link {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--fg-mute);
  font-weight: 400;
  margin-left: 12px;
}

.featured-strip {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 14px;
  margin-bottom: 24px;
}

@media (max-width: 1100px) {
  .featured-strip {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 700px) {
  .featured-strip {
    grid-template-columns: 1fr;
  }
}

.empty-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  text-align: center;
  color: var(--fg-mute);
  gap: 12px;
}
.error-state {
  color: var(--rose, #ff6b8a);
}
.error-state .error-msg {
  margin: 0;
  font-size: 14px;
  color: var(--fg);
}
.empty-state h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 18px;
  color: var(--fg);
  font-weight: 600;
}
.empty-state p {
  margin: 0;
  font-size: 13px;
  color: var(--fg-dim);
}

.skeleton-card {
  height: 280px;
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.02) 0%,
    rgba(255, 255, 255, 0.04) 50%,
    rgba(255, 255, 255, 0.02) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.4s ease-in-out infinite;
}
@keyframes shimmer {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}
</style>
