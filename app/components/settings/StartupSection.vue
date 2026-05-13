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
        <SettingsRow
          anchor="autoStart"
          :label="t('settings.autoStartLabel')"
          :desc="t('settings.autoStartDesc')"
        >
          <SettingsToggle v-model="autoStart" />
        </SettingsRow>
        <SettingsRow
          anchor="startMinimized"
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
@reference '~/assets/css/tailwind.css';

/* NOTE: --border / --bg-elev1 / --text-mute は未定義トークン (元コードの leftover)。
 * .prop-section / .prop-head / .prop-body は global.css にも同名ルールがあり、
 * scoped の dead 行は discarded → global が cascade で効いていた。
 * scoped はレイアウト/スペーシングの差分のみを保持する。 */

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
  padding: 4px 16px;
}
</style>
