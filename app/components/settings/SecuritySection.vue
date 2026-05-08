<script setup lang="ts">
/**
 * 設定 → セキュリティ セクション。
 * 署名検証関連の 2 トグル。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const requireSignedThemes = defineModel<boolean>('requireSignedThemes', { required: true })
const warnUnsignedImport = defineModel<boolean>('warnUnsignedImport', { required: true })
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionSecurity') }}</h1>
      <p>{{ t('settings.descSecurity') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupThemeVerify') }}</div>
      <div class="prop-body">
        <SettingsRow
          :label="t('settings.requireSignedLabel')"
          :desc="t('settings.requireSignedDesc')"
        >
          <SettingsToggle v-model="requireSignedThemes" />
        </SettingsRow>
        <SettingsRow
          :label="t('settings.warnUnsignedLabel')"
          :desc="t('settings.warnUnsignedDesc')"
        >
          <SettingsToggle v-model="warnUnsignedImport" />
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
</style>
