<script setup lang="ts">
/**
 * テーマ詳細モーダル
 *
 * カードのインライン展開だと開閉トグルが分かりにくく、複数同時展開で
 * グリッドが乱れる UX 問題があったため、シェブロン押下で中央オーバーレイ
 * のモーダルに切り替えた。
 *
 * 共通 `<UiModal>` シェル (Teleport + バックドロップ + Esc + スクロールロック +
 * focus trap) にラップを委譲し、ここでは:
 *   - body slot     : ThemeDetailDrawer (Hero + Strip)
 *   - #leftNote slot: 二次アクション (edit / export / duplicate / delete)
 *                     または marketplace / system 由来の代替表示
 *   - #actions slot : 閉じる (ghost) + 適用 (primary) / 適用中 (disabled)
 * を差し込む。旧 ThemeDetailDrawerFooter コンポーネントは削除済み。
 */
import type { ThemeCardData } from '~/types/theme'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'

const { t } = useI18n()

const props = defineProps<{
  /** 開く対象のテーマ。null のときはモーダル非表示。 */
  theme: ThemeCardData | null
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon フォールバック。 */
  previewMap: Record<string, string> | null
  /** 役割名 → ホットスポット詳細。ホットスポットドット表示に使う。 */
  previewDetails?: Record<string, RolePreviewDetail> | null
}>()

const emit = defineEmits<{
  close: []
  apply: [id: string]
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
}>()

const isOpen = computed(() => props.theme !== null)

const headerDescription = computed(() => {
  if (!props.theme) return ''
  const datePart = props.theme.date ? ` · ${props.theme.date.slice(0, 10)}` : ''
  return `@${props.theme.author ?? 'unknown'} · v${props.theme.version}${datePart}`
})

const isSystem = computed(() => props.theme?.kind === 'system')
const isMarketplace = computed(() => props.theme?.kind === 'marketplace')
</script>

<template>
  <UiModal
    :open="isOpen"
    :title="theme?.name ?? ''"
    :description="headerDescription"
    icon="Library"
    icon-tone="accent"
    size="xl"
    :body-padded="false"
    :aria-labelledby="theme ? `theme-detail-${theme.id}` : undefined"
    @close="emit('close')"
  >
    <ThemeDetailDrawer
      v-if="theme"
      :theme="theme"
      :preview-map="previewMap"
      :preview-details="previewDetails"
    />

    <!-- 二次アクション (フッター左側) -->
    <template v-if="theme" #leftNote>
      <div class="td-secondary">
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
          <span v-if="isMarketplace" class="td-hint">
            {{ t('themeDetail.cannotEditMarketplace') }}
          </span>
        </template>
        <template v-else>
          <!--
            システムスキームは編集・複製・削除はできないが、`.cursorpack`
            として書き出して別環境へ持ち運ぶことはできる。
          -->
          <button
            class="td-act"
            :aria-label="t('themeDetail.exportSchemeAria', { name: theme.name })"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />{{ t('themeDetail.exportSchemeLabel') }}
          </button>
          <span class="td-source-readonly">
            <UiIcon name="Globe" :size="11" />{{ t('themeDetail.systemSchemeReadOnly') }}
          </span>
        </template>
      </div>
    </template>

    <!-- 主アクション (フッター右側) -->
    <template #actions>
      <UiButton variant="ghost" @click="emit('close')">
        {{ t('common.close') }}
      </UiButton>
      <UiButton v-if="theme && theme.isActive" variant="default" :disabled="true" icon-left="Check">
        {{ t('themeDetail.applyingNow') }}
      </UiButton>
      <UiButton v-else-if="theme" variant="primary" @click="emit('apply', theme.id)">
        {{ t('themeDetail.applyTheme') }}
      </UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* モーダル内の drawer は通常のカード文脈ではないので、外側の境界線を消して二重枠を避ける */
:deep(.td-drawer) {
  background: transparent;
}

/* 二次アクション群 (旧 ThemeDetailDrawerFooter の .td-foot-l 相当)。
 * UiModal の `.modal-foot .left-note` (font-mono / text-fg-mute) の文字サイズと
 * 噛み合わないため、ここで text style を上書きする。 */
.td-secondary {
  @apply flex flex-wrap items-center gap-1.5;
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

.td-hint {
  @apply text-[11px] text-fg-mute;
}

.td-source-readonly {
  @apply inline-flex items-center gap-1.5 font-mono text-[10.5px] tracking-[0.02em] text-fg-mute;
}
</style>
