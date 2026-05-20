<script setup lang="ts">
/**
 * 確認ダイアログ。`UiModal` を内部で使うラッパで、
 * 「タイトル + 1 段落説明 + cancel/confirm」専用。
 *
 * 複雑な body (KV リストや diff グリッド) が要る場合は `UiModal` を直接使う。
 */
const { t } = useI18n()

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    message: string
    confirmLabel?: string
    cancelLabel?: string
    tone?: 'primary' | 'danger'
    icon?: string
    busy?: boolean
    leftNote?: string
  }>(),
  {
    tone: 'primary',
    busy: false,
  },
)

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
  cancel: []
}>()

const resolvedIcon = computed(() => props.icon ?? (props.tone === 'danger' ? 'Alert' : 'Check'))
const iconTone = computed<'accent' | 'danger'>(() =>
  props.tone === 'danger' ? 'danger' : 'accent',
)
const resolvedCancelLabel = computed(() => props.cancelLabel ?? t('common.cancel'))
const resolvedConfirmLabel = computed(() => props.confirmLabel ?? t('common.confirm'))

function onCancel() {
  emit('update:open', false)
  emit('cancel')
}
function onConfirm() {
  emit('confirm')
}
</script>

<template>
  <UiModal
    :open="open"
    :title="title"
    :description="message"
    :icon="resolvedIcon"
    :icon-tone="iconTone"
    size="sm"
    :busy="busy"
    @update:open="(val) => emit('update:open', val)"
    @close="onCancel"
  >
    <template v-if="leftNote" #leftNote>
      <UiIcon name="Shield" :size="12" />
      <span>{{ leftNote }}</span>
    </template>
    <template #actions>
      <UiButton variant="ghost" :disabled="busy" @click="onCancel">
        {{ resolvedCancelLabel }}
      </UiButton>
      <UiButton
        :variant="tone === 'danger' ? 'danger' : 'primary'"
        :loading="busy"
        :icon-left="resolvedIcon"
        @click="onConfirm"
      >
        {{ resolvedConfirmLabel }}
      </UiButton>
    </template>
  </UiModal>
</template>
