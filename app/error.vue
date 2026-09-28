<script setup lang="ts">
/**
 * Nuxt 致命エラー画面 (P11c。`app.vue` と同階層 = `app/` 直下に置く)。
 *
 * ルート解決不能など `<NuxtLayout>` に届く前のエラーを `AppErrorFallback` の
 * 最小レイアウトで表示する。再起動は Tauri の `relaunch()`、Tauri 外では
 * トップ (`/`) への `clearError` リダイレクト。
 */
import type { NuxtError } from '#app'

const props = defineProps<{
  error: NuxtError
}>()

const captured = computed(() =>
  toCapturedError(props.error, 'nuxt', String(props.error.statusCode ?? '')),
)

async function reload() {
  try {
    const { relaunch } = await import('@tauri-apps/plugin-process')
    await relaunch()
  } catch {
    await clearError({ redirect: '/' })
  }
}
</script>

<template>
  <div class="error-page">
    <AppErrorFallback :error="captured" @reload="reload" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.error-page {
  @apply grid min-h-screen place-items-center bg-bg-0 p-6;
}
.error-page > * {
  @apply w-full max-w-2xl;
}
</style>
