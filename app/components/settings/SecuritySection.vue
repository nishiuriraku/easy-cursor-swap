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
</style>
