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
.bi-btn-host {
  position: relative;
  display: inline-block;
}
.bi-split {
  display: inline-flex;
  align-items: stretch;
  gap: 0;
}
.bi-primary {
  border-top-right-radius: 0;
  border-bottom-right-radius: 0;
}
.bi-chevron {
  border-top-left-radius: 0;
  border-bottom-left-radius: 0;
  border-left: 1px solid var(--line);
  padding-left: 6px;
  padding-right: 6px;
}
.caret {
  font-size: 9px;
}
.bi-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  background: var(--bg-1, #14161c);
  border: 1px solid var(--line);
  border-radius: 8px;
  min-width: 200px;
  z-index: 50;
  display: flex;
  flex-direction: column;
}
.bi-menu-item {
  background: transparent;
  border: 0;
  padding: 8px 12px;
  text-align: left;
  font-size: 12px;
  color: var(--fg);
  cursor: pointer;
}
.bi-menu-item:hover {
  background: rgba(124, 242, 212, 0.08);
}
</style>
