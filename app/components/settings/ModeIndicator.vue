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
.mode-indicator {
  padding: 18px 22px;
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
}
.mode-indicator.light {
  border-right: 1px solid var(--line);
}
.mode-indicator.light.active {
  background: linear-gradient(180deg, rgba(245, 194, 107, 0.08), transparent);
}
.mode-indicator.dark.active {
  background: linear-gradient(180deg, rgba(124, 242, 212, 0.08), transparent);
}

.mi-icon {
  width: 36px;
  height: 36px;
  border-radius: 9px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}
.mi-icon.light {
  background: rgba(245, 194, 107, 0.12);
  border: 1px solid rgba(245, 194, 107, 0.3);
  color: var(--amber);
}
.mi-icon.dark {
  background: rgba(124, 242, 212, 0.12);
  border: 1px solid var(--accent-line);
  color: var(--accent);
}

.mi-text {
  flex: 1;
}
.mi-title {
  font-family: var(--font-display);
  font-size: 14px;
  font-weight: 600;
}
.mi-sub {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
}

.mi-tag .dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
  box-shadow: 0 0 6px currentColor;
}

.mode-indicator.light .mi-tag {
  background: rgba(245, 194, 107, 0.12);
  color: var(--amber);
  border-color: rgba(245, 194, 107, 0.3);
}
</style>
