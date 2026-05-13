<script setup lang="ts">
/**
 * 設定 → ログ・診断 セクション。
 *
 * ログレベル / 保持期間 / 合計上限 / フォルダを開く ボタン。
 * Open ボタンは `open_log_folder` IPC 経由で `logging::log_dir()` を
 * Windows Explorer で開く。
 */
import { useI18n } from '~/composables/useI18n'
import { invokeTauri } from '~/composables/useTauri'

const { t } = useI18n()

const logLevel = defineModel<string>('logLevel', { required: true })
const retentionDays = defineModel<number>('retentionDays', { required: true })
const maxSizeMb = defineModel<number>('maxSizeMb', { required: true })

async function openLogFolder() {
  try {
    await invokeTauri<void>('open_log_folder')
  } catch (err) {
    // Tauri 未接続 (web preview) では何もできない。コンソールに warning を出して握る。
    console.error('open_log_folder failed:', err)
  }
}
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionLogging') }}</h1>
      <p>{{ t('settings.descLogging') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupLogOutput') }}</div>
      <div class="prop-body">
        <SettingsRow :label="t('settings.logLevelLabel')" :desc="t('settings.logLevelDesc')">
          <UiSelect
            v-model="logLevel"
            width="140px"
            :options="[
              { value: 'TRACE', label: 'TRACE' },
              { value: 'DEBUG', label: 'DEBUG' },
              { value: 'INFO', label: 'INFO' },
              { value: 'WARN', label: 'WARN' },
              { value: 'ERROR', label: 'ERROR' },
            ]"
          />
        </SettingsRow>
        <SettingsRow :label="t('settings.retentionLabel')" :desc="t('settings.retentionDesc')">
          <input
            v-model.number="retentionDays"
            type="number"
            class="input"
            min="1"
            max="365"
            style="width: 80px"
          />
        </SettingsRow>
        <SettingsRow :label="t('settings.maxSizeLabel')" :desc="t('settings.maxSizeDesc')">
          <input
            v-model.number="maxSizeMb"
            type="number"
            class="input"
            min="10"
            max="2048"
            style="width: 80px"
          />
        </SettingsRow>
        <SettingsRow
          :label="t('settings.openLogFolderLabel')"
          :desc="t('settings.openLogFolderDesc')"
        >
          <button class="btn" @click="openLogFolder">
            <UiIcon name="Globe" :size="13" />{{ t('settings.btnOpen') }}
          </button>
        </SettingsRow>
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
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.prop-body {
  padding: 4px 16px;
}
.input {
  height: 32px;
  border-radius: 8px;
  padding: 0 10px;
  font-size: 13px;
}
.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}
</style>
