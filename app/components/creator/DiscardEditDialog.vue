<script setup lang="ts">
/**
 * Creator の編集破棄を確認するダイアログ。
 *
 * 表示トリガ:
 *  - Clear ボタン押下時 (mode='clear')
 *  - 別ページへのナビゲーション直前 (mode='navigate')
 *
 * `creator.vue` 側で `hasUnsavedEdits` (アセット or メタ編集) を判定してから開く。
 * confirm すると親が resetCreator() または router.next() を実行する。
 *
 * 既存の ImportConflictDialog と同じ `.modal-page` / `.modal` 共有 utility 構成。
 */

const { t } = useI18n()

defineProps<{
  open: boolean
  mode: 'clear' | 'navigate'
}>()

const emit = defineEmits<{
  /** 「破棄して続行」を選択 */
  (e: 'confirm'): void
  /** キャンセル (バックドロップクリック / Esc / キャンセルボタン) */
  (e: 'cancel'): void
}>()

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit('cancel')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('cancel')
}
</script>

<template>
  <div
    v-if="open"
    class="modal-page"
    role="dialog"
    aria-modal="true"
    aria-labelledby="discard-edit-title"
    tabindex="-1"
    @click="onBackdrop"
    @keydown="onKeydown"
  >
    <div class="modal discard-modal" @click.stop>
      <div class="modal-head">
        <div
          class="modal-icon"
          aria-hidden="true"
          :style="{
            borderColor: 'rgba(255, 107, 138, 0.35)',
            color: 'var(--rose)',
            background: 'rgba(255, 107, 138, 0.12)',
          }"
        >
          <UiIcon name="Alert" :size="20" />
        </div>
        <div style="flex: 1; min-width: 0">
          <h2 id="discard-edit-title">
            {{
              mode === 'clear'
                ? t('creator.discardDialog.titleClear')
                : t('creator.discardDialog.titleNavigate')
            }}
          </h2>
          <p>
            {{
              mode === 'clear'
                ? t('creator.discardDialog.messageClear')
                : t('creator.discardDialog.messageNavigate')
            }}
          </p>
        </div>
      </div>

      <div class="modal-foot">
        <div class="left-note">
          <UiIcon name="Alert" :size="12" :style="{ color: 'var(--rose)' }" />
          {{ t('creator.discardDialog.warning') }}
        </div>
        <div class="actions">
          <button class="btn ghost" @click="emit('cancel')">
            {{ t('common.cancel') }}
          </button>
          <button class="btn danger" @click="emit('confirm')">
            <UiIcon name="X" :size="13" />
            {{ t('creator.discardDialog.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.discard-modal {
  @apply w-[480px];
}
</style>
