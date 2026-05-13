<script setup lang="ts">
/**
 * デフォルトレイアウト: Win11 風シェル。
 * 構造: [タイトルバー] / [サイドバー | <slot /> ]
 * ナビ状態は URL パスから派生 (route.path) させ、Sidebar の click で navigateTo。
 *
 * グローバルパニックホットキー (Ctrl+Alt+Shift+R) もここで購読する。
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useAppSettings } from '~/composables/useAppSettings'
import { useI18n } from '~/composables/useI18n'
import { useThemes } from '~/composables/useThemes'
import { invokeTauri } from '~/composables/useTauri'
// `useRoute` / `useRouter` / コンポーネント類は Nuxt の自動インポートで解決

const { config: appConfig, load: loadAppConfig } = useAppSettings()
const { syncFromConfig } = useI18n()
const { themes, refresh: refreshThemes } = useThemes()

const route = useRoute()
const router = useRouter()
const panicOpen = ref(false)

/** サイドバー Marketplace バッジ用: 公式インデックスのテーマ数。 */
const marketplaceCount = ref(0)
const themeCount = computed(() => themes.value.length)

/**
 * Marketplace 件数を 1 度だけ取得しキャッシュする。
 * 失敗時は 0 のまま (バッジを出さない代わりに 0 を表示)。
 * 公式インデックスへの HTTP は app 起動時の 1 回のみ走らせる。
 */
async function loadMarketplaceCount() {
  try {
    const idx = await invokeTauri<{ entries: unknown[] }>('marketplace_fetch_index')
    marketplaceCount.value = idx?.entries?.length ?? 0
  } catch {
    marketplaceCount.value = 0
  }
}

const NAV_ROUTES: Record<string, string> = {
  library: '/',
  creator: '/creator',
  marketplace: '/marketplace',
  settings: '/settings',
}

// パス → ナビ ID の逆引き
const activeNav = computed(() => {
  const entry = Object.entries(NAV_ROUTES).find(([, path]) => path === route.path)
  return entry ? entry[0] : 'library'
})

function onNavigate(id: string) {
  const path = NAV_ROUTES[id]
  if (path && path !== route.path) router.push(path)
}

function onPanic() {
  panicOpen.value = true
}

/**
 * パニックリセット完了通知。
 *
 * Rust 側の `reset_to_default` / `reset_to_initial` も `cursor-changed`
 * イベントを発火しているが、その経路は Tauri の listen に依存する。
 * フロントエンド側の保険として `window` 上にも同名の CustomEvent を投げ、
 * 各ページが「カーソルが外部更新された」前提で再ロードできるようにする。
 *
 * これにより:
 *   - PanicFlow → default.vue: コンポーネント階層を介した直接通知
 *   - reset_to_default IPC → cursor-changed: Tauri event 経由
 *   - PanicFlow → window: DOM event 経由 (ページの listen に直結)
 *
 * のうちどれかが落ちてもアクティブテーマ表示が更新される。
 */
function onPanicDone(_stage: 1 | 2) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent('easycs:cursors-changed'))
}

// グローバルパニックホットキー (Ctrl+Alt+Shift+R)
function onKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.altKey && e.shiftKey && (e.key === 'R' || e.key === 'r')) {
    e.preventDefault()
    panicOpen.value = true
  }
}

// Tauri 側のグローバルホットキー (Rust の RegisterHotKey)
// アプリがフォーカスを持っていなくてもバックグラウンドから panic-hotkey を発火するので
// keydown ハンドラ (フォーカス時のみ) と二重に購読する。
let unlistenHotkey: (() => void) | null = null

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenHotkey = await listen('panic-hotkey', () => {
      panicOpen.value = true
    })
  } catch {
    // Web 開発時はスキップ
  }
  await loadAppConfig()
  syncFromConfig(appConfig.value?.general.language)
  // テーマ一覧 / Marketplace 件数の取得は app 起動の他処理と並行で OK なので await しない。
  // useThemes はシングルトンなので、ここで refresh しておけばどのページでも最新値が読める。
  void refreshThemes()
  void loadMarketplaceCount()
})

// config が後から変わった場合にも追随
watch(
  () => appConfig.value?.general.language,
  (lang) => syncFromConfig(lang),
)

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
  if (unlistenHotkey) unlistenHotkey()
})
</script>

<template>
  <div class="win">
    <!-- スキップナビゲーション: キーボード / スクリーンリーダー用 -->
    <a href="#main-content" class="skip-to-content">メインコンテンツへスキップ</a>

    <AppTitlebar />
    <EnvironmentBanner />
    <div class="body">
      <AppSidebar
        :active="activeNav"
        :theme-count="themeCount"
        :marketplace-count="marketplaceCount"
        @update:active="onNavigate"
        @panic="onPanic"
      />
      <main id="main-content" class="main" tabindex="-1">
        <slot />
      </main>
    </div>

    <!-- パニックリセットフロー (グローバル) -->
    <PanicFlow v-model:open="panicOpen" @done="onPanicDone" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.win {
  @apply relative flex size-full flex-col overflow-hidden bg-bg-0;
  isolation: isolate;
}
.win::before {
  content: '';
  position: absolute;
  inset: -200px;
  background:
    radial-gradient(800px 500px at 12% -10%, rgba(124, 242, 212, 0.08), transparent 60%),
    radial-gradient(900px 600px at 110% 110%, rgba(139, 125, 255, 0.07), transparent 60%);
  pointer-events: none;
  z-index: 0;
}
:where(html.light) .win::before {
  background:
    radial-gradient(800px 500px at 12% -10%, rgba(15, 168, 133, 0.1), transparent 60%),
    radial-gradient(900px 600px at 110% 110%, rgba(106, 92, 255, 0.08), transparent 60%);
}
.body {
  @apply relative z-[1] grid min-h-0 flex-1;
  grid-template-columns: 248px 1fr;
}
.main {
  @apply relative flex min-w-0 flex-col overflow-hidden;
}
.skip-to-content {
  @apply absolute left-2 z-[9999] rounded text-[13px] font-semibold text-black no-underline;
  top: -100%;
  padding: 6px 14px;
  background: var(--accent);
}
.skip-to-content:focus {
  top: 8px;
}
</style>
