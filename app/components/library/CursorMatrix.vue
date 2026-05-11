<script setup lang="ts">
/**
 * 17 役割の 6x3 グリッド表示 (テーマカードのプレビュー領域)。
 * - `included` に含まれるロールはアイコン描画、含まれないものは斜線埋めの empty セル。
 * - `previewMap` (任意) を渡すと、該当ロールのみ実際の PNG カーソル画像を表示する。
 *   渡されなければ従来どおり SVG ベクター (CursorIcon) を表示する。
 */
// CursorIcon は Nuxt の自動インポートで解決される
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'

defineProps<{
  included: string[]
  /** ロール ID → Blob URL のマップ。指定ロールはこの URL を img として表示する。 */
  previewMap?: Record<string, string> | null
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
      <template v-if="included.includes(role.id)">
        <img
          v-if="previewMap && previewMap[role.id]"
          :src="previewMap[role.id]"
          :alt="role.jp"
          class="cell-img"
        />
        <CursorIcon v-else :role="role.id" :size="14" />
      </template>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.cell-img {
  @apply size-[18px] object-contain;
  /* image-rendering は複数フォールバック値のスタックなので CSS リテラルで残す。 */
  image-rendering: -webkit-optimize-contrast;
  image-rendering: pixelated;
  image-rendering: crisp-edges;
}
</style>
