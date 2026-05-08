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
.cpane.left {
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--bg-elev1);
  overflow: hidden;
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
}

.pane-head h6 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-mute);
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  font-size: 11px;
  color: var(--text-mute);
}

.role-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  display: grid;
  gap: 4px;
}
</style>
