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
    <div class="chips" role="group" aria-label="フィルター">
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
.filters {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
}

.chips {
  display: flex;
  align-items: center;
  gap: 6px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev1);
  color: var(--text-mute);
  font-size: 12px;
  cursor: pointer;
}

.chip.active {
  background: var(--bg-elev2);
  color: var(--text);
  border-color: var(--accent);
}

.chip .num {
  margin-left: 4px;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--bg-elev2);
  color: var(--text-mute);
  font-size: 10px;
  line-height: 14px;
  min-width: 18px;
  text-align: center;
}

.spacer-x {
  flex: 1;
}

.sort {
  display: flex;
  align-items: center;
  gap: 6px;
}

.sort .lbl {
  font-size: 11px;
  color: var(--text-mute);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
}

.btn.ghost {
  background: transparent;
}
</style>
