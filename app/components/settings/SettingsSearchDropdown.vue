<script setup lang="ts">
/**
 * 設定検索結果のドロップダウン。
 *
 * Dumb component: state を持たず、props で受け取った結果を表示し、
 * ユーザー操作を select / hover でホスト (settings.vue) に通知する。
 */
import { useI18n } from '~/composables/useI18n'
import type { SearchResult, SettingsSearchEntry } from '~/composables/useSettingsSearch'

const { t } = useI18n()

defineProps<{
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
</script>

<template>
  <div v-if="query.trim().length > 0" class="search-dd" role="presentation">
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
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* 背景・ボーダー・影はトークン経由でライト/ダーク両対応。
   UiSelect.vue のリストボックスと同じ pattern (--bg-2 → html.light で --bg-1)。 */
.search-dd {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 50;
  max-height: 360px;
  overflow-y: auto;
  background: var(--bg-2);
  border: 1px solid var(--line-hi);
  border-radius: 10px;
  box-shadow:
    0 12px 32px -12px rgba(0, 0, 0, 0.55),
    0 0 0 1px rgba(0, 0, 0, 0.25);
  color: var(--fg);
}
:where(html.light) .search-dd {
  background: var(--bg-1);
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
