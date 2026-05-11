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
  margin-bottom: 12px;
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
.btn:disabled {
  @apply cursor-not-allowed opacity-50;
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
