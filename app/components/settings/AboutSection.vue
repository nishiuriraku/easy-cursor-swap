<script setup lang="ts">
/**
 * 設定 → About セクション。
 *
 * バージョン情報・ホームページリンク・OSS ライセンス表示の固定 UI のみで、
 * 動的な状態を持たないので props/emit なしの完全独立コンポーネント。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionAbout') }}</h1>
      <p>{{ t('settings.descAbout') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">
        {{ t('app.name') }}
        <span class="head-hint">{{ t('settings.aboutAppHint', { version: '1.0.0' }) }}</span>
      </div>
      <div class="prop-body">
        <SettingsRow :label="t('settings.homepageLabel')" mono>
          <a
            class="btn ghost"
            href="https://github.com/nishiuriraku/easy-cursor-swap"
            target="_blank"
            rel="noopener"
          >
            <UiIcon name="Globe" :size="13" />github.com/nishiuriraku/easy-cursor-swap
          </a>
        </SettingsRow>
        <SettingsRow :label="t('settings.issuesLabel')" mono>
          <a
            class="btn ghost"
            href="https://github.com/nishiuriraku/easy-cursor-swap/issues"
            target="_blank"
            rel="noopener"
          >
            <UiIcon name="Alert" :size="13" />Issues
          </a>
        </SettingsRow>
        <SettingsRow :label="t('settings.ossLicenseLabel')">
          <button class="btn">{{ t('settings.btnView') }}</button>
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
.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}
.btn.ghost {
  background: transparent;
}
</style>
