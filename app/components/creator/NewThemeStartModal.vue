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
import { useI18n } from '~/composables/useI18n'

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
  <div v-if="open" class="nt-overlay" @click.self="cancel" @keydown.esc="cancel">
    <div class="nt-modal" role="dialog" aria-modal="true" :aria-label="t('newTheme.title')">
      <header class="nt-head">
        <div>
          <div class="nt-eyebrow">CREATOR · NEW</div>
          <h3>{{ t('newTheme.title') }}</h3>
        </div>
        <button class="btn ghost" :aria-label="t('common.close')" @click="cancel">✕</button>
      </header>

      <div class="nt-body">
        <p class="nt-desc">{{ t('newTheme.description') }}</p>

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

        <p class="nt-tip">
          <UiIcon name="Shield" :size="11" />
          {{ t('newTheme.tip') }}
        </p>
      </div>

      <footer class="nt-foot">
        <button class="btn ghost" @click="cancel">{{ t('common.cancel') }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.nt-overlay {
  @apply fixed inset-0 z-[100] flex items-center justify-center bg-[rgba(10,11,15,0.7)] backdrop-blur-[2px];
}
.nt-modal {
  @apply flex max-h-[90vh] w-[min(560px,96vw)] flex-col rounded-[12px] border border-line shadow-[0_30px_60px_rgba(0,0,0,0.45)];
  background: var(--bg-1, #14161c);
}
.nt-head,
.nt-foot {
  @apply flex items-start justify-between px-5 py-3.5;
}
.nt-head {
  @apply border-b border-line;
}
.nt-foot {
  @apply items-center justify-end border-t border-line;
}
.nt-head h3 {
  @apply mb-0 ml-0 mr-0 mt-1 text-[16px] font-semibold;
}
.nt-eyebrow {
  @apply font-mono text-[10px] tracking-[0.16em] text-accent;
}
.nt-body {
  @apply overflow-y-auto px-5 pb-[18px] pt-3.5;
}
.nt-desc {
  @apply m-0 mb-3.5 text-[12.5px] leading-[1.55] text-fg-dim;
}

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

.nt-tip {
  @apply m-0 mt-3.5 flex items-center gap-1.5 text-[11px] text-fg-mute;
}
</style>
