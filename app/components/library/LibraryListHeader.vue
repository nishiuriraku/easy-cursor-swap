<script setup lang="ts">
/**
 * Library の一覧ヘッダ行 (P08a Step 4 / V5)。
 *
 * 4 列の `aria-sort` / `tabindex` / Enter・Space をそのまま移したもの。
 * `.lib-row.lib-head/.lt-*` は共有 CSS。
 */
import type { SortKey, SortDir } from '~/pages/index.helpers'

const { t } = useI18n()

defineProps<{
  sortKey: SortKey
  sortDir: SortDir
}>()

defineEmits<{
  (e: 'sort', key: SortKey): void
}>()
</script>

<template>
  <div class="lib-row lib-head" role="row">
    <div class="lt-col lt-fav" role="columnheader" />
    <div class="lt-col lt-preview" role="columnheader">{{ t('library.colPreview') }}</div>
    <div
      :class="['lt-col', 'lt-name', 'lt-sortable', { active: sortKey === 'name' }]"
      role="columnheader"
      :aria-sort="sortKey === 'name' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'"
      tabindex="0"
      @click="$emit('sort', 'name')"
      @keydown.enter.prevent="$emit('sort', 'name')"
      @keydown.space.prevent="$emit('sort', 'name')"
    >
      {{ t('library.colNameAuthor') }}
      <span v-if="sortKey === 'name'" class="sort-dir">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
    </div>
    <div class="lt-col lt-ver" role="columnheader">{{ t('library.colVersion') }}</div>
    <div
      :class="['lt-col', 'lt-date', 'lt-sortable', { active: sortKey === 'updated' }]"
      role="columnheader"
      :aria-sort="sortKey === 'updated' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'"
      tabindex="0"
      @click="$emit('sort', 'updated')"
      @keydown.enter.prevent="$emit('sort', 'updated')"
      @keydown.space.prevent="$emit('sort', 'updated')"
    >
      {{ t('library.colUpdated') }}
      <span v-if="sortKey === 'updated'" class="sort-dir">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
    </div>
    <div
      :class="['lt-col', 'lt-size', 'lt-sortable', { active: sortKey === 'size' }]"
      role="columnheader"
      :aria-sort="sortKey === 'size' ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'"
      tabindex="0"
      @click="$emit('sort', 'size')"
      @keydown.enter.prevent="$emit('sort', 'size')"
      @keydown.space.prevent="$emit('sort', 'size')"
    >
      {{ t('library.colSize') }}
      <span v-if="sortKey === 'size'" class="sort-dir">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
    </div>
  </div>
</template>
