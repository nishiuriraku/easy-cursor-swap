<script setup lang="ts">
/**
 * 外観設定の OS 状態インスペクター用 (Light / Dark カード)
 */
const props = defineProps<{
  side: 'light' | 'dark'
  active: boolean
}>()
</script>

<template>
  <div :class="['mode-indicator', side, { active }]">
    <div class="mi-icon" :class="side">
      <UiIcon :name="side === 'light' ? 'Sun' : 'Moon'" :size="18" />
    </div>
    <div class="mi-text">
      <div class="mi-title">{{ side === 'light' ? 'Light Mode' : 'Dark Mode' }}</div>
      <div class="mi-sub">AppsUseLightTheme = {{ side === 'light' ? '1' : '0' }}</div>
    </div>
    <span v-if="active" class="tag ok mi-tag">
      <span class="dot" />
      CURRENT
    </span>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.mode-indicator {
  @apply relative flex items-center gap-3 px-[22px] py-[18px];
}
.mode-indicator.light {
  @apply border-r border-line;
}
.mode-indicator.light.active {
  background: linear-gradient(180deg, rgba(245, 194, 107, 0.08), transparent);
}
.mode-indicator.dark.active {
  background: linear-gradient(180deg, rgba(124, 242, 212, 0.08), transparent);
}

.mi-icon {
  @apply grid size-9 shrink-0 place-items-center rounded-[9px];
}
.mi-icon.light {
  @apply text-amber;
  background: rgba(245, 194, 107, 0.12);
  border: 1px solid rgba(245, 194, 107, 0.3);
}
.mi-icon.dark {
  @apply text-accent;
  background: rgba(124, 242, 212, 0.12);
  border: 1px solid var(--accent-line);
}

.mi-text {
  @apply flex-1;
}
.mi-title {
  @apply font-display text-[14px] font-semibold;
}
.mi-sub {
  @apply font-mono text-[10.5px] text-fg-mute;
}

.mi-tag .dot {
  @apply size-[5px] rounded-full;
  background: currentColor;
  box-shadow: 0 0 6px currentColor;
}

.mode-indicator.light .mi-tag {
  @apply text-amber;
  background: rgba(245, 194, 107, 0.12);
  border-color: rgba(245, 194, 107, 0.3);
}
</style>
