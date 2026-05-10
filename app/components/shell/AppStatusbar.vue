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
  <div
    class="flex h-7 shrink-0 items-center gap-4 border-t border-line bg-[rgba(10,11,15,0.6)] px-4 font-mono text-[10.5px] tracking-[0.04em] text-fg-mute"
  >
    <template v-for="(it, i) in items" :key="i">
      <span v-if="i > 0" class="text-fg-faint">·</span>
      <span class="flex items-center gap-1.5">
        <span
          v-if="it.dot"
          class="size-1.5 rounded-full bg-accent shadow-[0_0_6px_var(--color-accent)]"
        />
        {{ it.text }}
      </span>
    </template>
  </div>
</template>
