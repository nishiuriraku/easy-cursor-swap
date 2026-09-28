<script setup lang="ts">
/**
 * 設定画面の検索ボックス (P08a Step 3 / V4)。
 *
 * `settings.vue` のツールバー内検索 UI (input + dropdown) を `section` の
 * v-model とともに切り出したもの。検索ロジックは `useSettingsSearch`。
 */
import type {
  SearchContext,
  SettingsSectionId,
  SettingsSearchEntry,
} from '~/composables/useSettingsSearch'
import { useSettingsSearch } from '~/composables/useSettingsSearch'

const props = defineProps<{
  context: SearchContext
}>()

const section = defineModel<SettingsSectionId>('section', { required: true })

const { t, locale } = useI18n()

const searchQuery = ref('')
// 設定検索 composable (横断検索 → ジャンプ)
// SettingsSearchDropdown は Teleport で body 直下に描画するため、トリガー要素の
// 座標計算用に検索ラッパ div の ref を渡す。
// テンプレ側の ref アンラップに対応するため toRef で包み直す。
const contextRef = toRef(props, 'context')
const searchAnchorRef = ref<HTMLElement | null>(null)
const {
  open: searchOpen,
  activeIndex: searchActiveIndex,
  visibleResults: searchResults,
  overflowCount: searchOverflow,
  focus: openSearchDropdown,
  close: closeSearchDropdown,
  moveActive: moveSearchActive,
  resetActive: resetSearchActive,
  jumpTo: jumpToSearchResult,
} = useSettingsSearch({
  query: searchQuery,
  locale,
  context: contextRef,
  sectionRef: section,
})

function onSearchInput() {
  resetSearchActive()
  searchOpen.value = searchQuery.value.trim().length > 0
}

function onSearchKeydown(ev: KeyboardEvent) {
  if (!searchOpen.value) return
  switch (ev.key) {
    case 'ArrowDown':
      ev.preventDefault()
      moveSearchActive(1)
      break
    case 'ArrowUp':
      ev.preventDefault()
      moveSearchActive(-1)
      break
    case 'Enter': {
      ev.preventDefault()
      const r = searchResults.value[searchActiveIndex.value]
      if (r) {
        searchQuery.value = ''
        closeSearchDropdown()
        void jumpToSearchResult(r.entry)
      }
      break
    }
    case 'Escape':
      ev.preventDefault()
      closeSearchDropdown()
      break
  }
}

function onSearchSelect(entry: SettingsSearchEntry) {
  searchQuery.value = ''
  closeSearchDropdown()
  void jumpToSearchResult(entry)
}

function onSearchHover(i: number) {
  searchActiveIndex.value = i
}

function onSearchBlur() {
  // mousedown 経由の select 後でも安全に閉じる (mousedown 内で .prevent 済)
  setTimeout(() => closeSearchDropdown(), 0)
}
</script>

<template>
  <div ref="searchAnchorRef" class="search" style="max-width: 280px; position: relative">
    <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
    <input
      v-model="searchQuery"
      :placeholder="t('settings.searchPlaceholder')"
      :aria-label="t('common.search')"
      role="combobox"
      :aria-expanded="searchOpen"
      aria-controls="settings-search-listbox"
      :aria-activedescendant="
        searchOpen && searchResults.length > 0
          ? `settings-search-opt-${searchActiveIndex}`
          : undefined
      "
      @input="onSearchInput"
      @focus="openSearchDropdown"
      @keydown="onSearchKeydown"
      @blur="onSearchBlur"
    />
    <SettingsSearchDropdown
      v-if="searchOpen"
      id="settings-search-listbox"
      :anchor-el="searchAnchorRef"
      :results="searchResults"
      :overflow-count="searchOverflow"
      :active-index="searchActiveIndex"
      :query="searchQuery"
      @select="onSearchSelect"
      @hover="onSearchHover"
    />
  </div>
</template>
