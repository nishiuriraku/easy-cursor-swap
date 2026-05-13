<script setup lang="ts">
/**
 * 設定画面の汎用行: ラベル + 説明 + 任意コントロール (toggle / select 等)
 * コントロールはデフォルトスロットで差し込む。
 *
 * `anchor` を渡すと `data-search-anchor` 属性が付き、設定検索からジャンプ可能になる。
 */
defineProps<{
  label: string
  desc?: string
  /** ラベルを等幅にする (config パスやキー名表示時) */
  mono?: boolean
  /** 設定検索ジャンプ用アンカー (セクション内一意) */
  anchor?: string
}>()
</script>

<template>
  <div class="settings-row" :data-search-anchor="anchor">
    <div class="row-text">
      <div :class="['row-label', { mono }]">{{ label }}</div>
      <div v-if="desc" class="row-desc">{{ desc }}</div>
    </div>
    <div class="row-control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.settings-row {
  @apply grid grid-cols-[1fr_auto] items-center gap-4 border-b border-line py-3;
}
.settings-row:last-child {
  @apply border-b-0;
}
.row-label {
  @apply text-[13px] font-medium;
}
.row-label.mono {
  @apply font-mono;
}
.row-desc {
  @apply mt-[3px] text-[11.5px] leading-[1.5] text-fg-mute;
}

/* 設定検索からジャンプした際の 1.5s パルスハイライト */
.settings-row.is-search-hit {
  animation: search-hit-pulse 1.5s ease-out;
}
@keyframes search-hit-pulse {
  0% {
    background: rgba(124, 242, 212, 0.18);
  }
  100% {
    background: transparent;
  }
}
</style>
