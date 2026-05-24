<script setup lang="ts">
/**
 * 共通ボタン。Tailwind v4 shared `.btn` クラスの薄い Vue ラッパ。
 *
 * - variant: 'default' (.btn のみ) / 'primary' / 'ghost' / 'danger'
 * - size:    'md' (h-8 既定) / 'icon' (w-8 正方形、ariaLabel 推奨)
 * - loading: spinner 表示 + 自動 disabled
 *
 * loading=true 時は iconLeft の位置に <span class="spinner"> を差し替える。
 * iconRight は loading 中も維持する。
 */
const props = withDefaults(
  defineProps<{
    variant?: 'default' | 'primary' | 'ghost' | 'danger'
    size?: 'md' | 'icon'
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
    iconLeft?: string
    iconRight?: string
    ariaLabel?: string
  }>(),
  {
    variant: 'default',
    size: 'md',
    loading: false,
    disabled: false,
    type: 'button',
  },
)

defineEmits<{
  click: [event: MouseEvent]
}>()

const isDisabled = computed(() => props.disabled || props.loading)

const buttonClasses = computed(() => {
  const out: string[] = ['btn']
  if (props.variant !== 'default') out.push(props.variant)
  if (props.size === 'icon') out.push('icon')
  return out
})
</script>

<template>
  <button
    :type="type"
    :class="buttonClasses"
    :disabled="isDisabled"
    :aria-label="ariaLabel"
    @click="(e) => !isDisabled && $emit('click', e)"
  >
    <span v-if="loading" class="spinner" style="width: 13px; height: 13px" aria-hidden="true" />
    <UiIcon v-else-if="iconLeft" :name="iconLeft" :size="13" />
    <slot />
    <UiIcon v-if="iconRight" :name="iconRight" :size="13" />
  </button>
</template>
