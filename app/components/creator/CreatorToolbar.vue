<script setup lang="ts">
/**
 * Creator のツールバー (パンくず + クリアボタン + 署名状態タグ + Export ボタン)。
 *
 * メタデータ表示 (テーマ名 / バージョン) と export 状態を props で受け取り、
 * アクション 2 種 (clear / export) を emit で親に通知する。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

defineProps<{
  metaName: string
  metaVersion: string
  hasKeystoreSigning: boolean
  exportBusy: boolean
  arrowAssigned: boolean
}>()

defineEmits<{
  (e: 'reset'): void
  (e: 'export', payload: { sign: boolean }): void
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
    </div>
  </div>
</template>

<style scoped>
/* NOTE: 元の scoped style は --border / --bg-elev1 / --bg-elev2 / --text /
 * --text-mute / --bg-base / --mint などの未定義トークンを多数含み、それらは
 * cascade で global.css の .toolbar / .bcrumb / .btn / .tag ルールに引き継がれ
 * ていた。Tailwind の border utility を @apply で持ち込むと border-color が
 * currentColor に化けて global の subtle border を上書きするため、scoped 側は
 * global と衝突しないレイアウト/スペーシングの差分のみを CSS リテラルで保持する。 */

.toolbar {
  gap: 12px;
  padding: 10px 16px;
}

.bcrumb {
  font-size: 12px;
}

.crumb.active {
  font-weight: 600;
}

.draft-tag {
  margin-left: 8px;
  font-size: 11px;
  font-weight: 400;
  font-family: var(--font-mono);
}

.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}

.btn.ghost {
  background: transparent;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.tag {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  gap: 4px;
}

.tag.ok {
  border-color: rgba(106, 213, 184, 0.3);
}

.spinner {
  display: inline-block;
  width: 13px;
  height: 13px;
  border: 2px solid var(--fg-mute);
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
