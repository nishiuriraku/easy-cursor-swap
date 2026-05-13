<script setup lang="ts">
/**
 * 設定 → 署名鍵 セクション。
 *
 * Ed25519 鍵ペアの生成 / 削除 / Export / Import / Regenerate を提供する。
 * 鍵情報・進捗・エラー状態は `useKeystore()` 由来のものを親が渡し、
 * 子は表示と emit のみ担当する (鍵ペアの実体生成 / IPC 呼び出しは settings.vue 側)。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

interface KeystoreInfo {
  has_keypair: boolean
  key_id: string | null
  public_key_b64: string | null
}

defineProps<{
  keystoreInfo: KeystoreInfo
  keystoreBusy: boolean
  keystoreError: string | null
  keystoreMessage: string | null
}>()

defineEmits<{
  (e: 'generate'): void
  (e: 'regenerate'): void
  (e: 'delete'): void
  (e: 'export'): void
  (e: 'import'): void
}>()
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionKeys') }}</h1>
      <p>{{ t('settings.descKeys') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">
        {{ t('settings.groupKeyPair') }}
        <span class="head-hint">{{ t('settings.keyPairHint') }}</span>
      </div>
      <div class="prop-body">
        <template v-if="keystoreInfo.has_keypair">
          <SettingsRow anchor="keyId" :label="t('settings.keyIdLabel')" mono>
            <span class="tag ok">{{ keystoreInfo.key_id ?? '—' }}</span>
          </SettingsRow>
          <SettingsRow
            v-if="keystoreInfo.public_key_b64"
            anchor="publicKey"
            :label="t('settings.publicKeyLabel')"
            :desc="t('settings.publicKeyDesc')"
            mono
          >
            <span
              class="tag"
              style="
                max-width: 320px;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
                display: inline-block;
              "
            >
              {{ keystoreInfo.public_key_b64 }}
            </span>
          </SettingsRow>
          <SettingsRow
            anchor="exportPrivate"
            :label="t('settings.exportPrivateLabel')"
            :desc="t('settings.exportPrivateDesc')"
          >
            <button class="btn" :disabled="keystoreBusy" @click="$emit('export')">
              <UiIcon name="Export" :size="13" />{{ t('common.export') }}
            </button>
          </SettingsRow>
          <SettingsRow
            anchor="regenerate"
            :label="t('settings.regenerateLabel')"
            :desc="t('settings.regenerateDesc')"
          >
            <button class="btn danger" :disabled="keystoreBusy" @click="$emit('regenerate')">
              <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
              <UiIcon v-else name="Alert" :size="13" />{{ t('settings.btnRegenerate') }}
            </button>
          </SettingsRow>
          <SettingsRow
            anchor="deleteKey"
            :label="t('settings.deleteKeyLabel')"
            :desc="t('settings.deleteKeyDesc')"
          >
            <button class="btn danger" :disabled="keystoreBusy" @click="$emit('delete')">
              <UiIcon name="X" :size="13" />{{ t('common.delete') }}
            </button>
          </SettingsRow>
        </template>
        <template v-else>
          <SettingsRow
            anchor="generate"
            :label="t('settings.generateLabel')"
            :desc="t('settings.generateDesc')"
          >
            <button class="btn primary" :disabled="keystoreBusy" @click="$emit('generate')">
              <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
              <UiIcon v-else name="Plus" :size="13" />{{ t('settings.btnGenerate') }}
            </button>
          </SettingsRow>
          <SettingsRow
            anchor="importExisting"
            :label="t('settings.importExistingLabel')"
            :desc="t('settings.importExistingDesc')"
          >
            <button class="btn" :disabled="keystoreBusy" @click="$emit('import')">
              <UiIcon name="Import" :size="13" />{{ t('common.import') }}
            </button>
          </SettingsRow>
        </template>
        <div v-if="keystoreMessage" class="profile-msg">
          {{ keystoreMessage }}
        </div>
        <div
          v-if="keystoreError"
          class="profile-msg"
          style="
            background: rgba(255, 107, 138, 0.06);
            border-color: rgba(255, 107, 138, 0.4);
            color: #ffb8c5;
          "
        >
          {{ keystoreError }}
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* NOTE: dead-var pattern (Phase 6-F 参照)。scoped は layout/spacing 差分のみ。 */

.section-head {
  @apply mb-4;
}
.section-head h1 {
  @apply mb-1 mt-0 text-[18px] font-bold;
}
.section-head p {
  @apply m-0 text-[13px];
}
.prop-section {
  border-radius: 12px;
}
.prop-head {
  @apply flex items-baseline justify-between;
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.head-hint {
  @apply text-[11px] font-normal normal-case tracking-normal;
}
.prop-body {
  padding: 4px 16px 12px;
}
.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}
.btn.danger {
  @apply text-rose;
  border-color: rgba(255, 107, 138, 0.3);
}
.btn:disabled {
  @apply cursor-not-allowed opacity-50;
}
.tag {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  gap: 4px;
  font-family: var(--font-mono);
}
.tag.ok {
  border-color: rgba(106, 213, 184, 0.3);
}
.profile-msg {
  @apply mt-2 rounded-[8px] border px-3 py-2 text-[12px];
  background: rgba(106, 213, 184, 0.06);
  border-color: rgba(106, 213, 184, 0.4);
}
.spinner {
  @apply inline-block size-[13px] rounded-full;
  border: 2px solid var(--fg-mute);
  border-top-color: transparent;
  animation: spin 800ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
