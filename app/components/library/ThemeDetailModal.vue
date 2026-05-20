<script setup lang="ts">
/**
 * テーマ詳細モーダル
 *
 * カードのインライン展開だと開閉トグルが分かりにくく、複数同時展開で
 * グリッドが乱れる UX 問題があったため、シェブロン押下で中央オーバーレイ
 * のモーダルに切り替えた。
 *
 * 共通 `<UiModal>` シェル (Teleport + バックドロップ + Esc + スクロールロック +
 * focus trap) にラップを委譲し、ここでは body / footer に詳細表示と閉じる
 * ボタンだけを差し込む。
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
      @apply="(id) => emit('apply', id)"
      @edit="(id) => emit('edit', id)"
      @duplicate="(id) => emit('duplicate', id)"
      @export-pack="(id) => emit('exportPack', id)"
      @delete="(id) => emit('delete', id)"
      @close="emit('close')"
    />

    <template #actions>
      <UiButton variant="ghost" @click="emit('close')">
        {{ t('common.close') }}
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
</style>
