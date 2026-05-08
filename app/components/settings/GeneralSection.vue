<script setup lang="ts">
/**
 * 設定 → 一般 セクション。
 *
 * UI 言語選択 + 通知トグル 2 つ + ConfigRecoveryPanel (バックアップ復旧) を含む。
 * ConfigRecoveryPanel が emit する `restored` を親に伝播するためのラッパー。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const language = defineModel<string>('language', { required: true })
const showApplyToast = defineModel<boolean>('showApplyToast', { required: true })
const applyShadowControl = defineModel<boolean>('applyShadowControl', { required: true })

defineEmits<{
  (e: 'config-restored'): void
}>()
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionGeneral') }}</h1>
      <p>{{ t('settings.descGeneral') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">
        {{ t('settings.groupDisplayLanguage') }}
      </div>
      <div class="prop-body">
        <SettingsRow :label="t('settings.languageLabel')" :desc="t('settings.languageDesc')">
          <UiSelect
            v-model="language"
            width="140px"
            :options="[
              { value: 'ja', label: '日本語' },
              { value: 'en', label: 'English' },
            ]"
          />
        </SettingsRow>
      </div>
    </div>

    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupNotifications') }}</div>
      <div class="prop-body">
        <SettingsRow
          :label="t('settings.showApplyToastLabel')"
          :desc="t('settings.showApplyToastDesc')"
        >
          <SettingsToggle v-model="showApplyToast" />
        </SettingsRow>
        <SettingsRow
          :label="t('settings.applyShadowControlLabel')"
          :desc="t('settings.applyShadowControlDesc')"
        >
          <SettingsToggle v-model="applyShadowControl" />
        </SettingsRow>
      </div>
    </div>

    <!-- バックアップが存在する場合のみ復旧パネルを表示 -->
    <ConfigRecoveryPanel @restored="$emit('config-restored')" />
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
}
.prop-body {
  padding: 4px 16px;
}
</style>
