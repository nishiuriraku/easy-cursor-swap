<script setup lang="ts">
/**
 * トグルスイッチ。v-model 互換。
 * `disabled` を渡すと操作不能 + opacity 低下。
 */
const props = withDefaults(
  defineProps<{
    modelValue: boolean
    disabled?: boolean
  }>(),
  { disabled: false },
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
    :disabled="disabled"
    @click="toggle"
  >
    <span class="knob" />
  </button>
</template>
