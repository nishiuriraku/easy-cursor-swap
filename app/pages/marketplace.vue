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
import { computed, onMounted, ref, watch } from 'vue'
import type { MarketplaceEntry, MarketplaceTag } from '~/types/marketplace'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'
import { useThemes } from '~/composables/useThemes'

const { t } = useI18n()

const submitOpen = ref(false)

const entries = ref<MarketplaceEntry[]>([])
const isLoading = ref(true)
const filter = ref<MarketplaceTag>('all')
const searchQuery = ref('')
const installingId = ref<string | null>(null)

/**
 * インストール結果バナー。
 *  - 成功時: `{ kind: 'ok', name }` を表示しテーマ一覧をリフレッシュ
 *  - 失敗時: `{ kind: 'err', name, message }` を残す
 * 3.5 秒で自動消去する (creator のトーストと同じ仕様)。
 */
type InstallStatus = { kind: 'ok'; name: string } | { kind: 'err'; name: string; message: string }
const installStatus = ref<InstallStatus | null>(null)
let installStatusTimer: ReturnType<typeof setTimeout> | null = null
const INSTALL_TOAST_MS = 3500
watch(installStatus, (next) => {
  if (installStatusTimer) {
    clearTimeout(installStatusTimer)
    installStatusTimer = null
  }
  if (next !== null) {
    installStatusTimer = setTimeout(() => {
      installStatus.value = null
      installStatusTimer = null
    }, INSTALL_TOAST_MS)
  }
})

// テーマ一覧のシングルトンに直接アクセスし、インストール成功時に再取得する。
// これで「Marketplace でインポート → サイドバーバッジ・Library 画面に即時反映」が実現する。
const { refresh: refreshThemes } = useThemes()

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

// --- ハンドラ ---
async function installEntry(id: string) {
  const e = entries.value.find((x) => x.id === id)
  if (!e) return
  installingId.value = id
  installStatus.value = null
  try {
    // Rust 側 (marketplace::MarketplaceClient::install) は検証成功時にテーマ ID を返す。
    // 戻り値は使わないが、await することで成否判定する。
    await invokeTauri<string>('marketplace_install', {
      req: {
        downloadUrl: e.downloadUrl,
        sha256: e.sha256,
        signature: e.signature,
        authorGithub: e.authorGithub,
        authorPubkeyId: e.authorPubkeyId,
      },
    })
    installStatus.value = { kind: 'ok', name: e.name }
    // インストール直後にライブラリの一覧をリフレッシュ。
    // useThemes はシングルトンなので Library / サイドバーバッジに即反映される。
    void refreshThemes()
    console.info('[Marketplace] installed', e.name)
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    installStatus.value = { kind: 'err', name: e.name, message }
    console.error('[Marketplace] install failed:', err)
  } finally {
    installingId.value = null
  }
}

async function openGithub() {
  // Tauri 2 webview は window.open() を許さないので、Rust 側 `open_url` IPC を経由する
  // (内部で Win32 ShellExecuteW を叩いてホスト OS のブラウザを起動)。
  // SubmitThemeDialog と同じパターン。
  const url = 'https://github.com/nishiuriraku/easy-cursor-swap-index'
  try {
    await invokeTauri<void>('open_url', { url })
  } catch (e) {
    // Tauri コンテキスト外 (nuxt dev) のフォールバック
    if (typeof window !== 'undefined') {
      window.open(url, '_blank', 'noopener,noreferrer')
    }
    console.error('openGithub failed:', e)
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

      <!-- インストール結果バナー -->
      <Transition name="fade">
        <div
          v-if="installStatus"
          :class="['install-banner', installStatus.kind]"
          role="status"
          aria-live="polite"
        >
          <UiIcon :name="installStatus.kind === 'ok' ? 'Check' : 'AlertTriangle'" :size="14" />
          <span v-if="installStatus.kind === 'ok'">
            {{ t('marketplace.installedToast', { name: installStatus.name }) }}
          </span>
          <span v-else>
            {{
              t('marketplace.installFailedToast', {
                name: installStatus.name,
                error: installStatus.message,
              })
            }}
          </span>
          <button
            type="button"
            class="install-banner-close"
            :aria-label="t('common.close')"
            @click="installStatus = null"
          >
            <UiIcon name="X" :size="11" />
          </button>
        </div>
      </Transition>

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

    <!-- テーマ提出ダイアログ -->
    <SubmitThemeDialog v-model:open="submitOpen" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.marketplace-host {
  @apply relative flex h-full flex-col;
}

.repo-link {
  @apply ml-3 font-mono text-[12px] font-normal text-fg-mute;
}

.featured-strip {
  @apply mb-6 grid grid-cols-3 gap-3.5;
}

@media (max-width: 1100px) {
  .featured-strip {
    @apply grid-cols-2;
  }
}

@media (max-width: 700px) {
  .featured-strip {
    @apply grid-cols-1;
  }
}

.empty-state,
.error-state {
  @apply flex flex-col items-center justify-center gap-3 px-6 py-20 text-center text-fg-mute;
}
.error-state {
  color: var(--rose, #ff6b8a);
}
.error-state .error-msg {
  @apply m-0 text-[14px] text-fg;
}
.empty-state h3 {
  @apply m-0 font-display text-[18px] font-semibold text-fg;
}
.empty-state p {
  @apply m-0 text-[13px] text-fg-dim;
}

.install-banner {
  @apply mb-4 flex items-center gap-2 rounded-[8px] border px-3 py-2 text-[12.5px];
}
.install-banner.ok {
  @apply border-accent-line text-accent;
  background: rgba(124, 242, 212, 0.08);
}
.install-banner.err {
  @apply text-rose;
  background: rgba(255, 107, 138, 0.08);
  border-color: rgba(255, 107, 138, 0.3);
}
.install-banner-close {
  @apply ml-auto inline-flex size-5 cursor-pointer items-center justify-center rounded-[4px] border-0 bg-transparent text-fg-mute;
}
.install-banner-close:hover {
  @apply text-fg;
  background: rgba(255, 255, 255, 0.06);
}
:where(html.light) .install-banner-close:hover {
  background: rgba(15, 20, 35, 0.06);
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.skeleton-card {
  @apply h-[280px];
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
