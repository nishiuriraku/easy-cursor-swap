<script setup lang="ts">
/**
 * グリッド用カード型スケルトン。テーマ一覧 / marketplace グリッドのロード中
 * プレースホルダ。
 *
 * `.card` (border + radius) に共有 `.skeleton` シマーを重ねるだけ。元々
 * index.vue / marketplace.vue が個別 scoped で `.skeleton-card` を二重定義していた
 * (LD2 でこのコンポーネントに集約)。
 */
withDefaults(
  defineProps<{
    height?: string
  }>(),
  {
    height: '280px',
  },
)
</script>

<template>
  <div class="card skeleton" :style="{ height }" aria-hidden="true" />
</template>
