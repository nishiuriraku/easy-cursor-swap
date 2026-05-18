<script setup lang="ts">
/**
 * テーマカード。プレビュー (17 ロールマトリクス) + 名前/作者 + メタ + カバレッジ + アクション。
 * - 詳細モーダルへの遷移とお気に入りトグルは `useThemeCardState` に共通化済み。
 * - カード本体クリック/Enter/Space で詳細モーダルを開く (内側 button は stopPropagation)。
 */
import type { ThemeCardData } from '~/types/theme'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

const { previewMap, isSystem, isMarketplace, displayDate, onActivate, onKeydown, onFavorite } =
  useThemeCardState(toRef(props, 'theme'), {
    showDetails: (id) => emit('showDetails', id),
    toggleFavorite: (id) => emit('toggleFavorite', id),
  })

/* 2026-05-14: ライブラリカードはプレビューを 3x2 (6 セル) に縮小したため、
 * .card-preview の min-height (default 132px) を抑えて全体高さを詰める。
 * MarketplaceCard も同日に同じ 3x2 + 112px へ揃えたため、両者で同じ調整が入る。 */
const coveragePct = computed(() => Math.round((props.theme.includedRoles.length / 17) * 100))
</script>

<template>
  <article
    :class="['card', { active: theme.isActive }, 'interactive']"
    :aria-label="t('library.detailAria', { name: theme.name })"
    tabindex="0"
    role="button"
    @click="onActivate"
    @keydown="onKeydown"
  >
    <div class="card-preview">
      <div v-if="theme.isActive" class="card-active-tag">
        <span class="pulse" aria-hidden="true" />{{ t('library.activeTag') }}
      </div>
      <div v-if="isSystem" class="card-source-tag" :aria-label="t('library.sourceTagSchemeAria')">
        SYSTEM
      </div>
      <div
        v-else-if="isMarketplace"
        class="card-source-tag marketplace"
        :aria-label="t('library.sourceTagMarketplaceAria')"
      >
        MARKETPLACE
      </div>
      <CursorMatrix
        :included="theme.includedRoles"
        :preview-map="previewMap"
        :limit="6"
        :cols="3"
        :aria-label="t('library.coverage', { filled: theme.includedRoles.length })"
      />
    </div>
    <div class="card-body">
      <div class="card-row">
        <div>
          <div class="card-name">{{ theme.name }}</div>
          <div class="card-author">@{{ theme.author ?? 'unknown' }}</div>
        </div>
        <button
          v-if="!isSystem"
          :class="['star', { on: theme.isFavorite }]"
          :aria-label="theme.isFavorite ? t('library.favRemove') : t('library.favAdd')"
          :aria-pressed="theme.isFavorite"
          @click="onFavorite"
        >
          <UiIcon :name="theme.isFavorite ? 'Star' : 'StarO'" :size="13" aria-hidden="true" />
        </button>
      </div>
      <div
        v-if="!isSystem"
        class="meta-row"
        aria-label="`${theme.name} v${theme.version}, ${displayDate}`"
      >
        <span v-if="theme.signed" class="tag ok featured-tag">
          <UiIcon name="Shield" :size="9" />
        </span>
        <span class="m" aria-hidden="true"><b>v</b>{{ theme.version }}</span>
        <span class="m" aria-hidden="true">{{ displayDate }}</span>
        <span class="m" aria-hidden="true">×{{ theme.applyCount }}</span>
      </div>
      <div
        class="coverage"
        :aria-label="t('library.coverageAria', { filled: theme.includedRoles.length })"
      >
        <div class="bar" aria-hidden="true"><i :style="{ width: coveragePct + '%' }" /></div>
        <span class="num" aria-hidden="true">{{ theme.includedRoles.length }}/17</span>
      </div>
    </div>
  </article>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* ライブラリカード固有: matrix が 3x2 に縮んだぶん preview を詰めて全体高さを軽く。
 * Marketplace は別ファイル (.card-preview グローバル既定 132px) のまま。 */
.card-preview {
  @apply min-h-[112px] p-3;
}

.card-source-tag.marketplace {
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.16), rgba(124, 242, 212, 0.04));
  color: var(--accent, #7cf2d4);
  border-color: rgba(124, 242, 212, 0.35);
}
</style>
