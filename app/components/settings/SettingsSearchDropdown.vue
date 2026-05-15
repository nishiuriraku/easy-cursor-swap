<script setup lang="ts">
/**
 * 設定検索結果のドロップダウン。
 *
 * Dumb component: state を持たず、props で受け取った結果を表示し、
 * ユーザー操作を select / hover でホスト (settings.vue) に通知する。
 *
 * 描画は `<Teleport to="body">` + `position: fixed` で行う。理由:
 *  - `.toolbar` が `backdrop-filter: blur(8px)` を持つため、子要素として
 *    `position: absolute` で配置すると stacking-context の影響で背景が
 *    透けて見えるケースがある (UiSelect.vue と同じ問題)。
 *  - body 直下にレンダリングしておけば祖先の `overflow` や stacking-context
 *    に左右されず、必ず最前面に不透明な背景で描画される。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from '~/composables/useI18n'
import type { SearchResult, SettingsSearchEntry } from '~/composables/useSettingsSearch'

const { t } = useI18n()

const props = defineProps<{
  /** トリガー (検索 input ラッパ) の要素。位置計算の基準にする */
  anchorEl: HTMLElement | null
  /** カタログから絞り込んだ可視結果 (最大 N 件) */
  results: SearchResult[]
  /** 表示限界を超えた件数 (> 0 のとき末尾フッタに表示) */
  overflowCount: number
  /** ハイライト対象 (visibleResults 上の index) */
  activeIndex: number
  /** 現在のクエリ。空なら何も描画しない */
  query: string
}>()

defineEmits<{
  (e: 'select', entry: SettingsSearchEntry): void
  (e: 'hover', index: number): void
}>()

const ddStyle = ref<Record<string, string>>({})

function updatePosition() {
  const el = props.anchorEl
  if (!el) return
  const rect = el.getBoundingClientRect()
  ddStyle.value = {
    position: 'fixed',
    top: `${Math.round(rect.bottom + 4)}px`,
    left: `${Math.round(rect.left)}px`,
    width: `${Math.round(rect.width)}px`,
    zIndex: '1000',
  }
}

function onWinChange() {
  if (props.query.trim().length > 0) updatePosition()
}

watch(
  () => props.anchorEl,
  () => updatePosition(),
  { immediate: true },
)
watch(
  () => props.query,
  (q) => {
    if (q.trim().length > 0) updatePosition()
  },
)

onMounted(() => {
  updatePosition()
  window.addEventListener('scroll', onWinChange, { passive: true, capture: true })
  window.addEventListener('resize', onWinChange, { passive: true })
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', onWinChange, { capture: true } as EventListenerOptions)
  window.removeEventListener('resize', onWinChange)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="query.trim().length > 0" class="search-dd" :style="ddStyle" role="presentation">
      <div v-if="results.length === 0" class="dd-empty">
        {{ t('settings.searchNoResult') }}
      </div>

      <ul v-else role="listbox" class="dd-list">
        <li
          v-for="(r, i) in results"
          :key="`${r.entry.section}:${r.entry.anchor}`"
          :id="`settings-search-opt-${i}`"
          role="option"
          :aria-selected="i === activeIndex ? 'true' : 'false'"
          :class="['dd-item', { active: i === activeIndex }]"
          @mousedown.prevent="$emit('select', r.entry)"
          @mouseenter="$emit('hover', i)"
        >
          <div class="dd-bcrumb">
            <span class="dd-sec">{{ r.displaySectionLabel }}</span>
            <span class="dd-sep">›</span>
            <span class="dd-label">{{ r.displayLabel }}</span>
          </div>
        </li>

        <li v-if="overflowCount > 0" class="dd-overflow" aria-hidden="true">
          {{ t('settings.searchMoreResults', { count: overflowCount }) }}
        </li>
      </ul>
    </div>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* Teleport で body 直下にレンダリングするので親 stacking-context の影響を
   受けない。`background-color` は不透明な solid color にし、token 解決失敗時の
   fallback を含めることで透過事故を防ぐ。 */
.search-dd {
  max-height: 360px;
  overflow-y: auto;
  background-color: #161922; /* fallback */
  background-color: var(--bg-2);
  border: 1px solid var(--line-hi);
  border-radius: 10px;
  box-shadow:
    0 12px 32px -12px rgba(0, 0, 0, 0.55),
    0 0 0 1px rgba(0, 0, 0, 0.25);
  color: var(--fg);
  isolation: isolate;
}
:where(html.light) .search-dd {
  background-color: #ffffff;
  background-color: var(--bg-1);
  box-shadow:
    0 12px 32px -12px rgba(15, 20, 35, 0.18),
    0 0 0 1px rgba(15, 20, 35, 0.08);
}
.dd-empty {
  @apply px-3 py-2.5 text-[12px] text-fg-mute;
}
.dd-list {
  @apply m-0 list-none p-1;
}
.dd-item {
  @apply cursor-pointer rounded-md px-2.5 py-2 text-[12.5px];
}
.dd-item.active,
.dd-item:hover {
  background: var(--accent-dim);
  color: var(--accent);
}
.dd-bcrumb {
  @apply flex items-baseline gap-1.5;
}
.dd-sec {
  @apply text-[11px] text-fg-mute;
}
.dd-sep {
  @apply text-fg-mute;
}
.dd-label {
  @apply font-medium;
}
.dd-overflow {
  @apply px-2.5 py-1.5 font-mono text-[10.5px] text-fg-mute;
}
</style>
