<script setup lang="ts">
/**
 * Creator のタブバー (P08a Step 2 / V1)。
 *
 * `.tabs/.tab/.num` は共有 CSS のため scoped スタイル不要。
 */

export type CreatorTabId = string

defineProps<{
  tabs: Array<{ id: CreatorTabId; label: string; count?: string }>
  modelValue: CreatorTabId
}>()

defineEmits<{
  (e: 'update:modelValue', id: CreatorTabId): void
}>()
</script>

<template>
  <div class="tabs">
    <button
      v-for="t in tabs"
      :key="t.id"
      :class="['tab', { active: modelValue === t.id }]"
      @click="$emit('update:modelValue', t.id)"
    >
      {{ t.label }}
      <span v-if="t.count" class="num">{{ t.count }}</span>
    </button>
  </div>
</template>
