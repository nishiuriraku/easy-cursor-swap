<script setup lang="ts">
/**
 * クリエイターモード左ペインの 17 役割リスト項目。
 * filled (アセット有り) / empty (未割当) の状態ドット付き。
 */
import type { CursorRoleDef } from '~/components/icons/CursorIcons'

defineProps<{
  role: CursorRoleDef
  index: number
  status: 'filled' | 'empty'
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

.role {
  @apply grid w-full cursor-pointer items-center gap-2.5 rounded-[7px] border border-transparent bg-transparent px-2.5 py-2 text-left text-[12.5px] text-fg-dim;
  grid-template-columns: 20px 1fr auto auto;
}
.role:hover {
  @apply text-fg;
  background: rgba(255, 255, 255, 0.03);
}
.role.active {
  @apply border-accent-line text-fg;
  background: rgba(124, 242, 212, 0.06);
}
.role-label {
  @apply flex min-w-0 items-center gap-2;
}
.ridx,
.rkey {
  @apply font-mono text-[10px] text-fg-mute;
}
.rstatus {
  @apply size-1.5 rounded-full bg-fg-faint;
}
.rstatus.filled {
  @apply bg-accent;
  box-shadow: 0 0 6px var(--accent);
}
</style>
