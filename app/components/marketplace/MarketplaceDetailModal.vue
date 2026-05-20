<script setup lang="ts">
/**
 * 公式インデックスエントリの詳細モーダル。
 *
 * `UiModal` で骨格 (Teleport / focus trap / body scroll lock / Esc) を共通化し、
 * - マウント時に useMarketplacePreviews で 6 ロール PNG を並列取得
 * - フッターに「ライブラリに追加」プライマリボタン (インストール済みなら disabled)
 * を載せる構成。
 */
import type { MarketplaceEntry } from '~/types/marketplace'

const { t, locale } = useI18n()

const props = defineProps<{
  entry: MarketplaceEntry | null
  installing: boolean
}>()

// MarketplaceEntry.name は LocalizedString (string | { ja, en, default, ... })。
// 現 locale でピックして見出し / aria-label に流す。
// entry は null のことがあるので空文字列でガード。
const displayName = computed(() =>
  props.entry ? pickLocalizedName(props.entry.name, locale.value) : '',
)

const emit = defineEmits<{
  close: []
  install: [id: string]
}>()

const isOpen = computed(() => props.entry !== null)
const previewMap = ref<Record<string, string> | null>(null)
const { getMap } = useMarketplacePreviews()
const { themes } = useThemes()

const alreadyInstalled = computed(() => {
  const id = props.entry?.id
  if (!id) return false
  return themes.value.some((t) => t.id === id)
})

const coveragePct = computed(() => {
  const n = props.entry?.includedRoles.length ?? 0
  return Math.round((n / 17) * 100)
})

const subtitle = computed(() => {
  if (!props.entry) return ''
  return `@${props.entry.author} · v${props.entry.version}`
})

const ariaLabel = computed(() =>
  t('marketplace.openMarketplaceDetailAria', { name: displayName.value }),
)

async function fetchPreviews(entry: MarketplaceEntry | null) {
  previewMap.value = null
  if (!entry || !entry.previewBaseUrl) return
  const map = await getMap(entry.id, entry.previewBaseUrl)
  previewMap.value = map
}

watch(() => props.entry, fetchPreviews, { immediate: true })

function close() {
  emit('close')
}

function onInstall() {
  if (!props.entry) return
  if (alreadyInstalled.value || props.installing) return
  emit('install', props.entry.id)
}
</script>

<template>
  <UiModal
    :open="isOpen"
    :title="displayName"
    :description="subtitle"
    size="lg"
    @close="close"
  >
    <template v-if="entry" #headExtra>
      <span v-if="entry.verified" class="tag ok md-verified" :aria-label="ariaLabel">
        <UiIcon name="Shield" :size="10" />
      </span>
    </template>

    <template v-if="entry">
      <div class="md-eyebrow">{{ t('marketplace.detailEyebrow') }}</div>

      <div class="md-preview">
        <CursorMatrix
          :included="entry.includedRoles"
          :preview-map="previewMap"
          :limit="6"
          :cols="3"
        />
      </div>

      <div class="md-meta">
        <div class="coverage">
          <div class="bar" aria-hidden="true">
            <i :style="{ width: coveragePct + '%' }" />
          </div>
          <span class="num">{{ entry.includedRoles.length }}/17</span>
        </div>
        <div v-if="entry.tags.length" class="md-row">
          <span class="md-k">Tags</span>
          <span class="md-v chips-row">
            <span v-for="tag in entry.tags" :key="tag" class="chip">{{ tag }}</span>
          </span>
        </div>
        <div v-if="entry.homepage" class="md-row">
          <span class="md-k">Homepage</span>
          <a
            :href="entry.homepage"
            target="_blank"
            rel="noopener noreferrer"
            class="md-v link"
            >{{ entry.homepage }}</a
          >
        </div>
      </div>
    </template>

    <template #actions>
      <UiButton variant="ghost" @click="close">{{ t('common.cancel') }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="alreadyInstalled || installing"
        :icon-left="alreadyInstalled ? 'Check' : 'Import'"
        @click="onInstall"
      >
        <span v-if="alreadyInstalled">{{ t('marketplace.alreadyInstalled') }}</span>
        <span v-else-if="installing">{{ t('marketplace.installing') }}</span>
        <span v-else>{{ t('marketplace.addToLibrary') }}</span>
      </UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.md-eyebrow {
  @apply mb-3 font-mono text-[9.5px] uppercase tracking-[0.16em] text-accent;
}
.md-verified {
  padding: 2px 6px;
}
.md-preview {
  @apply mb-4 grid place-items-center rounded-[10px] border border-line p-4;
  background: rgba(255, 255, 255, 0.02);
}
.md-meta {
  @apply flex flex-col gap-3;
}
.md-row {
  @apply flex items-center justify-between gap-3 text-[13px];
}
.md-k {
  @apply font-mono text-[11px] uppercase tracking-[0.08em] text-fg-mute;
}
.md-v {
  @apply text-fg-dim;
}
.md-v.link {
  @apply text-accent underline;
}
.chips-row {
  @apply inline-flex flex-wrap gap-1;
}
</style>
