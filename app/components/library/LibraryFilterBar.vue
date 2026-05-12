<script setup lang="ts">
/**
 * Library 画面のフィルタチップ + ソートボタン。
 *
 * 4 つのフィルタ (all / favorites / recent / unsigned) と件数バッジ、
 * 右端の ソート切替ボタン (cycleSort で 3 状態を循環) で構成される。
 * カウントは親が computed で渡し、ソートラベルは sortLabel ですでに整形済みのものを props で受け取る。
 */
import { useI18n } from '~/composables/useI18n'

type FilterId = 'all' | 'favorites' | 'recent' | 'unsigned'

const { t } = useI18n()

const filter = defineModel<FilterId>('filter', { required: true })

defineProps<{
  counts: { all: number; favorites: number; recent: number; unsigned: number }
  sortLabel: string
}>()

defineEmits<{
  (e: 'cycle-sort'): void
}>()
</script>

<template>
  <div class="filters" role="group" :aria-label="t('common.search')">
    <div class="chips" role="group" :aria-label="t('library.filterGroupAria')">
      <button
        :class="['chip', { active: filter === 'all' }]"
        :aria-pressed="filter === 'all'"
        @click="filter = 'all'"
      >
        {{ t('library.filterAll') }}<span class="num" aria-hidden="true">{{ counts.all }}</span>
      </button>
      <button
        :class="['chip', { active: filter === 'favorites' }]"
        :aria-pressed="filter === 'favorites'"
        @click="filter = 'favorites'"
      >
        <UiIcon name="Star" :size="11" aria-hidden="true" />{{ t('library.filterFavorites')
        }}<span class="num" aria-hidden="true">{{ counts.favorites }}</span>
      </button>
      <button
        :class="['chip', { active: filter === 'recent' }]"
        :aria-pressed="filter === 'recent'"
        @click="filter = 'recent'"
      >
        {{ t('library.filterRecent')
        }}<span class="num" aria-hidden="true">{{ counts.recent }}</span>
      </button>
      <button
        :class="['chip', { active: filter === 'unsigned' }]"
        :aria-pressed="filter === 'unsigned'"
        @click="filter = 'unsigned'"
      >
        {{ t('library.filterUnsigned')
        }}<span class="num" aria-hidden="true">{{ counts.unsigned }}</span>
      </button>
    </div>
    <div class="spacer-x" />
    <div class="sort">
      <span class="lbl" aria-hidden="true">{{ t('library.sort') }}</span>
      <button
        class="btn ghost"
        style="height: 28px"
        :aria-label="`${t('library.sort')}: ${sortLabel}`"
        @click="$emit('cycle-sort')"
      >
        <UiIcon name="Sort" :size="13" aria-hidden="true" />{{ sortLabel }}
        <UiIcon name="ChevD" :size="11" aria-hidden="true" />
      </button>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.filters {
  @apply mb-[22px] flex items-center gap-3 border-b border-line pb-4;
}
.spacer-x {
  @apply flex-1;
}
.sort {
  @apply flex items-center gap-2 text-[12px] text-fg-dim;
}
.sort .lbl {
  @apply font-mono text-[10px] uppercase tracking-[0.12em] text-fg-mute;
}
</style>
