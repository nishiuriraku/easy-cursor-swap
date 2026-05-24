<script setup lang="ts">
/**
 * パスフレーズ入力プロンプト (秘密鍵 export/import 用)。
 *
 * - 確認入力を強制 (export 時のみ)
 * - 8 文字以上の弱バリデーション
 * - エンターで確定
 *
 * 共通の `<UiModal>` shell を使用。focus trap が body 内の最初の focusable
 * (= passphrase input) に自動 focus するため、open 直後にパスワード欄が
 * フォーカスされる UX 期待を満たす。
 */

const { t } = useI18n()

const props = defineProps<{
  /** モード: export(2回入力) / import(1回入力) */
  mode: 'export' | 'import'
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [v: boolean]
  /** confirm payload: passphrase 文字列 */
  confirm: [passphrase: string]
}>()

const phrase = ref('')
const phraseConfirm = ref('')

watch(
  () => props.open,
  (v) => {
    if (v) {
      phrase.value = ''
      phraseConfirm.value = ''
    }
  },
)

const canConfirm = computed(() => {
  if (phrase.value.length < 8) return false
  if (props.mode === 'export' && phrase.value !== phraseConfirm.value) return false
  return true
})

const error = computed(() => {
  if (phrase.value.length === 0) return ''
  if (phrase.value.length < 8) return t('passphrase.errorTooShort')
  if (props.mode === 'export' && phraseConfirm.value && phrase.value !== phraseConfirm.value) {
    return t('passphrase.errorMismatch')
  }
  return ''
})

function close() {
  emit('update:open', false)
}

function confirm() {
  if (!canConfirm.value) return
  emit('confirm', phrase.value)
  close()
}
</script>

<template>
  <UiModal
    :open="open"
    :title="mode === 'export' ? t('passphrase.exportTitle') : t('passphrase.importTitle')"
    :description="mode === 'export' ? t('passphrase.exportDesc') : t('passphrase.importDesc')"
    icon="Shield"
    size="sm"
    @close="close"
  >
    <label class="pp-row">
      <span class="pp-label">{{ t('passphrase.phraseLabel') }}</span>
      <input
        v-model="phrase"
        type="password"
        class="input mono"
        autocomplete="new-password"
        @keydown.enter="confirm"
      />
    </label>
    <label v-if="mode === 'export'" class="pp-row">
      <span class="pp-label">{{ t('passphrase.confirmLabel') }}</span>
      <input
        v-model="phraseConfirm"
        type="password"
        class="input mono"
        autocomplete="new-password"
        @keydown.enter="confirm"
      />
    </label>
    <p v-if="error" class="pp-error">{{ error }}</p>
    <p class="pp-note">
      <UiIcon name="Alert" :size="11" />
      {{ t('passphrase.note') }}
    </p>

    <template #leftNote>
      <UiIcon name="Shield" :size="12" style="color: var(--accent)" />
      XChaCha20-Poly1305 + Argon2id (m=64MiB, t=3)
    </template>
    <template #actions>
      <UiButton variant="ghost" @click="close">{{ t('common.cancel') }}</UiButton>
      <UiButton variant="primary" icon-left="Check" :disabled="!canConfirm" @click="confirm">
        {{ mode === 'export' ? t('common.export') : t('common.import') }}
      </UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.pp-row {
  @apply mb-3.5 flex flex-col gap-1.5;
}
.pp-label {
  @apply font-mono text-[10px] uppercase tracking-[0.12em] text-fg-mute;
}
.pp-error {
  @apply mb-2.5 text-[12px] text-rose;
  margin-top: 0;
  margin-left: 0;
  margin-right: 0;
}
.pp-note {
  @apply mt-1 flex items-center gap-1.5 text-[11.5px] text-fg-dim;
}
</style>
