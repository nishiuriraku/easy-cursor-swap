<script setup lang="ts">
/**
 * Marketplace の Featured ストリップ用カード。
 * 横並びレイアウト + ハイライトラベル + ダウンロードボタン。
 */
import type { MarketplaceEntry } from '~/types/marketplace'

defineProps<{
  entry: MarketplaceEntry
}>()

const emit = defineEmits<{
  install: [id: string]
}>()

function fmtNumber(n: number): string {
  return n.toLocaleString('ja-JP')
}

function highlightLabel(h: MarketplaceEntry['highlight']): string {
  if (h === 'new') return '新着'
  if (h === 'popular') return '人気'
  return ''
}
</script>

<template>
  <div class="card featured-card">
    <div class="featured-thumb">
      <CursorIcon role="Arrow" :size="28" style="color: var(--fg)" />
    </div>
    <div class="featured-body">
      <div class="featured-row">
        <div class="card-name">{{ entry.name }}</div>
        <span v-if="entry.verified" class="tag ok featured-tag">
          <UiIcon name="Shield" :size="9" />
        </span>
      </div>
      <div class="card-author">@{{ entry.author }}</div>
      <div class="meta-row featured-meta">
        <span class="m">↓ {{ fmtNumber(entry.downloadCount) }}</span>
        <span v-if="entry.highlight" class="m" style="color: var(--accent)">
          {{ highlightLabel(entry.highlight) }}
        </span>
      </div>
    </div>
    <button
      class="btn"
      :aria-label="`${entry.name} をインストール`"
      @click="emit('install', entry.id)"
    >
      <UiIcon name="Import" :size="13" />
    </button>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.featured-card {
  @apply flex-row items-center gap-3.5 p-4;
}

.featured-thumb {
  @apply grid size-14 shrink-0 place-items-center rounded-[10px] border border-line-hi;
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.2), rgba(139, 125, 255, 0.2));
}

.featured-body {
  @apply min-w-0 flex-1;
}

.featured-row {
  @apply flex items-center gap-2;
}

.featured-tag {
  @apply px-[5px] py-px text-[9px];
}

.featured-meta {
  @apply mt-1.5;
}
</style>
