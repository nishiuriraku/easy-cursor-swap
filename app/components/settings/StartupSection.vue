<script setup lang="ts">
/**
 * 設定 → 起動・常駐 セクション。
 * 自動起動 / 最小化起動の 2 つのトグルのみ。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const autoStart = defineModel<boolean>('autoStart', { required: true })
const startMinimized = defineModel<boolean>('startMinimized', { required: true })
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionStartup') }}</h1>
      <p>{{ t('settings.descStartup') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">
        {{ t('settings.groupAutoStart') }}
        <span class="head-hint">{{ t('settings.autoStartHint') }}</span>
      </div>
      <div class="prop-body">
        <SettingsRow :label="t('settings.autoStartLabel')" :desc="t('settings.autoStartDesc')">
          <SettingsToggle v-model="autoStart" />
        </SettingsRow>
        <SettingsRow
          :label="t('settings.startMinimizedLabel')"
          :desc="t('settings.startMinimizedDesc')"
        >
          <SettingsToggle v-model="startMinimized" />
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
  padding: 4px 16px;
}
</style>
