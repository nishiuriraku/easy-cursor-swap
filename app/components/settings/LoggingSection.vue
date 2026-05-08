<script setup lang="ts">
/**
 * 設定 → ログ・診断 セクション。
 *
 * ログレベル / 保持期間 / 合計上限 / フォルダを開く ボタン。
 * フォルダオープンは将来的に IPC 経由で実装予定 (現在はダミーボタン)。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const logLevel = defineModel<string>('logLevel', { required: true })
const retentionDays = defineModel<number>('retentionDays', { required: true })
const maxSizeMb = defineModel<number>('maxSizeMb', { required: true })
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
          <button class="btn"><UiIcon name="Globe" :size="13" />{{ t('settings.btnOpen') }}</button>
        </SettingsRow>
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
}
.prop-body {
  padding: 4px 16px;
}
.input {
  height: 32px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  padding: 0 10px;
  font-size: 13px;
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
</style>
