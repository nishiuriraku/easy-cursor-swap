<script setup lang="ts">
/**
 * Marketplace のグリッドカード (全エントリ共通)。
 * 横並びレイアウト + ハイライトラベル + 詳細モーダルを開くボタン。
 *
 * 2026-05-14: Featured ストリップ廃止に伴い、Featured/通常の区別なく
 * `pages/marketplace.vue` の `.grid` 内で全エントリの描画に使う。
 * クリック / Enter / Space で showDetails を emit し、MarketplaceDetailModal
 * が「ライブラリに追加」フローを担う。
 *
 * 2026-05-15: サムネを `previewBaseUrl` の Arrow.png に切り替え。
 * 取得は useMarketplacePreviews のキャッシュ越しなので、後で DetailModal を
 * 開いたときは即座に 6 ロール分の preview がヒットする。
 */
import type { MarketplaceEntry } from '~/types/marketplace'

const { t, locale } = useI18n()

const props = defineProps<{
  entry: MarketplaceEntry
}>()

// index.json の name は LocalizedString (string | { ja, en, default, ... }) で来る。
// 現在の locale でピックして表示する。locale を切替えると computed が再評価されるので
// 「設定で日本語 ↔ 英語を切替えるとカードの名前も追従する」が成り立つ。
const displayName = computed(() => pickLocalizedName(props.entry.name, locale.value))

const arrowPreviewUrl = ref<string | null>(null)
const { getMap } = useMarketplacePreviews()

async function fetchArrowPreview() {
  arrowPreviewUrl.value = null
  if (!props.entry.previewBaseUrl) return
  try {
    const map = await getMap(props.entry.id, props.entry.previewBaseUrl)
    arrowPreviewUrl.value = map.Arrow ?? null
  } catch (err) {
    // 画像が無い場合は SVG フォールバックに任せる
    console.warn('[FeaturedCard] preview fetch failed:', err)
  }
}

onMounted(fetchArrowPreview)
watch(() => props.entry.id, fetchArrowPreview)

const emit = defineEmits<{
  showDetails: [id: string]
}>()

function highlightLabel(h: MarketplaceEntry['highlight']): string {
  if (h === 'new') return t('marketplace.featuredNew')
  if (h === 'popular') return t('marketplace.featuredPopular')
  return ''
}

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
    class="card featured-card interactive"
    tabindex="0"
    role="button"
    :aria-label="t('marketplace.openMarketplaceDetailAria', { name: displayName })"
    @click="onCardActivate"
    @keydown="onCardKeydown"
  >
    <div class="featured-thumb">
      <img
        v-if="arrowPreviewUrl"
        :src="arrowPreviewUrl"
        alt=""
        class="featured-thumb-img"
        draggable="false"
      />
      <CursorIcon v-else role="Arrow" :size="28" style="color: var(--fg)" />
    </div>
    <div class="featured-body">
      <div class="featured-row">
        <div class="card-name">{{ displayName }}</div>
      </div>
      <div class="card-author">@{{ entry.author }}</div>
      <div class="meta-row featured-meta">
        <span v-if="entry.verified" class="tag ok featured-tag">
          <UiIcon name="Shield" :size="9" />
        </span>
        <span v-if="entry.highlight" class="m" style="color: var(--accent)">
          {{ highlightLabel(entry.highlight) }}
        </span>
      </div>
    </div>
  </article>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.featured-card {
  @apply flex-row items-center gap-3.5 p-4;
}

.featured-thumb {
  @apply grid size-14 shrink-0 place-items-center overflow-hidden rounded-[10px] border border-line-hi;
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.2), rgba(139, 125, 255, 0.2));
}

.featured-thumb-img {
  @apply size-9 object-contain;
  image-rendering: pixelated;
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
