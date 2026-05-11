<script setup lang="ts">
/**
 * クリエイターモード左ペインの 17 役割リスト項目。
 * filled (全6サイズ) / partial (一部) / empty の状態ドット付き。
 */
import type { CursorRoleDef } from '~/components/icons/CursorIcons'

defineProps<{
  role: CursorRoleDef
  index: number
  status: 'filled' | 'partial' | 'empty'
  active: boolean
}>()

defineEmits<{
  select: [id: string]
}>()
</script>

<template>
  <button :class="['role', { active }]" @click="$emit('select', role.id)">
    <span class="ridx">{{ String(index).padStart(2, '0') }}</span>
    <span class="role-label">
      <CursorIcon :role="role.id" :size="14" />
      <span>{{ role.jp }}</span>
    </span>
    <span class="rkey">{{ role.id }}</span>
    <span :class="['rstatus', status]" />
  </button>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* `.role` 本体のレイアウトは global.css で定義。ここでは role-label のみ。 */
.role-label {
  @apply flex min-w-0 items-center gap-2;
}
</style>
