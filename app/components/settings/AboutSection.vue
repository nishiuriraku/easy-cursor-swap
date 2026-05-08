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
            href="https://github.com/easycursorswap"
            target="_blank"
            rel="noopener"
          >
            <UiIcon name="Globe" :size="13" />github.com/easycursorswap
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
  text-decoration: none;
}

.btn.ghost {
  background: transparent;
}
</style>
