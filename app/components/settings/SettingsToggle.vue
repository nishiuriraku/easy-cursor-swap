<script setup lang="ts">
/**
 * トグルスイッチ。v-model 互換。
 */
const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

function toggle() {
  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    :class="['toggle', { on: modelValue }]"
    :aria-pressed="modelValue"
    @click="toggle"
  >
    <span class="knob" />
  </button>
</template>
