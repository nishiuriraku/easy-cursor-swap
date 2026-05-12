<script setup lang="ts">
/**
 * Marketplace のグリッドカード。
 * テーマライブラリの ThemeCard と似ているが、
 *  - 「適用」ではなく「インポート」ボタン
 *  - 署名済みバッジを常時表示
 *  - applyCount ではなく downloadCount
 * という差分がある。
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
      <CursorMatrix :included="entry.includedRoles" />
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
        <span class="m">v{{ entry.version }}</span>
        <span class="m">{{ entry.includedRoles.length }}/17</span>
      </div>
      <div class="coverage">
        <div class="bar"><i :style="{ width: coveragePct + '%' }" /></div>
        <span class="num">{{ coveragePct }}%</span>
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

.card-actions {
  @apply mt-1 flex gap-1.5;
}
.card-actions .btn {
  @apply h-[30px] flex-1 text-[12px];
}
</style>
