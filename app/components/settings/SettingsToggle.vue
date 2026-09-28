<script setup lang="ts">
/**
 * トグルスイッチ。v-model 互換。
 * `disabled` を渡すと操作不能 + opacity 低下。
 */
const props = withDefaults(
  defineProps<{
    modelValue: boolean
    disabled?: boolean
    /** スクリーンリーダー用のラベル。未指定時は親の SettingsRow label で説明される想定。 */
    label?: string
    /** E2E 用アンカー (`toggle-<anchor>` data-testid になる)。 */
    anchor?: string
  }>(),
  { disabled: false, label: undefined, anchor: undefined },
)

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

function toggle() {
  if (props.disabled) return
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    :class="['toggle', { on: modelValue }]"
    :aria-pressed="modelValue"
    :aria-label="label ?? undefined"
    :data-testid="anchor ? `toggle-${anchor}` : undefined"
    :disabled="disabled"
    @click="toggle"
  >
    <span class="knob" />
  </button>
</template>
