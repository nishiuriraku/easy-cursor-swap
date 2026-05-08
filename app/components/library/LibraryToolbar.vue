<script setup lang="ts">
/**
 * Library 画面のヘッダー (パンくず + 検索ボックス + Import/New ボタン)。
 *
 * 検索クエリは v-model で双方向バインディング、Import ボタンは emit で親に通知する。
 * New ボタンは `/creator` ページへの NuxtLink なので emit 不要。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const searchQuery = defineModel<string>('searchQuery', { required: true })

defineEmits<{
  (e: 'open-import'): void
}>()
</script>

<template>
  <div class="toolbar">
    <div class="bcrumb">
      <span class="crumb">{{ t('library.breadcrumbWorkspace') }}</span>
      <span class="sep">/</span>
      <span class="crumb active">{{ t('library.title') }}</span>
    </div>
    <div class="search">
      <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
      <input
        v-model="searchQuery"
        :placeholder="t('library.searchPlaceholder')"
        :aria-label="t('common.search')"
      />
      <span class="kbd">⌘K</span>
    </div>
    <div class="tb-actions">
      <button class="btn ghost" @click="$emit('open-import')">
        <UiIcon name="Import" :size="14" />{{ t('common.import') }}
      </button>
      <NuxtLink class="btn primary" to="/creator">
        <UiIcon name="Plus" :size="14" />{{ t('library.new') }}
      </NuxtLink>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-elev1);
}

.bcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-mute);
}

.crumb.active {
  color: var(--text);
  font-weight: 600;
}

.sep {
  color: var(--text-mute);
}

.search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  max-width: 480px;
  width: 100%;
  margin: 0 auto;
}

.search input {
  flex: 1;
  border: 0;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  outline: none;
}

.kbd {
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-elev1);
  color: var(--text-mute);
}

.tb-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  text-decoration: none;
}

.btn.ghost {
  background: transparent;
}

.btn.primary {
  background: var(--accent);
  color: var(--bg-base);
  border-color: var(--accent);
}
</style>
