<script setup lang="ts">
/**
 * テーマカード。プレビュー (17 ロールマトリクス) + 名前/作者 + メタ + カバレッジ + アクション。
 * - `apply` クリックで親へ emit (Rust IPC は親で実行)
 * - `toggleFavorite` でスター切替
 * - マウント時に Rust から実カーソル PNG を取得し、マトリクスに反映する
 */
import { computed, onMounted, ref, watch } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'
import { useThemePreviews } from '~/composables/useThemePreviews'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

/**
 * カード本体のクリック/Enter/Space で詳細モーダルを開く。
 * 適用フローは「カード → 詳細モーダル → 適用モーダル」の 2 段階に統一されており、
 * カード自体には適用ボタンを置かない。
 * 子の <button>/<a>/<input> は個別ハンドラに委譲するため stopPropagation する。
 */
function onCardActivate(e: Event) {
  const target = e.target as HTMLElement | null
  if (target?.closest('button, a, input')) return
  emit('showDetails', props.theme.id)
}

function onCardKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    onCardActivate(e)
  }
}

/* 2026-05-14: ライブラリカードはプレビューを 3x2 (6 セル) に縮小したため、
 * .card-preview の min-height (default 132px) を抑えて全体高さを詰める。
 * MarketplaceCard も同日に同じ 3x2 + 112px へ揃えたため、両者で同じ調整が入る。 */
const coveragePct = computed(() => Math.round((props.theme.includedRoles.length / 17) * 100))

const displayDate = computed(() => {
  // ISO8601 が来たら先頭 10 文字 (YYYY-MM-DD) にトリム
  const d = props.theme.date
  return d.length > 10 ? d.slice(0, 10) : d
})

/** Windows のシステムスキーム (HKCU\Cursors\Schemes) は編集・お気に入りを許可しない。 */
const isSystem = computed(() => props.theme.kind === 'system')
const isMarketplace = computed(() => props.theme.kind === 'marketplace')

// 実カーソル画像のプレビュー (キャッシュ越しに取得)
const previewMap = ref<Record<string, string> | null>(null)
const { getMap } = useThemePreviews()

async function fetchPreview() {
  if (!props.theme.id) return
  const map = await getMap(props.theme.id)
  previewMap.value = map
}

onMounted(fetchPreview)
watch(() => props.theme.id, fetchPreview)
</script>

<template>
  <article
    :class="['card', { active: theme.isActive }, 'interactive']"
    :aria-label="t('library.detailAria', { name: theme.name })"
    tabindex="0"
    role="button"
    @click="onCardActivate"
    @keydown="onCardKeydown"
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
          @click="emit('toggleFavorite', theme.id)"
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
