<script setup lang="ts">
/**
 * 確定 / 不確定プログレスバー。バックエンドが進捗を出せる操作
 * (creator export / bulk import / updater DL) で使う汎用バー。
 *
 * - value / max: 確定モード。`pct = clamp(value / max * 100, 0, 100)`。
 * - indeterminate: 進捗値が無いときの往復アニメーション (max=0 でも自動)。
 * - label (or default slot): バー上部の見出し。showPercent で右端に % を出す。
 *
 * a11y: トラックに `role="progressbar"`。確定時のみ `aria-valuenow/min/max`。
 * CreatorMetadataPane の export-progress バー (唯一の確定バー実装) を汎用化したもの (LD3)。
 */
const props = withDefaults(
  defineProps<{
    value?: number
    max?: number
    indeterminate?: boolean
    label?: string
    showPercent?: boolean
    ariaLabel?: string
  }>(),
  {
    value: 0,
    max: 100,
    indeterminate: false,
    showPercent: false,
  },
)

const slots = useSlots()

/** max<=0 は不確定扱い (確定値を出せないため往復アニメに倒す)。 */
const isIndeterminate = computed(() => props.indeterminate || props.max <= 0)

const pct = computed(() => {
  if (isIndeterminate.value) return 0
  return Math.min(100, Math.max(0, (props.value / props.max) * 100))
})
const pctRounded = computed(() => Math.round(pct.value))

const hasHead = computed(() => Boolean(props.label || props.showPercent || slots.default))
</script>

<template>
  <div class="ui-progress">
    <div v-if="hasHead" class="ui-progress-head">
      <span class="ui-progress-label"
        ><slot>{{ props.label }}</slot></span
      >
      <span v-if="props.showPercent && !isIndeterminate" class="ui-progress-pct"
        >{{ pctRounded }}%</span
      >
    </div>
    <div
      class="ui-progress-track"
      role="progressbar"
      :aria-label="props.ariaLabel || props.label || undefined"
      aria-valuemin="0"
      :aria-valuemax="isIndeterminate ? undefined : 100"
      :aria-valuenow="isIndeterminate ? undefined : pctRounded"
    >
      <div
        class="ui-progress-fill"
        :class="{ indeterminate: isIndeterminate }"
        :style="isIndeterminate ? undefined : { width: `${pct}%` }"
      />
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ui-progress {
  @apply grid gap-1.5;
}
.ui-progress-head {
  @apply flex items-center justify-between gap-2 text-[12px] text-fg-mute;
}
.ui-progress-label {
  @apply min-w-0 truncate;
}
.ui-progress-pct {
  @apply shrink-0 font-mono text-fg-dim;
}
.ui-progress-track {
  @apply h-1.5 overflow-hidden rounded-full;
  background: var(--line);
}
.ui-progress-fill {
  @apply h-full rounded-full;
  background: linear-gradient(90deg, var(--accent), #5dd9bd);
  transition: width 200ms ease;
}
.ui-progress-fill.indeterminate {
  width: 40%;
  border-radius: 999px;
  animation: ui-progress-slide 1.1s ease-in-out infinite;
}
@keyframes ui-progress-slide {
  0% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(320%);
  }
}
</style>
