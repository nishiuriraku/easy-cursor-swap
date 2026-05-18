<script setup lang="ts">
/**
 * 公式インデックス (Marketplace)
 *
 * design/screens.jsx の `CFMarketplace` を Vue/Nuxt に移植したもの。
 * Phase 5-6 に対応。
 *
 * - GitHub 上の公式メタデータインデックス (`index.json`) から取得
 * - 全エントリを FeaturedCard の横並びレイアウトで 1 グリッドに表示
 * - Ed25519 署名検証済みのテーマのみ掲載 (CI 自動検証)
 * - インポートは Rust 側の `import_from_marketplace` (将来実装) に委譲
 */
import { computed, onMounted, ref, watch } from 'vue'
import type { MarketplaceEntry, MarketplaceName, MarketplaceTag } from '~/types/marketplace'
import { computeFilteredGrid } from '~/pages/marketplace.helpers'
import { invokeTauri } from '~/composables/useTauri'
import { openExternalUrl } from '~/composables/useExternalUrl'
import { useI18n } from '~/composables/useI18n'
import { useThemes } from '~/composables/useThemes'
import { pickLocalizedName } from '~/composables/pickLocalizedName'

const { t, locale } = useI18n()

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

// 詳細モーダル管理
const selectedEntry = ref<MarketplaceEntry | null>(null)
const detailInstalling = ref(false)

function openDetails(id: string) {
  // モーダルを即時表示してから useThemes シングルトンを再同期する。
  // `alreadyInstalled` は themes ref のリアクティブ computed なので、
  // refresh が完了した時点で自動的に再評価され、ボタンは正しい状態に補正される。
  // (Library 画面で削除されていた場合に "ライブラリに追加" が disabled のままになる
  // バグへの defensive refresh。await すると IPC レイテンシ分モーダル表示が遅れるため
  // void で fire-and-forget する。)
  const entry = entries.value.find((x) => x.id === id) ?? null
  selectedEntry.value = entry
  void refreshThemes()
}

function closeDetails() {
  selectedEntry.value = null
}

async function installFromDetail(id: string) {
  detailInstalling.value = true
  try {
    await installEntry(id)
    // 成功してたら詳細モーダルを閉じる
    if (installStatus.value?.kind === 'ok') closeDetails()
  } finally {
    detailInstalling.value = false
  }
}

// --- IPC 経由で受け取る Rust 側スキーマ (snake_case) ---
interface RustMarketplaceEntry {
  id: string
  // Rust 側は LocalizedString (untagged) なので JSON 上は string | { [locale]: string } のどちらも来る。
  name: MarketplaceName
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
  preview_base_url?: string
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
    previewBaseUrl: e.preview_base_url,
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
const filteredGrid = computed(() =>
  computeFilteredGrid(entries.value, filter.value, searchQuery.value),
)

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
    // トースト / ログは displayName (現 locale でピックした文字列) を使う。
    // e.name は LocalizedString の生形 (string | map) なので、{{ name }} 補間に
    // そのまま渡すと map のとき "[object Object]" になる。
    const displayName = pickLocalizedName(e.name, locale.value)
    installStatus.value = { kind: 'ok', name: displayName }
    // インストール直後にライブラリの一覧をリフレッシュ。
    // useThemes はシングルトンなので Library / サイドバーバッジに即反映される。
    void refreshThemes()
    console.info('[Marketplace] installed', displayName)
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    const displayName = pickLocalizedName(e.name, locale.value)
    installStatus.value = { kind: 'err', name: displayName, message }
    console.error('[Marketplace] install failed:', err)
  } finally {
    installingId.value = null
  }
}

async function openGithub() {
  await openExternalUrl('https://github.com/nishiuriraku/easy-cursor-swap-index')
}

// computed にする理由: useI18n の t() は言語切替時に reactive 評価される。
// 静的配列のままだと言語変更後も英語ラベルが残ってしまうので追従させる。
const filters = computed<Array<{ id: MarketplaceTag; label: string }>>(() => [
  { id: 'all', label: t('marketplace.tagAll') },
  { id: 'pixel', label: t('marketplace.tagPixel') },
  { id: 'minimal', label: t('marketplace.tagMinimal') },
  { id: 'animated', label: t('marketplace.tagAnimated') },
  { id: 'dark', label: t('marketplace.tagDark') },
])

onMounted(async () => {
  // marketplace 由来テーマの alreadyInstalled 判定で stale state を踏まないよう、
  // Library のテーマ一覧と marketplace index を並列取得して singleton を最新化する。
  await Promise.all([loadIndex(), refreshThemes()])
})
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
        <!-- 全エントリを一覧 -->
        <div v-if="filteredGrid.length > 0" class="grid">
          <FeaturedCard
            v-for="entry in filteredGrid"
            :key="entry.id"
            :entry="entry"
            @show-details="openDetails"
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

    <!-- Marketplace 詳細モーダル -->
    <MarketplaceDetailModal
      :entry="selectedEntry"
      :installing="detailInstalling"
      @close="closeDetails"
      @install="installFromDetail"
    />
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
