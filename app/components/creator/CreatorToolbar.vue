<script setup lang="ts">
/**
 * Creator のツールバー (パンくず + クリアボタン + 署名状態タグ + Export/Build ボタン)。
 *
 * メタデータ表示 (テーマ名 / バージョン) と build/export 状態を props で受け取り、
 * アクション 3 種 (clear / export / build) を emit で親に通知する。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

defineProps<{
  metaName: string
  metaVersion: string
  hasKeystoreSigning: boolean
  exportBusy: boolean
  buildBusy: boolean
  arrowAssigned: boolean
  importedPngBytes: Uint8Array | null
}>()

defineEmits<{
  (e: 'reset'): void
  (e: 'export', payload: { sign: boolean }): void
  (e: 'build'): void
}>()
</script>

<template>
  <div class="toolbar">
    <div class="bcrumb">
      <span class="crumb">{{ t('creator.breadcrumb') }}</span>
      <span class="sep">/</span>
      <span class="crumb active">
        {{ metaName || 'Untitled' }}
        <span class="draft-tag">v{{ metaVersion }} · {{ t('creator.draft') }}</span>
      </span>
    </div>
    <div />
    <div class="tb-actions">
      <button
        class="btn ghost"
        aria-label="クリアして初期画面に戻る"
        title="編集中のアセットを破棄して初期画面に戻る"
        @click="$emit('reset')"
      >
        <UiIcon name="X" :size="13" />クリア
      </button>
      <span v-if="hasKeystoreSigning" class="tag ok">
        <UiIcon name="Shield" :size="11" />{{ t('creator.signedTag') }}
      </span>
      <span v-else class="tag" style="color: var(--rose); border-color: rgba(255, 107, 138, 0.3)">
        <UiIcon name="Alert" :size="11" />{{ t('creator.unsignedTag') }}
      </span>
      <button
        class="btn ghost"
        :disabled="exportBusy || !arrowAssigned"
        title=".cursorpack"
        @click="$emit('export', { sign: false })"
      >
        <span v-if="exportBusy" class="spinner" style="width: 13px; height: 13px" />
        <UiIcon v-else name="Export" :size="14" />
        {{ exportBusy ? t('creator.exportBusy') : t('creator.exportPack') }}
      </button>
      <button
        v-if="hasKeystoreSigning"
        class="btn primary"
        :disabled="exportBusy || !arrowAssigned"
        @click="$emit('export', { sign: true })"
      >
        <UiIcon name="Shield" :size="14" />{{ t('creator.exportSign') }}
      </button>
      <button
        v-else
        class="btn primary"
        :disabled="buildBusy || !importedPngBytes"
        @click="$emit('build')"
      >
        <span v-if="buildBusy" class="spinner" style="width: 13px; height: 13px" />
        <UiIcon v-else name="Check" :size="14" />
        {{ buildBusy ? t('creator.buildBusy') : t('creator.buildExport') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 12px;
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

.draft-tag {
  margin-left: 8px;
  font-size: 11px;
  font-weight: 400;
  color: var(--text-mute);
  font-family: var(--font-mono);
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
}

.btn.ghost {
  background: transparent;
}

.btn.primary {
  background: var(--accent);
  color: var(--bg-base);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  font-size: 11px;
  color: var(--text-mute);
}

.tag.ok {
  color: var(--mint);
  border-color: rgba(106, 213, 184, 0.3);
}

.spinner {
  display: inline-block;
  width: 13px;
  height: 13px;
  border: 2px solid var(--text-mute);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 800ms linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
