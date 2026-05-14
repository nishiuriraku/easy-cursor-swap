<script setup lang="ts">
/**
 * Marketplace のグリッドカード。
 * ThemeCard と視覚スタイルを揃えている: 3x2 マトリクス, 112px プレビュー高さ。
 *
 * 2026-05-14: カードクリックで詳細モーダルを開く方式に変更。Import ボタンは廃止し、
 * 「ライブラリに追加」フローは MarketplaceDetailModal の中で行う。
 * 子の <button>/<a>/<input> は個別ハンドラに委譲するため stopPropagation する。
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'
import type { MarketplaceEntry } from '~/types/marketplace'

const { t } = useI18n()

const props = defineProps<{
  entry: MarketplaceEntry
}>()

const emit = defineEmits<{
  showDetails: [id: string]
}>()

const coveragePct = computed(() => Math.round((props.entry.includedRoles.length / 17) * 100))
const fmtDownloads = computed(() => props.entry.downloadCount.toLocaleString('ja-JP'))

function onCardActivate(e: Event) {
  const target = e.target as HTMLElement | null
  if (target?.closest('button, a, input')) return
  emit('showDetails', props.entry.id)
}

function onCardKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    onCardActivate(e)
  }
}
</script>

<template>
  <article
    class="card interactive"
    tabindex="0"
    role="button"
    :aria-label="t('marketplace.openMarketplaceDetailAria', { name: entry.name })"
    @click="onCardActivate"
    @keydown="onCardKeydown"
  >
    <div class="card-preview">
      <CursorMatrix :included="entry.includedRoles" :limit="6" :cols="3" />
    </div>
    <div class="card-body">
      <div class="card-row">
        <div>
          <div class="card-name">{{ entry.name }}</div>
          <div class="card-author">@{{ entry.author }}</div>
        </div>
        <span v-if="entry.verified" class="tag ok" style="padding: 2px 6px">
          <UiIcon name="Shield" :size="10" />
        </span>
      </div>
      <div class="meta-row">
        <span class="m">↓ {{ fmtDownloads }}</span>
        <span class="m"><b>v</b>{{ entry.version }}</span>
      </div>
      <div class="coverage">
        <div class="bar" aria-hidden="true"><i :style="{ width: coveragePct + '%' }" /></div>
        <span class="num" aria-hidden="true">{{ entry.includedRoles.length }}/17</span>
      </div>
    </div>
  </article>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.card-preview {
  @apply min-h-[112px] p-3;
}
</style>
