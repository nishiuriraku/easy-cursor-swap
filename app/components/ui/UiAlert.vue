<script setup lang="ts">
/**
 * インラインアラートバナー。`role="alert"` 必須。
 *
 * tone ごとに border / bg / fg / default icon を切り替える:
 * - info     : accent      (Info icon)
 * - success  : accent      (Check icon)
 * - warn     : amber       (AlertTriangle icon)
 * - danger   : rose        (Alert icon)
 *
 * デザイントークンの値はすべて既存 `global.css` / `tailwind.css` の `--accent`
 * / `--rose` / amber RGBA を参照。ダーク/ライト切替は token 側が追従する。
 */
type Tone = 'info' | 'success' | 'warn' | 'danger'

const props = withDefaults(
  defineProps<{
    tone: Tone
    title?: string
    /** 既定は tone 連動。`false` を渡すと非表示。 */
    icon?: string | false
    dense?: boolean
  }>(),
  {
    dense: false,
    icon: undefined,
  },
)

const DEFAULT_ICONS: Record<Tone, string> = {
  info: 'Info',
  success: 'Check',
  warn: 'AlertTriangle',
  danger: 'Alert',
}

const resolvedIcon = computed<string | false>(() => {
  if (props.icon === false) return false
  if (typeof props.icon === 'string') return props.icon
  return DEFAULT_ICONS[props.tone]
})
</script>

<template>
  <div :class="['ui-alert', tone, { dense }]" role="alert">
    <UiIcon v-if="resolvedIcon" :name="resolvedIcon" :size="14" class="ui-alert-icon" />
    <div class="ui-alert-content">
      <strong v-if="title" class="ui-alert-title">{{ title }}</strong>
      <div class="ui-alert-body">
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ui-alert {
  @apply flex items-start gap-2.5 rounded-md border px-3 py-2.5 text-[12px] leading-[1.5];
}
.ui-alert.dense {
  @apply py-1.5;
}
.ui-alert-icon {
  @apply mt-px shrink-0;
}
.ui-alert-content {
  @apply min-w-0 flex-1;
}
.ui-alert-title {
  @apply mb-1 block text-[12px] font-semibold;
}
.ui-alert-body {
  @apply text-fg;
}

.ui-alert.info {
  background: var(--accent-dim);
  border-color: var(--accent-line);
  color: var(--accent);
}
.ui-alert.info .ui-alert-body {
  color: var(--fg);
}
.ui-alert.success {
  background: rgba(124, 242, 212, 0.1);
  border-color: var(--accent-line);
  color: var(--accent);
}
.ui-alert.success .ui-alert-body {
  color: var(--fg);
}
.ui-alert.warn {
  background: rgba(245, 158, 11, 0.1);
  border-color: rgba(245, 158, 11, 0.35);
  color: #f59e0b;
}
.ui-alert.warn .ui-alert-body {
  color: var(--fg);
}
.ui-alert.danger {
  background: rgba(255, 107, 138, 0.08);
  border-color: rgba(255, 107, 138, 0.4);
  color: var(--rose);
}
.ui-alert.danger .ui-alert-body {
  color: var(--fg);
}
</style>
