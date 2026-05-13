<script setup lang="ts">
/**
 * カーソルプレビューグリッド。
 * - 既定 (`limit` 未指定): 17 役割の 6x3 グリッド。`included` に含まれないロールは斜線埋めの empty セル。
 * - `limit` 指定時: `CURSOR_ROLES` 正規順に `included` をフィルタした上で先頭 N 個を描画 (empty セルなし)。
 *   バックエンド (`metadata.cursors.keys()`) の順序が HashMap 起因で不定でも、
 *   フロント側でロール並びを安定化するため、テーマ間で「Arrow → Help → ...」の順を保証する。
 *   テーマが N 個未満しか持たない場合はその数だけ表示する。
 * - `previewMap` (任意) を渡すと、該当ロールのみ実際の PNG カーソル画像を表示する。
 *   渡されなければ従来どおり SVG ベクター (CursorIcon) を表示する。
 */
import { computed } from 'vue'
// CursorIcon は Nuxt の自動インポートで解決される
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'

const props = defineProps<{
  included: string[]
  /** ロール ID → Blob URL のマップ。指定ロールはこの URL を img として表示する。 */
  previewMap?: Record<string, string> | null
  /** 表示セル数の上限。指定すると `CURSOR_ROLES` 正規順に絞り込み、先頭 N 個のみ描画する。 */
  limit?: number
  /** グリッドの列数。既定 6 (従来の 6x3 表示)。3 を指定するとライブラリカード向けの 3x2 表示になる。 */
  cols?: 3 | 6
}>()

/** 描画対象セル。limit 指定時のみ included ベース、未指定時は従来の 17 セル表示。 */
const displayCells = computed<Array<{ role: CursorRoleDef; included: boolean }>>(() => {
  if (props.limit == null) {
    return CURSOR_ROLES.map((role) => ({
      role,
      included: props.included.includes(role.id),
    }))
  }
  // CURSOR_ROLES の canonical 順で included をフィルタ → 先頭 N 個。
  // バックエンドの included_roles は HashMap 起因で順序が不定なので、フロントで安定化する。
  const includedSet = new Set(props.included)
  return CURSOR_ROLES.filter((role) => includedSet.has(role.id))
    .slice(0, props.limit)
    .map((role) => ({ role, included: true }))
})
</script>

<template>
  <div :class="['cursors', props.cols === 3 ? 'cols-3' : 'cols-6']">
    <div
      v-for="cell in displayCells"
      :key="cell.role.id"
      :class="['cell', { empty: !cell.included }]"
      :title="cell.role.jp"
    >
      <template v-if="cell.included">
        <img
          v-if="previewMap && previewMap[cell.role.id]"
          :src="previewMap[cell.role.id]"
          :alt="cell.role.jp"
          class="cell-img"
        />
        <CursorIcon v-else :role="cell.role.id" :size="14" />
      </template>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.cursors {
  @apply grid gap-1;
}
/* Tailwind v4 のコンテンツスキャナがリテラルで拾えるよう grid-cols-* は静的記述する。 */
.cursors.cols-6 {
  @apply grid-cols-6;
}
.cursors.cols-3 {
  @apply grid-cols-3;
}
.cell {
  @apply relative grid aspect-square place-items-center rounded-[5px] border border-line text-fg;
  background: rgba(255, 255, 255, 0.03);
}
.cell.empty {
  @apply text-fg-faint;
  background: repeating-linear-gradient(
    -45deg,
    rgba(255, 255, 255, 0.025),
    rgba(255, 255, 255, 0.025) 3px,
    transparent 3px,
    transparent 6px
  );
  border-color: rgba(255, 255, 255, 0.04);
}
.cell svg {
  @apply size-[14px];
}
/* cols=3 (ライブラリカードの 3x2 表示) は cell 幅が広くなりすぎるため、
 * matrix 自体を中央寄せ + 幅制限し、その分アイコン/画像を大きく見せる。 */
.cursors.cols-3 {
  @apply mx-auto;
  max-width: 216px;
}
.cursors.cols-3 .cell svg {
  @apply size-[22px];
}
:where(html.light) .cell {
  background: rgba(15, 20, 35, 0.025);
}
:where(html.light) .cell.empty {
  background: repeating-linear-gradient(
    -45deg,
    rgba(15, 20, 35, 0.04),
    rgba(15, 20, 35, 0.04) 3px,
    transparent 3px,
    transparent 6px
  );
}
.cell-img {
  @apply size-[18px] object-contain;
  /* image-rendering は複数フォールバック値のスタックなので CSS リテラルで残す。 */
  image-rendering: -webkit-optimize-contrast;
  image-rendering: pixelated;
  image-rendering: crisp-edges;
}
.cursors.cols-3 .cell-img {
  @apply size-[30px];
}
</style>
