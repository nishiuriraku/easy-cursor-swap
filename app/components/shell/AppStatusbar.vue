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
