<script setup lang="ts">
/**
 * 描画エラーバウンダリ (P11c)。
 *
 * `layouts/default.vue` の `<main>` 内 `<slot />` を包み、ページ描画中の未捕捉例外を
 * `useAppErrorState` に集約して `AppErrorFallback` に置き換える (`return false` で
 * 伝播停止)。サイドバー / タイトルバーは境界の外なので生きたまま。
 *
 * `clear()` (続行を試す) で `<slot>` が再マウントされる。壊れたページに戻る可能性は
 * あるが、サイドバーから別ページへ移動する逃げ道があるので許容する。
 */
const { error, capture, clear } = useAppErrorState()

onErrorCaptured((err, _instance, info) => {
  capture(err, 'render', info)
  return false
})

async function reload() {
  try {
    const { relaunch } = await import('@tauri-apps/plugin-process')
    await relaunch()
  } catch {
    if (typeof window !== 'undefined') window.location.reload()
  }
}
</script>

<template>
  <slot v-if="!error" />
  <div v-else class="eb-host">
    <AppErrorFallback :error="error" show-dismiss @reload="reload" @dismiss="clear" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.eb-host {
  @apply p-4;
}
</style>
