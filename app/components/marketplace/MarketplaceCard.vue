<script setup lang="ts">
/**
 * Marketplace のグリッドカード。
 * テーマライブラリの ThemeCard と視覚スタイルを揃えている (2026-05-14):
 *  - プレビューは 3x2 (CURSOR_ROLES 正規順で先頭 6 個) のコンパクトマトリクス
 *  - .card-preview の高さを 112px に詰めて全体高さを統一
 *  - coverage の数値表記は `X/17`
 * ただし機能面の差分は維持する:
 *  - 「適用」ではなく「インポート」ボタンをカード下部に配置
 *  - 署名済みバッジ (verified) を常時表示
 *  - applyCount ではなく downloadCount を meta-row に表示
 *  - Marketplace の MarketplaceEntry には `date` が無いので、meta-row は
 *    「↓ downloads」+ 「v version」の 2 項目構成 (Library は 3 項目)。
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'
import type { MarketplaceEntry } from '~/types/marketplace'

const { t } = useI18n()

const props = defineProps<{
  entry: MarketplaceEntry
}>()

const emit = defineEmits<{
  install: [id: string]
}>()

const coveragePct = computed(() => Math.round((props.entry.includedRoles.length / 17) * 100))

const fmtDownloads = computed(() => props.entry.downloadCount.toLocaleString('ja-JP'))
</script>

<template>
  <div class="card">
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
      <div class="card-actions">
        <button class="btn primary" @click="emit('install', entry.id)">
          <UiIcon name="Import" :size="13" />{{ t('marketplace.importBtn') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* Library の ThemeCard と同じ高さに詰める。
 * CursorMatrix が 3x2 (6 セル) に縮んだので、デフォルトの 132px だと余白が空きすぎる。 */
.card-preview {
  @apply min-h-[112px] p-3;
}

.card-actions {
  @apply mt-1 flex gap-1.5;
}
.card-actions .btn {
  @apply h-[30px] flex-1 text-[12px];
}
</style>
