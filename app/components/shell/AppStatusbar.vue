<script setup lang="ts">
/**
 * メインビュー下部のステータスバー。
 * `items` 配列で表示内容を可変化。先頭要素のみ脈動ドット付与可。
 */
interface StatItem {
  text: string
  dot?: boolean
}

defineProps<{
  items: StatItem[]
}>()
</script>

<template>
  <div class="statbar">
    <template v-for="(it, i) in items" :key="i">
      <span v-if="i > 0" class="sep">·</span>
      <span class="stat">
        <span v-if="it.dot" class="dot" />
        {{ it.text }}
      </span>
    </template>
  </div>
</template>

<style scoped>
/* Tailwind v4: scoped style 内で @apply を使うには @reference でテーマを取り込む必要がある。 */
@reference '~/assets/css/tailwind.css';

/* legacy global.css の `.statbar` を Vue scoped + @apply に移植。
 * 背景は `--bg-statbar` (global.css 内で `html.light` が値を上書き) を経由し、
 * Tailwind の `bg-bg-statbar` utility がライト/ダーク両方で正しい色を返す。 */
.statbar {
  @apply flex h-7 shrink-0 items-center gap-4 border-t border-line bg-bg-statbar px-4 font-mono text-[10.5px] tracking-[0.04em] text-fg-mute;
}
.sep {
  @apply text-fg-faint;
}
.stat {
  @apply flex items-center gap-1.5;
}
.stat .dot {
  @apply size-1.5 rounded-full bg-accent;
  box-shadow: 0 0 6px var(--accent);
}
</style>
