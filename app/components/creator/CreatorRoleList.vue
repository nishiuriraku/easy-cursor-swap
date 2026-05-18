<script setup lang="ts">
/**
 * Creator の左ペイン — 17 役割一覧 (リストボックス相当)。
 *
 * キーボード操作 (↑↓/jk/Home/End) は親側の `onRoleListKeydown` を keydown ハンドラとして
 * 受け取り、フォーカス移動を担う。アクティブな役割の判定や filled/partial/empty の
 * ステータス計算も親 (creator.vue) で行い、こちらは props を受けて表示するだけ。
 */
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'

type RoleStatus = 'filled' | 'empty'

const { t } = useI18n()

defineProps<{
  filledCount: number
  activeRoleId: string
  statusOf: (id: string) => RoleStatus
}>()

defineEmits<{
  (e: 'select', id: string): void
  (e: 'keydown', ev: KeyboardEvent): void
}>()

defineExpose({ CURSOR_ROLES })
// 子テンプレートで使うために再公開
const cursorRoles: readonly CursorRoleDef[] = CURSOR_ROLES
</script>

<template>
  <div class="cpane left">
    <div class="pane-head">
      <h6>{{ t('creator.rolesPaneTitle') }}</h6>
      <span class="tag">{{ filledCount }} / 17</span>
    </div>
    <div
      class="role-list"
      role="listbox"
      :aria-label="t('creator.rolesPaneTitle')"
      @keydown="$emit('keydown', $event)"
    >
      <RoleListItem
        v-for="(role, i) in cursorRoles"
        :key="role.id"
        :role="role"
        :index="i"
        :status="statusOf(role.id)"
        :active="activeRoleId === role.id"
        @select="$emit('select', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* NOTE: 元コードは var(--border) や var(--bg-elev1/2)、var(--text-mute) などの
 * 未定義トークンに依存しており、それらの declaration は invalid → global.css の
 * .cpane.left / .tag / .role-list 等のルールがカスケードで効いていた。
 *
 * Tailwind の `border` utility を @apply で持ち込むと border-color が
 * currentColor に化けて global の subtle border-color (--line) を上書きしてしまう
 * 問題が確認できたため、scoped style では layout/spacing の独自上書きのみを
 * 純粋な CSS リテラルで記述し、border/background/color などは global にゆだねる。 */

.cpane {
  padding: 14px;
  overflow: auto;
  min-width: 0;
}
.cpane.left {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.01);
}

@media (max-width: 880px) {
  .cpane.left {
    grid-column: 1;
    grid-row: 1;
    border-right: none;
    border-bottom: 1px solid var(--line);
    max-height: 200px;
  }
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
}

/* design/styles.css の `.nav-section h6` パターン (mono uppercase) と一致させる。
 * 旧 scoped は font-size:12px / weight:600 / tracking:0.04em のため visual drift していた。 */
.pane-head h6 {
  @apply m-0 font-mono text-[10px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}

/* `.tag` の shared utility をそのまま使う (design: padding 3px 8px / font-size 10px /
 * border-radius 4px の四角寄りタグ)。旧 scoped は pill 形状 + 大きめ font に
 * drift していた。ローカル上書きは削除して shared に統一。 */

.role-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  display: grid;
  gap: 4px;
}
</style>
