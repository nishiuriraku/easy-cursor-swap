<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const emit = defineEmits<{
  (e: 'bulk-files'): void
  (e: 'bulk-folder'): void
  (e: 'bulk-cursorpack'): void
}>()

const open = ref(false)
function close() {
  open.value = false
}

function pick(action: 'files' | 'folder' | 'cursorpack') {
  close()
  if (action === 'files') emit('bulk-files')
  else if (action === 'folder') emit('bulk-folder')
  else emit('bulk-cursorpack')
}
</script>

<template>
  <div class="bi-btn-host" @keydown.esc="close">
    <button class="btn ghost" @click="open = !open">
      <UiIcon name="Import" :size="13" />
      {{ t('bulkImport.dropdownLabel') }}
      <span class="caret">▾</span>
    </button>
    <Transition name="fade">
      <div v-if="open" v-click-outside="close" class="bi-menu">
        <button class="bi-menu-item" @click="pick('files')">
          {{ t('bulkImport.dropdownFiles') }}
        </button>
        <button class="bi-menu-item" @click="pick('folder')">
          {{ t('bulkImport.dropdownFolder') }}
        </button>
        <button class="bi-menu-item" @click="pick('cursorpack')">
          {{ t('bulkImport.dropdownPack') }}
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
.caret {
  font-size: 9px;
  margin-left: 4px;
}
.bi-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  background: var(--bg-1, #14161c);
  border: 1px solid var(--line);
  border-radius: 8px;
  min-width: 220px;
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
