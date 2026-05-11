<script setup lang="ts">
/**
 * 一括インポートのエントリ。スプリットボタン構成。
 *
 *  [ 一括インポート ] [ ▾ ]
 *                          └── 📁 フォルダから取込
 *
 * - 主クリック: png/svg/cur/ico/.cursorpack をまとめて選択するファイルダイアログ。
 *   親側 (creator.vue) で拡張子を見て `.cursorpack` 単独 or 通常一括に分岐する。
 * - サブメニュー: フォルダ取込のみ。ネイティブダイアログがファイル/フォルダ併用不可なため
 *   ここだけ別経路。
 */
import { ref } from 'vue'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const emit = defineEmits<{
  /** 主アクション。creator.vue が拡張子で `.cursorpack` 単独 or bulk-resolve に分岐する。 */
  (e: 'bulk-auto'): void
  /** フォルダ取込。chevron 経由のみ。 */
  (e: 'bulk-folder'): void
}>()

const open = ref(false)
function close() {
  open.value = false
}
function toggle() {
  open.value = !open.value
}
function pickFolder() {
  close()
  emit('bulk-folder')
}
</script>

<template>
  <div class="bi-btn-host" @keydown.esc="close">
    <div class="bi-split">
      <button class="btn ghost bi-primary" @click="emit('bulk-auto')">
        <UiIcon name="Import" :size="13" />
        {{ t('bulkImport.dropdownLabel') }}
      </button>
      <button
        class="btn ghost bi-chevron"
        :aria-label="t('bulkImport.moreOptions')"
        :aria-expanded="open"
        @click="toggle"
      >
        <span class="caret">▾</span>
      </button>
    </div>
    <Transition name="fade">
      <div v-if="open" v-click-outside="close" class="bi-menu">
        <button class="bi-menu-item" @click="pickFolder">
          {{ t('bulkImport.dropdownFolder') }}
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.bi-btn-host {
  @apply relative inline-block;
}
.bi-split {
  @apply inline-flex items-stretch gap-0;
}
.bi-primary {
  @apply rounded-r-none;
}
.bi-chevron {
  @apply rounded-l-none border-l border-line px-1.5;
}
.caret {
  @apply text-[9px];
}
.bi-menu {
  @apply absolute right-0 top-full z-50 mt-1 flex min-w-[200px] flex-col rounded-[8px] border border-line;
  background: var(--bg-1, #14161c);
}
.bi-menu-item {
  @apply cursor-pointer border-0 bg-transparent px-3 py-2 text-left text-[12px] text-fg;
}
.bi-menu-item:hover {
  background: rgba(124, 242, 212, 0.08);
}
</style>
