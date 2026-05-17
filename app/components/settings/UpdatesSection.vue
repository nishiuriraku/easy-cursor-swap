<script setup lang="ts">
/**
 * 設定 → アップデート セクション。
 *
 * 自動更新トグル + 自動チェック状態 + 確認ボタン + ダウンロード進捗。
 * 状態 (checking / downloading / available / error / progress) は親が `useUpdater()`
 * から受け取って渡す。autoCheckHint は親側で localStorage の last_check_at を見て計算した
 * 表示文字列。子は表示と emit 通知のみ。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const autoUpdate = defineModel<boolean>('autoUpdate', { required: true })

interface AvailableInfo {
  version: string
  body?: string | null
}

defineProps<{
  updaterChecking: boolean
  updaterDownloading: boolean
  updaterAvailable: AvailableInfo | null
  updaterMessage: string | null
  updaterError: string | null
  updaterProgress: number
  updaterTotal: number
  autoCheckHint: string
}>()

defineEmits<{
  (e: 'check-update'): void
  (e: 'download-update'): void
  (e: 'force-recheck'): void
}>()
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionUpdates') }}</h1>
      <p>{{ t('settings.descUpdates') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupAutoUpdate') }}</div>
      <div class="prop-body">
        <SettingsRow
          anchor="autoUpdate"
          :label="t('settings.autoUpdateLabel')"
          :desc="t('settings.autoUpdateDesc')"
        >
          <SettingsToggle v-model="autoUpdate" />
        </SettingsRow>
        <SettingsRow
          anchor="autoCheckStatus"
          :label="t('settings.autoCheckStatus')"
          :desc="autoCheckHint"
        >
          <button class="btn" @click="$emit('force-recheck')">
            <UiIcon name="Refresh" :size="13" />
            {{ t('settings.btnForceRecheck') }}
          </button>
        </SettingsRow>
        <SettingsRow anchor="checkNow" :label="t('settings.checkNowLabel')">
          <button
            class="btn"
            :disabled="updaterChecking || updaterDownloading"
            @click="$emit('check-update')"
          >
            <span v-if="updaterChecking" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Import" :size="13" />
            {{ updaterChecking ? t('settings.btnChecking') : t('settings.btnCheckUpdate') }}
          </button>
        </SettingsRow>
        <SettingsRow
          v-if="updaterAvailable"
          :label="
            t('settings.updateAvailableLabel', {
              version: updaterAvailable.version,
            })
          "
          :desc="updaterAvailable.body ?? ''"
        >
          <button
            class="btn primary"
            :disabled="updaterDownloading"
            @click="$emit('download-update')"
          >
            <span v-if="updaterDownloading" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Import" :size="13" />
            {{
              updaterDownloading
                ? t('settings.btnDownloading', {
                    percent:
                      updaterTotal > 0 ? Math.round((updaterProgress / updaterTotal) * 100) : 0,
                  })
                : t('settings.btnDownloadInstall')
            }}
          </button>
        </SettingsRow>
        <div v-if="updaterMessage" class="profile-msg">
          {{ updaterMessage }}
        </div>
        <div
          v-if="updaterError"
          class="profile-msg"
          style="
            background: rgba(255, 107, 138, 0.06);
            border-color: rgba(255, 107, 138, 0.4);
            color: #ffb8c5;
          "
        >
          {{ updaterError }}
        </div>
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
  padding: 4px 16px 12px;
}
.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}
.btn:disabled {
  @apply cursor-not-allowed opacity-50;
}
.profile-msg {
  @apply mt-2 rounded-[8px] border px-3 py-2 text-[12px];
  background: rgba(106, 213, 184, 0.06);
  border-color: rgba(106, 213, 184, 0.4);
}
.spinner {
  @apply inline-block size-[13px] rounded-full;
  border: 2px solid var(--fg-mute);
  border-top-color: transparent;
  animation: spin 800ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
