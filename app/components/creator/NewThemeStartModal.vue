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
.nt-overlay {
  position: fixed;
  inset: 0;
  background: rgba(10, 11, 15, 0.7);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.nt-modal {
  background: var(--bg-1, #14161c);
  border: 1px solid var(--line);
  border-radius: 12px;
  width: min(560px, 96vw);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 30px 60px rgba(0, 0, 0, 0.45);
}
.nt-head,
.nt-foot {
  padding: 14px 20px;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}
.nt-head {
  border-bottom: 1px solid var(--line);
}
.nt-foot {
  border-top: 1px solid var(--line);
  align-items: center;
  justify-content: flex-end;
}
.nt-head h3 {
  margin: 4px 0 0;
  font-size: 16px;
  font-weight: 600;
}
.nt-eyebrow {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--accent);
}
.nt-body {
  padding: 14px 20px 18px;
  overflow-y: auto;
}
.nt-desc {
  font-size: 12.5px;
  color: var(--fg-dim);
  margin: 0 0 14px;
  line-height: 1.55;
}

.nt-choices {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.nt-choice {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(124, 242, 212, 0.02);
  text-align: left;
  cursor: pointer;
  color: var(--fg);
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
  flex: 0 0 auto;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  background: rgba(124, 242, 212, 0.08);
}
.nt-choice-body {
  flex: 1;
  min-width: 0;
}
.nt-choice-title {
  font-size: 13.5px;
  font-weight: 600;
  margin-bottom: 2px;
}
.nt-choice-sub {
  font-size: 11.5px;
  color: var(--fg-mute);
  line-height: 1.5;
}

.nt-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 14px 0 0;
  font-size: 11px;
  color: var(--fg-mute);
}
</style>
