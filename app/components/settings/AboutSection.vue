<script setup lang="ts">
/**
 * 設定 → About セクション。
 *
 * バージョン情報・ホームページリンク・OSS ライセンス表示の固定 UI のみで、
 * 動的な状態を持たないので props/emit なしの完全独立コンポーネント。
 *
 * 外部 URL を開く際は Tauri 2 webview の制限上 `<a href target=_blank>` では動かない。
 * Rust 側 `open_url` IPC (内部で Win32 ShellExecuteW) を経由してホスト OS の
 * ブラウザを起動する。SubmitThemeDialog / marketplace.vue と同じパターン。
 */

const { t } = useI18n()

const ossOpen = ref(false)

// 実バージョンは Rust 側 get_app_info (= env!("CARGO_PKG_VERSION")) から取得する。
// Tauri 未接続時は info.value が null のままなので '—' で fallback。
const { info: appInfo, load: loadAppInfo } = useAppInfo()
onMounted(() => {
  void loadAppInfo()
})

// 外部 URL は openExternalUrl が open_url IPC + window.open フォールバックを担う。
const openExternal = openExternalUrl
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
        <span class="head-hint">{{
          t('settings.aboutAppHint', { version: appInfo?.version ?? '—' })
        }}</span>
      </div>
      <div class="prop-body">
        <SettingsRow anchor="homepage" :label="t('settings.homepageLabel')" mono>
          <button
            class="btn ghost"
            type="button"
            @click="openExternal('https://github.com/nishiuriraku/easy-cursor-swap')"
          >
            <UiIcon name="Globe" :size="13" />github.com/nishiuriraku/easy-cursor-swap
          </button>
        </SettingsRow>
        <SettingsRow anchor="issues" :label="t('settings.issuesLabel')" mono>
          <button
            class="btn ghost"
            type="button"
            @click="
              openExternal('https://github.com/nishiuriraku/easy-cursor-swap/issues/new/choose')
            "
          >
            <UiIcon name="Alert" :size="13" />{{ t('settings.btnReportIssue') }}
          </button>
        </SettingsRow>
        <SettingsRow anchor="ossLicense" :label="t('settings.ossLicenseLabel')">
          <button class="btn" type="button" @click="ossOpen = true">
            {{ t('settings.btnView') }}
          </button>
        </SettingsRow>
      </div>
    </div>
    <OssLicenseModal :open="ossOpen" @close="ossOpen = false" />
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
