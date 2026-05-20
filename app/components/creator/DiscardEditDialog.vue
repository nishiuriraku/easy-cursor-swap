<script setup lang="ts">
/**
 * Creator の編集破棄を確認するダイアログ。
 * mode='clear' / 'navigate' で文言を切り替える。
 *
 * 内部実装は `UiConfirmDialog` (danger tone) を薄くラップしているだけ。
 */
const { t } = useI18n()

const props = defineProps<{
  open: boolean
  mode: 'clear' | 'navigate'
}>()

const emit = defineEmits<{
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

const title = computed(() =>
  props.mode === 'clear'
    ? t('creator.discardDialog.titleClear')
    : t('creator.discardDialog.titleNavigate'),
)
const message = computed(() =>
  props.mode === 'clear'
    ? t('creator.discardDialog.messageClear')
    : t('creator.discardDialog.messageNavigate'),
)
const confirmLabel = computed(() => t('creator.discardDialog.confirm'))
const warning = computed(() => t('creator.discardDialog.warning'))
</script>

<template>
  <UiConfirmDialog
    :open="open"
    :title="title"
    :message="message"
    tone="danger"
    icon="Alert"
    :confirm-label="confirmLabel"
    :left-note="warning"
    @confirm="emit('confirm')"
    @cancel="emit('cancel')"
  />
</template>
