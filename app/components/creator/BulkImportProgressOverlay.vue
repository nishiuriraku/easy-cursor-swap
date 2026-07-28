<script setup lang="ts">
/**
 * 一括インポート (bulk_resolve / cursorpack parse) 実行中のオーバーレイ (LD5)。
 *
 * BulkImportPreviewModal が開く前の「解析中」フェーズに、StageStepper + 確定バー +
 * キャンセルボタンを表示する。これが無いと resolve 中は UI が完全に無反応に見える
 * (`useBulkImport` が progress を計算しているのに描画先が無かった)。
 *
 * 表示条件は親 (creator.vue) が `bulkImport.busy` で制御する。total が 0 (scan 中など
 * 件数未確定) のときは UiProgress が自動で不確定アニメに倒す。
 */
interface BulkProgress {
  jobId: string
  stage: 'scan' | 'parse' | 'extract' | 'done' | 'error' | string
  current: number
  total: number
  message: string | null
}

const props = defineProps<{
  progress: BulkProgress | null
}>()

defineEmits<{
  (e: 'cancel'): void
}>()

const { t } = useI18n()

const stages = computed(() => [
  { id: 'scan', label: t('creator.bulkStageScan') },
  { id: 'parse', label: t('creator.bulkStageParse') },
  { id: 'extract', label: t('creator.bulkStageExtract') },
])

const currentId = computed(() => props.progress?.stage ?? 'scan')
const failedId = computed(() => (props.progress?.stage === 'error' ? currentId.value : null))
const current = computed(() => props.progress?.current ?? 0)
const total = computed(() => props.progress?.total ?? 0)
</script>

<template>
  <div class="bulk-overlay">
    <div class="bulk-card" role="status" aria-live="polite">
      <div class="bulk-head">
        <UiSpinner :size="16" :label="t('creator.bulkImporting')" />
        <span class="bulk-title">{{ t('creator.bulkImporting') }}</span>
      </div>
      <UiStageStepper :stages="stages" :current-id="currentId" :failed-id="failedId" />
      <UiProgress
        :value="current"
        :max="total"
        :show-percent="total > 0"
        :aria-label="t('creator.bulkImporting')"
      />
      <div v-if="progress?.message" class="bulk-msg">{{ progress.message }}</div>
      <div class="bulk-actions">
        <UiButton variant="ghost" icon-left="X" @click="$emit('cancel')">
          {{ t('creator.bulkCancel') }}
        </UiButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.bulk-overlay {
  @apply fixed inset-0 z-50 grid place-items-center p-6;
  background: rgba(10, 11, 15, 0.55);
  backdrop-filter: blur(2px);
}
:where(html.light) .bulk-overlay {
  background: rgba(15, 20, 35, 0.32);
}
.bulk-card {
  @apply grid w-full max-w-[420px] gap-3 rounded-xl border border-line p-5;
  background: var(--bg-2);
  box-shadow: var(--shadow-2);
}
.bulk-head {
  @apply flex items-center gap-2.5;
}
.bulk-title {
  @apply text-[13px] font-semibold text-fg;
}
.bulk-msg {
  @apply truncate font-mono text-[11px] text-fg-mute;
}
.bulk-actions {
  @apply mt-1 flex justify-end;
}
</style>
