<script setup lang="ts">
/**
 * パスフレーズ入力プロンプト (秘密鍵 export/import 用)。
 *
 * - 確認入力を強制 (export 時のみ)
 * - 8 文字以上の弱バリデーション
 * - エンターで確定
 */
import { computed, ref, watch } from 'vue'
import { useI18n } from '~/composables/useI18n'

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

watch(() => props.open, (v) => {
  if (v) {
    phrase.value = ''
    phraseConfirm.value = ''
  }
})

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
  <Transition name="fade">
    <div v-if="open" class="modal-page" role="dialog" aria-modal="true" aria-labelledby="passphrase-modal-title" @click.self="close">
      <div class="modal pp-modal">
        <div class="modal-head">
          <div class="modal-icon" aria-hidden="true"><UiIcon name="Shield" :size="20" /></div>
          <div style="flex: 1; min-width: 0">
            <h2 id="passphrase-modal-title">
              {{ mode === 'export' ? t('passphrase.exportTitle') : t('passphrase.importTitle') }}
            </h2>
            <p>{{ mode === 'export' ? t('passphrase.exportDesc') : t('passphrase.importDesc') }}</p>
          </div>
        </div>

        <div class="modal-body">
          <label class="pp-row">
            <span class="pp-label">{{ t('passphrase.phraseLabel') }}</span>
            <input
              v-model="phrase"
              type="password"
              class="input mono"
              autocomplete="new-password"
              @keydown.enter="confirm"
            >
          </label>
          <label v-if="mode === 'export'" class="pp-row">
            <span class="pp-label">{{ t('passphrase.confirmLabel') }}</span>
            <input
              v-model="phraseConfirm"
              type="password"
              class="input mono"
              autocomplete="new-password"
              @keydown.enter="confirm"
            >
          </label>
          <p v-if="error" class="pp-error">{{ error }}</p>
          <p class="pp-note">
            <UiIcon name="Alert" :size="11" />
            {{ t('passphrase.note') }}
          </p>
        </div>

        <div class="modal-foot">
          <div class="left-note">
            <UiIcon name="Shield" :size="12" style="color: var(--accent)" />
            XChaCha20-Poly1305 + Argon2id (m=64MiB, t=3)
          </div>
          <div class="actions">
            <button class="btn ghost" @click="close">{{ t('common.cancel') }}</button>
            <button class="btn primary" :disabled="!canConfirm" @click="confirm">
              <UiIcon name="Check" :size="13" />
              {{ mode === 'export' ? t('common.export') : t('common.import') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.pp-modal { width: 460px; }
.pp-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}
.pp-label {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--fg-mute);
}
.pp-error {
  margin: 0 0 10px;
  font-size: 12px;
  color: var(--rose);
}
.pp-note {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 4px 0 0;
  font-size: 11.5px;
  color: var(--fg-dim);
}
.fade-enter-active, .fade-leave-active { transition: opacity 0.18s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
