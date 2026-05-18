<script setup lang="ts">
/**
 * ThemeDetailDrawer の下段アクションレールを担う子コンポーネント。
 *
 * - 通常テーマ: edit / export / duplicate / delete / apply
 * - Marketplace 出自: edit/export を隠し "編集できません" ヒント表示
 * - システムスキーム: export のみ可、その他は隠し read-only ラベル表示
 *
 * すべてのアクションは emit でコンテナに伝え、IPC 呼出は親 (pages/index.vue) 側で行う。
 */
import type { ThemeCardData } from '~/types/theme'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  apply: [id: string]
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
}>()

const isSystem = computed(() => props.theme.kind === 'system')
const isMarketplace = computed(() => props.theme.kind === 'marketplace')
</script>

<template>
  <footer class="td-foot">
    <div class="td-foot-l">
      <template v-if="!isSystem">
        <button
          v-if="!isMarketplace"
          class="td-act"
          :aria-label="t('themeDetail.editAria', { name: theme.name })"
          @click="emit('edit', theme.id)"
        >
          <UiIcon name="Brush" :size="13" />{{ t('themeDetail.editLabel') }}
        </button>
        <button
          v-if="!isMarketplace"
          class="td-act"
          :aria-label="t('themeDetail.exportAria', { name: theme.name })"
          @click="emit('exportPack', theme.id)"
        >
          <UiIcon name="Export" :size="13" />{{ t('themeDetail.exportLabel') }}
        </button>
        <button
          class="td-act"
          :aria-label="t('themeDetail.duplicateAria', { name: theme.name })"
          @click="emit('duplicate', theme.id)"
        >
          <UiIcon name="Plus" :size="13" />{{ t('themeDetail.duplicateLabel') }}
        </button>
        <button
          class="td-act danger"
          :disabled="theme.isActive"
          :aria-label="
            theme.isActive
              ? t('themeDetail.deleteDisabledAria', { name: theme.name })
              : t('themeDetail.deleteAria', { name: theme.name })
          "
          :title="theme.isActive ? t('themeDetail.deleteDisabledTitle') : undefined"
          @click="emit('delete', theme.id)"
        >
          {{ t('themeDetail.deleteLabel') }}
        </button>
        <p v-if="isMarketplace" class="td-marketplace-hint">
          {{ t('themeDetail.cannotEditMarketplace') }}
        </p>
      </template>
      <template v-else>
        <!--
          システムスキームは編集・複製・削除はできないが、`.cursorpack`
          として書き出して別環境へ持ち運ぶことはできる。Rust 側の
          `export_windows_scheme_as_cursorpack` が `%SystemRoot%\cursors\*`
          を読み取って zip 化するため、ローカルディレクトリは不要。
        -->
        <button
          class="td-act"
          :aria-label="t('themeDetail.exportSchemeAria', { name: theme.name })"
          @click="emit('exportPack', theme.id)"
        >
          <UiIcon name="Export" :size="13" />{{ t('themeDetail.exportSchemeLabel') }}
        </button>
        <span class="td-source mono">
          <UiIcon name="Globe" :size="11" />{{ t('themeDetail.systemSchemeReadOnly') }}
        </span>
      </template>
    </div>
    <div class="td-foot-r">
      <button
        v-if="theme.isActive"
        class="btn"
        disabled
        style="opacity: 0.6; cursor: default; height: 32px"
      >
        <UiIcon name="Check" :size="13" />{{ t('themeDetail.applyingNow') }}
      </button>
      <button v-else class="btn primary" style="height: 32px" @click="emit('apply', theme.id)">
        {{ t('themeDetail.applyTheme') }}
      </button>
    </div>
  </footer>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-foot {
  @apply flex items-center justify-between gap-3 px-[18px] py-3;
  background: rgba(0, 0, 0, 0.18);
}
:where(html.light) .td-foot {
  background: rgba(15, 20, 35, 0.025);
}
.td-foot-l {
  @apply flex items-center gap-1.5;
}
.td-foot-r {
  @apply flex items-center gap-3;
}

.td-act {
  @apply inline-flex h-[30px] cursor-pointer items-center gap-1.5 rounded-md border border-transparent bg-transparent px-[11px] text-[12px] font-medium text-fg-dim;
  transition: all 0.12s;
}
.td-act:hover {
  @apply border-line-hi text-fg;
  background: rgba(255, 255, 255, 0.05);
}
:where(html.light) .td-act:hover {
  background: rgba(15, 20, 35, 0.04);
}
.td-act.danger {
  color: var(--rose);
}
.td-act.danger:hover {
  background: rgba(255, 107, 138, 0.08);
  border-color: rgba(255, 107, 138, 0.3);
  color: #fff;
}
:where(html.light) .td-act.danger:hover {
  color: var(--rose);
}
.td-act.danger:disabled,
.td-act.danger:disabled:hover {
  @apply cursor-not-allowed;
  opacity: 0.45;
  background: transparent;
  border-color: transparent;
  color: var(--rose);
}

.td-source {
  @apply inline-flex items-center gap-1.5 font-mono text-[10.5px] tracking-[0.02em] text-fg-mute;
}

.td-marketplace-hint {
  @apply m-0 ml-auto text-[11px] text-fg-mute;
}
</style>
