<script setup lang="ts">
/**
 * デフォルトレイアウト: Win11 風シェル。
 * 構造: [タイトルバー] / [サイドバー | <slot /> ]
 * ナビ状態は URL パスから派生 (route.path) させ、Sidebar の click で navigateTo。
 *
 * グローバルパニックホットキー (Ctrl+Alt+Shift+R) もここで購読する。
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useAppConfig } from '~/composables/useAppConfig'
import { useI18n } from '~/composables/useI18n'
// `useRoute` / `useRouter` / コンポーネント類は Nuxt の自動インポートで解決

const { config: appConfig, load: loadAppConfig } = useAppConfig()
const { syncFromConfig } = useI18n()

const route = useRoute()
const router = useRouter()
const panicOpen = ref(false)

const NAV_ROUTES: Record<string, string> = {
  library: '/',
  creator: '/creator',
  marketplace: '/marketplace',
  settings: '/settings',
  appearance: '/appearance',
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
        :theme-count="0"
        :marketplace-count="248"
        @update:active="onNavigate"
        @panic="onPanic"
      />
      <main id="main-content" class="main" tabindex="-1">
        <slot />
      </main>
    </div>

    <!-- パニックリセットフロー (グローバル) -->
    <PanicFlow v-model:open="panicOpen" />
  </div>
</template>
