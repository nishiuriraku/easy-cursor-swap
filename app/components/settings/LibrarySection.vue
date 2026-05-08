<script setup lang="ts">
/**
 * 設定 → ライブラリ セクション。
 *
 * ストレージ警告閾値 + .cursorprofile (config + 全テーマ ZIP) export/import。
 * profileBusy/profileMessage は親 (settings.vue) が IPC 呼び出しと進捗管理を持ち、
 * 子はそのスナップショットを読み取って UI に反映するだけ。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const totalLimitWarnGb = defineModel<number>('totalLimitWarnGb', { required: true })
const storageWarnEnabled = defineModel<boolean>('storageWarnEnabled', { required: true })

defineProps<{
  profileBusy: boolean
  profileMessage: string | null
}>()

defineEmits<{
  (e: 'export-profile'): void
  (e: 'import-profile'): void
}>()
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionLibrary') }}</h1>
      <p>{{ t('settings.descLibrary') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupStorageWarning') }}</div>
      <div class="prop-body">
        <SettingsRow
          :label="t('settings.storageThresholdLabel')"
          :desc="t('settings.storageThresholdDesc')"
        >
          <UiSelect
            v-model="totalLimitWarnGb"
            width="100px"
            :options="[
              { value: 0.5, label: '0.5 GB' },
              { value: 1, label: '1 GB' },
              { value: 2, label: '2 GB' },
              { value: 5, label: '5 GB' },
            ]"
          />
        </SettingsRow>
        <SettingsRow
          :label="t('settings.storageWarnEnabledLabel')"
          :desc="t('settings.storageWarnEnabledDesc')"
        >
          <SettingsToggle v-model="storageWarnEnabled" />
        </SettingsRow>
      </div>
    </div>

    <div class="prop-section">
      <div class="prop-head">
        {{ t('settings.groupProfileBackup') }}
        <span class="head-hint">{{ t('settings.profileBackupHint') }}</span>
      </div>
      <div class="prop-body">
        <SettingsRow
          :label="t('settings.profileExportLabel')"
          :desc="t('settings.profileExportDesc')"
        >
          <button class="btn" :disabled="profileBusy" @click="$emit('export-profile')">
            <span v-if="profileBusy" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Export" :size="13" />{{ t('common.export') }}
          </button>
        </SettingsRow>
        <SettingsRow
          :label="t('settings.profileImportLabel')"
          :desc="t('settings.profileImportDesc')"
        >
          <button class="btn" :disabled="profileBusy" @click="$emit('import-profile')">
            <span v-if="profileBusy" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Import" :size="13" />{{ t('common.import') }}
          </button>
        </SettingsRow>
        <div v-if="profileMessage" class="profile-msg">
          {{ profileMessage }}
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
  margin-bottom: 12px;
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
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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
