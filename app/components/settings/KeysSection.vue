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
          <SettingsRow :label="t('settings.keyIdLabel')" mono>
            <span class="tag ok">{{ keystoreInfo.key_id ?? '—' }}</span>
          </SettingsRow>
          <SettingsRow
            v-if="keystoreInfo.public_key_b64"
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
            :label="t('settings.exportPrivateLabel')"
            :desc="t('settings.exportPrivateDesc')"
          >
            <button class="btn" :disabled="keystoreBusy" @click="$emit('export')">
              <UiIcon name="Export" :size="13" />{{ t('common.export') }}
            </button>
          </SettingsRow>
          <SettingsRow :label="t('settings.regenerateLabel')" :desc="t('settings.regenerateDesc')">
            <button class="btn danger" :disabled="keystoreBusy" @click="$emit('regenerate')">
              <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
              <UiIcon v-else name="Alert" :size="13" />{{ t('settings.btnRegenerate') }}
            </button>
          </SettingsRow>
          <SettingsRow :label="t('settings.deleteKeyLabel')" :desc="t('settings.deleteKeyDesc')">
            <button class="btn danger" :disabled="keystoreBusy" @click="$emit('delete')">
              <UiIcon name="X" :size="13" />{{ t('common.delete') }}
            </button>
          </SettingsRow>
        </template>
        <template v-else>
          <SettingsRow :label="t('settings.generateLabel')" :desc="t('settings.generateDesc')">
            <button class="btn primary" :disabled="keystoreBusy" @click="$emit('generate')">
              <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
              <UiIcon v-else name="Plus" :size="13" />{{ t('settings.btnGenerate') }}
            </button>
          </SettingsRow>
          <SettingsRow
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
.section-head {
  margin-bottom: 16px;
}
.section-head h1 {
  font-size: 18px;
  font-weight: 700;
  margin: 0 0 4px 0;
}
.section-head p {
  font-size: 13px;
  color: var(--text-mute);
  margin: 0;
}
.prop-section {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elev1);
}
.prop-head {
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-mute);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}
.head-hint {
  font-size: 11px;
  font-weight: 400;
  text-transform: none;
  letter-spacing: 0;
  color: var(--text-mute);
}
.prop-body {
  padding: 4px 16px 12px;
}
.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
}
.btn.primary {
  background: var(--accent);
  color: var(--bg-base);
  border-color: var(--accent);
}
.btn.danger {
  color: var(--rose);
  border-color: rgba(255, 107, 138, 0.3);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  font-size: 11px;
  color: var(--text-mute);
  font-family: var(--font-mono);
}
.tag.ok {
  color: var(--mint);
  border-color: rgba(106, 213, 184, 0.3);
}
.profile-msg {
  margin-top: 8px;
  padding: 8px 12px;
  font-size: 12px;
  border-radius: 8px;
  background: rgba(106, 213, 184, 0.06);
  border: 1px solid rgba(106, 213, 184, 0.4);
  color: var(--mint);
}
.spinner {
  display: inline-block;
  width: 13px;
  height: 13px;
  border: 2px solid var(--text-mute);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 800ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
