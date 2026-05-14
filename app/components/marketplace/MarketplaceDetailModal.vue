<script setup lang="ts">
/**
 * 公式インデックスエントリの詳細モーダル。
 *
 * Library の ThemeDetailModal と類似:
 *  - Teleport + バックドロップ + Esc/外側クリックで閉じる
 *  - スクロールロック (body overflow: hidden)
 * 違い:
 *  - マウント時に useMarketplacePreviews で 6 ロール PNG を並列取得
 *  - フッター中央に「ライブラリに追加」プライマリボタン (インストール済みなら disabled)
 */
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import type { MarketplaceEntry } from '~/types/marketplace'
import { useI18n } from '~/composables/useI18n'
import { useMarketplacePreviews } from '~/composables/useMarketplacePreviews'
import { useThemes } from '~/composables/useThemes'

const { t } = useI18n()

const props = defineProps<{
  entry: MarketplaceEntry | null
  installing: boolean
}>()

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

const sha256Short = computed(() => {
  const s = props.entry?.sha256 ?? ''
  return s.length > 16 ? `${s.slice(0, 16)}…` : s
})

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

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
  }
}

let prevOverflow: string | null = null
watch(isOpen, (open) => {
  if (typeof document === 'undefined') return
  if (open) {
    prevOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    document.addEventListener('keydown', onKeydown)
  } else {
    if (prevOverflow !== null) {
      document.body.style.overflow = prevOverflow
      prevOverflow = null
    }
    document.removeEventListener('keydown', onKeydown)
  }
})

onBeforeUnmount(() => {
  if (typeof document === 'undefined') return
  if (prevOverflow !== null) document.body.style.overflow = prevOverflow
  document.removeEventListener('keydown', onKeydown)
})

function onInstall() {
  if (!props.entry) return
  if (alreadyInstalled.value || props.installing) return
  emit('install', props.entry.id)
}
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="isOpen && entry"
        class="md-backdrop"
        role="dialog"
        aria-modal="true"
        :aria-label="t('marketplace.openMarketplaceDetailAria', { name: entry.name })"
        @click.self="close"
      >
        <div class="md-shell" @click.stop>
          <div class="md-head">
            <div>
              <div class="md-eyebrow">{{ t('marketplace.detailEyebrow') }}</div>
              <h2>{{ entry.name }}</h2>
              <div class="md-sub">
                @{{ entry.author }} · v{{ entry.version }}
                <span v-if="entry.verified" class="tag ok md-verified">
                  <UiIcon name="Shield" :size="10" />
                </span>
              </div>
            </div>
            <button
              class="btn icon"
              :aria-label="t('common.close')"
              :title="`${t('common.close')} (Esc)`"
              @click="close"
            >
              <UiIcon name="X" :size="13" />
            </button>
          </div>

          <div class="md-body">
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
              <div class="md-row">
                <span class="md-k">{{ t('marketplace.downloads') }}</span>
                <span class="md-v">↓ {{ entry.downloadCount.toLocaleString('ja-JP') }}</span>
              </div>
              <div v-if="entry.tags.length" class="md-row">
                <span class="md-k">Tags</span>
                <span class="md-v chips-row">
                  <span v-for="tag in entry.tags" :key="tag" class="chip">{{ tag }}</span>
                </span>
              </div>
              <div class="md-row">
                <span class="md-k">SHA-256</span>
                <span class="md-v mono">{{ sha256Short }}</span>
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
          </div>

          <footer class="md-foot">
            <button class="btn ghost" @click="close">{{ t('common.cancel') }}</button>
            <button
              class="btn primary"
              :disabled="alreadyInstalled || installing"
              @click="onInstall"
            >
              <UiIcon :name="alreadyInstalled ? 'Check' : 'Import'" :size="13" aria-hidden="true" />
              <span v-if="alreadyInstalled">{{ t('marketplace.alreadyInstalled') }}</span>
              <span v-else-if="installing">{{ t('marketplace.installing') }}</span>
              <span v-else>{{ t('marketplace.addToLibrary') }}</span>
            </button>
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.md-backdrop {
  @apply fixed inset-0 z-[100] grid place-items-center bg-[rgba(8,9,14,0.6)] p-8 backdrop-blur-[8px];
}
.md-shell {
  @apply flex h-auto max-h-[calc(100vh-64px)] w-[min(720px,100%)] flex-col overflow-hidden rounded-[14px] border border-line-hi bg-bg-1;
  box-shadow: var(--shadow-2);
}
.md-head {
  @apply flex items-start justify-between gap-3 border-b border-line px-[22px] pb-4 pt-[18px];
}
.md-eyebrow {
  @apply mb-1 font-mono text-[9.5px] uppercase tracking-[0.16em] text-accent;
}
.md-head h2 {
  @apply m-0 font-display text-[20px] font-semibold tracking-[-0.02em];
}
.md-sub {
  @apply mt-1 inline-flex items-center gap-2 font-mono text-[12px] tracking-[0.02em] text-fg-dim;
}
.md-verified {
  padding: 2px 6px;
}
.md-body {
  @apply min-h-0 flex-1 overflow-y-auto p-[22px];
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
.md-v.mono {
  @apply font-mono text-[12px];
}
.md-v.link {
  @apply text-accent underline;
}
.chips-row {
  @apply inline-flex flex-wrap gap-1;
}
.md-foot {
  @apply flex items-center justify-end gap-2 border-t border-line px-[22px] py-3;
}
.md-foot .btn {
  @apply min-w-[120px];
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
