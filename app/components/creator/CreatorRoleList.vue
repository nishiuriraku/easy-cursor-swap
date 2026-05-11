<script setup lang="ts">
/**
 * Creator の左ペイン — 17 役割一覧 (リストボックス相当)。
 *
 * キーボード操作 (↑↓/jk/Home/End) は親側の `onRoleListKeydown` を keydown ハンドラとして
 * 受け取り、フォーカス移動を担う。アクティブな役割の判定や filled/partial/empty の
 * ステータス計算も親 (creator.vue) で行い、こちらは props を受けて表示するだけ。
 */
import { useI18n } from '~/composables/useI18n'
import { CURSOR_ROLES, type CursorRoleDef } from '~/components/icons/CursorIcons'

type RoleStatus = 'filled' | 'partial' | 'empty'

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

.pane-head h6 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.tag {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  gap: 4px;
}

.role-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  display: grid;
  gap: 4px;
}
</style>
