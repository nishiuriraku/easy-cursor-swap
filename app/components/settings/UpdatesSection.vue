<script setup lang="ts">
/**
 * 設定 → アップデート セクション。
 *
 * 自動更新トグル + チャンネル選択 + 確認ボタン + ダウンロード進捗。
 * 状態 (checking / downloading / available / error / progress) は親が `useUpdater()`
 * から受け取って渡す。子は表示と emit 通知のみ。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const autoUpdate = defineModel<boolean>('autoUpdate', { required: true })
const channel = defineModel<string>('channel', { required: true })

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
}>()

defineEmits<{
  (e: 'check-update'): void
  (e: 'download-update'): void
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
        <SettingsRow :label="t('settings.autoUpdateLabel')" :desc="t('settings.autoUpdateDesc')">
          <SettingsToggle v-model="autoUpdate" />
        </SettingsRow>
        <SettingsRow :label="t('settings.channelLabel')" :desc="t('settings.channelDesc')">
          <UiSelect
            v-model="channel"
            width="140px"
            :options="[
              { value: 'stable', label: 'stable' },
              { value: 'beta', label: 'beta' },
            ]"
          />
        </SettingsRow>
        <SettingsRow :label="t('settings.checkNowLabel')">
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
  padding: 4px 16px 12px;
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
.btn.primary {
  background: var(--accent);
  color: var(--bg-base);
  border-color: var(--accent);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.profile-msg {
  margin-top: 8px;
  padding: 8px 12px;
  font-size: 12px;
  border-radius: 8px;
  background: rgba(106, 213, 184, 0.06);
  border: 1px solid rgba(106, 213, 184, 0.4);
  color: var(--mint);
}
.spinner {
  display: inline-block;
  width: 13px;
  height: 13px;
  border: 2px solid var(--text-mute);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 800ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
