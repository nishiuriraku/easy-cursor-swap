<script setup lang="ts">
/**
 * テーマ詳細ドロワー (モーダル本体)
 *
 * 3 段構成 → 3 子コンポーネントに委譲:
 *   1. ThemeDetailDrawerHero  — DESCRIPTION ペイン + ROLE COVERAGE ペイン (activeRole 状態を内部で保持)
 *   2. ThemeDetailDrawerStrip — PACKAGE / USAGE / SOURCE の 3 セル strip (純粋表示)
 *   3. ThemeDetailDrawerFooter — アクションレール (apply / edit / duplicate / exportPack / delete を emit)
 *
 * 親はグラデ背景の `td-drawer` ラッパーのみ保持し、ロジックは子に分散させる。
 */
import type { ThemeCardData } from '~/types/theme'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'
import ThemeDetailDrawerHero from './ThemeDetailDrawerHero.vue'
import ThemeDetailDrawerStrip from './ThemeDetailDrawerStrip.vue'
import ThemeDetailDrawerFooter from './ThemeDetailDrawerFooter.vue'

const props = defineProps<{
  theme: ThemeCardData
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon のフォールバックを表示。 */
  previewMap: Record<string, string> | null
  /**
   * 役割名 → ホットスポット詳細 (寸法 + ホットスポット座標) のマップ。
   * `previewMap` と組で渡されることを想定。null や未取得ロールはホットスポットドット非表示。
   */
  previewDetails?: Record<string, RolePreviewDetail> | null
}>()

const emit = defineEmits<{
  apply: [id: string]
  close: []
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
  openSource: [id: string]
}>()
</script>

<template>
  <div class="td-drawer">
    <ThemeDetailDrawerHero
      :theme="props.theme"
      :preview-map="props.previewMap"
      :preview-details="props.previewDetails"
    />
    <ThemeDetailDrawerStrip :theme="props.theme" />
    <ThemeDetailDrawerFooter
      :theme="props.theme"
      @apply="emit('apply', $event)"
      @edit="emit('edit', $event)"
      @duplicate="emit('duplicate', $event)"
      @export-pack="emit('exportPack', $event)"
      @delete="emit('delete', $event)"
    />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-drawer {
  @apply flex flex-col;
  background:
    linear-gradient(180deg, rgba(124, 242, 212, 0.025), transparent 40%), rgba(0, 0, 0, 0.18);
}
:where(html.light) .td-drawer {
  background:
    linear-gradient(180deg, rgba(15, 168, 133, 0.04), transparent 40%), rgba(15, 20, 35, 0.02);
}
</style>
