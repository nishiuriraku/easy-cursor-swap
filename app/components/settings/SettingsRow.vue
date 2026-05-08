<script setup lang="ts">
/**
 * 設定画面の汎用行: ラベル + 説明 + 任意コントロール (toggle / select 等)
 * コントロールはデフォルトスロットで差し込む。
 */
defineProps<{
  label: string
  desc?: string
  /** ラベルを等幅にする (config パスやキー名表示時) */
  mono?: boolean
}>()
</script>

<template>
  <div class="settings-row">
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
.settings-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 16px;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--line);
}
.settings-row:last-child {
  border-bottom: none;
}
.row-label {
  font-size: 13px;
  font-weight: 500;
}
.row-label.mono {
  font-family: var(--font-mono);
}
.row-desc {
  font-size: 11.5px;
  color: var(--fg-mute);
  margin-top: 3px;
  line-height: 1.5;
}
</style>
