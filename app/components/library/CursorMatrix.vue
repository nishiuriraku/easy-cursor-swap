<script setup lang="ts">
/**
 * 17 役割の 6x3 グリッド表示 (テーマカードのプレビュー領域)。
 * `included` に含まれるロールはアイコン描画、含まれないものは斜線埋めの empty セル。
 */
// CursorIcon は Nuxt の自動インポートで解決される
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'

defineProps<{
  included: string[]
}>()
</script>

<template>
  <div class="cursors">
    <div
      v-for="role in CURSOR_ROLES"
      :key="role.id"
      :class="['cell', { empty: !included.includes(role.id) }]"
      :title="role.jp"
    >
      <CursorIcon v-if="included.includes(role.id)" :role="role.id" :size="14" />
    </div>
  </div>
</template>
