<script setup lang="ts">
/**
 * 「新規作成」を押したときに表示されるモーダル (chooser 構成)。
 *
 * 設計方針 (旧版からの変更):
 *  - 旧版は単一ファイルピッカ + プレビュー + Arrow ロール割当の専用モーダルだった。
 *    複数ファイル取込ができず、bulkImport との分岐重複が技術負債化していたため、
 *    本版ではモーダル自体は薄い chooser に縮小し、実際の取込は creator.vue の
 *    bulkImport ハンドラに委譲する。
 *  - 3 つのエントリ:
 *      1. ファイル/パックから (png/svg/cur/ico/.cursorpack 統合 — 主導線)
 *      2. フォルダから        (フォルダ単位の一括スキャン)
 *      3. 画像なしで開始          (空のキャンバスから編集開始)
 *  - 取込結果は `BulkImportPreviewModal` に流れ、ロールマッチング/上書き判定/
 *    メタデータ反映を経て Creator が editing ステージに遷移する。
 */

const { t } = useI18n()

interface Props {
  open: boolean
}
defineProps<Props>()

const emit = defineEmits<{
  /** ファイル/パック取込ダイアログを開く。creator.vue の `pickBulkAuto` に流す。 */
  (e: 'pick-files'): void
  /** フォルダ取込ダイアログを開く。creator.vue の `pickBulkFolder` に流す。 */
  (e: 'pick-folder'): void
  /** 画像なしで空テンプレから編集開始。 */
  (e: 'start-empty'): void
  /** モーダルを閉じる (キャンセル / Esc / オーバーレイクリック)。 */
  (e: 'cancel'): void
}>()

function pickFiles() {
  emit('pick-files')
}
function pickFolder() {
  emit('pick-folder')
}
function startEmpty() {
  emit('start-empty')
}
function cancel() {
  emit('cancel')
}
</script>

<template>
  <UiModal
    :open="open"
    :title="t('newTheme.title')"
    :description="t('newTheme.description')"
    icon="Plus"
    size="md"
    @close="cancel"
  >
    <div class="nt-choices">
      <button class="nt-choice primary" @click="pickFiles">
        <div class="nt-choice-icon">
          <UiIcon name="Import" :size="22" />
        </div>
        <div class="nt-choice-body">
          <div class="nt-choice-title">{{ t('newTheme.choiceFiles') }}</div>
          <div class="nt-choice-sub">{{ t('newTheme.choiceFilesSub') }}</div>
        </div>
      </button>

      <button class="nt-choice" @click="pickFolder">
        <div class="nt-choice-icon">
          <UiIcon name="Library" :size="22" />
        </div>
        <div class="nt-choice-body">
          <div class="nt-choice-title">{{ t('newTheme.choiceFolder') }}</div>
          <div class="nt-choice-sub">{{ t('newTheme.choiceFolderSub') }}</div>
        </div>
      </button>

      <button class="nt-choice" @click="startEmpty">
        <div class="nt-choice-icon">
          <UiIcon name="Brush" :size="22" />
        </div>
        <div class="nt-choice-body">
          <div class="nt-choice-title">{{ t('newTheme.choiceEmpty') }}</div>
          <div class="nt-choice-sub">{{ t('newTheme.choiceEmptySub') }}</div>
        </div>
      </button>
    </div>

    <template #leftNote>
      <UiIcon name="Shield" :size="12" />
      <span>{{ t('newTheme.tip') }}</span>
    </template>

    <template #actions>
      <UiButton variant="ghost" @click="cancel">{{ t('common.cancel') }}</UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.nt-choices {
  @apply flex flex-col gap-2.5;
}
.nt-choice {
  @apply flex cursor-pointer items-center gap-3.5 rounded-[10px] border border-line px-4 py-3.5 text-left text-fg;
  background: rgba(124, 242, 212, 0.02);
  transition:
    border-color 160ms ease,
    background 160ms ease,
    transform 80ms ease;
}
.nt-choice:hover {
  border-color: var(--accent-line);
  background: rgba(124, 242, 212, 0.05);
}
.nt-choice:active {
  transform: translateY(1px);
}
.nt-choice.primary {
  border-color: var(--accent-line);
  background: rgba(124, 242, 212, 0.06);
}
.nt-choice-icon {
  @apply flex size-9 shrink-0 items-center justify-center rounded-[8px] text-accent;
  background: rgba(124, 242, 212, 0.08);
}
.nt-choice-body {
  @apply min-w-0 flex-1;
}
.nt-choice-title {
  @apply mb-0.5 text-[13.5px] font-semibold;
}
.nt-choice-sub {
  @apply text-[11.5px] leading-[1.5] text-fg-mute;
}
</style>
