<script setup lang="ts">
/**
 * 設定 → ログ・診断 セクション。
 *
 * 2 ブロック構成:
 *  1. ログ出力 — ログレベル / 保持期間 / 合計上限 / フォルダを開く
 *  2. クラッシュレポート — opt-in トグル / 保存件数表示 / 送信ボタン / クリアボタン
 *
 * ログ出力の Open ボタンは `open_log_folder` IPC 経由で `logging::log_dir()` を
 * Windows Explorer で開く。
 * クラッシュ系の親子契約は `keystoreInfo` パターンと同じ: 状態 (件数 / メッセージ)
 * は親が `useAppSettings` から渡し、子は表示と emit のみ。
 */
import { useI18n } from '~/composables/useI18n'
import { invokeTauri } from '~/composables/useTauri'

const { t } = useI18n()

const logLevel = defineModel<string>('logLevel', { required: true })
const retentionDays = defineModel<number>('retentionDays', { required: true })
const maxSizeMb = defineModel<number>('maxSizeMb', { required: true })
const crashReporting = defineModel<boolean>('crashReporting', { required: true })

defineProps<{
  crashReportsCount: number
  crashBusy: boolean
  crashMessage: string | null
}>()

defineEmits<{
  (e: 'submit-crash'): void
  (e: 'clear-crash'): void
}>()

async function openLogFolder() {
  try {
    await invokeTauri<void>('open_log_folder')
  } catch (err) {
    // Tauri 未接続 (web preview) では何もできない。コンソールに warning を出して握る。
    console.error('open_log_folder failed:', err)
  }
}
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionLogging') }}</h1>
      <p>{{ t('settings.descLogging') }}</p>
    </header>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupLogOutput') }}</div>
      <div class="prop-body">
        <SettingsRow
          anchor="logLevel"
          :label="t('settings.logLevelLabel')"
          :desc="t('settings.logLevelDesc')"
        >
          <UiSelect
            v-model="logLevel"
            width="140px"
            :options="[
              { value: 'TRACE', label: 'TRACE' },
              { value: 'DEBUG', label: 'DEBUG' },
              { value: 'INFO', label: 'INFO' },
              { value: 'WARN', label: 'WARN' },
              { value: 'ERROR', label: 'ERROR' },
            ]"
          />
        </SettingsRow>
        <SettingsRow
          anchor="retention"
          :label="t('settings.retentionLabel')"
          :desc="t('settings.retentionDesc')"
        >
          <input
            v-model.number="retentionDays"
            type="number"
            class="input"
            min="1"
            max="365"
            style="width: 80px"
          />
        </SettingsRow>
        <SettingsRow
          anchor="maxSize"
          :label="t('settings.maxSizeLabel')"
          :desc="t('settings.maxSizeDesc')"
        >
          <input
            v-model.number="maxSizeMb"
            type="number"
            class="input"
            min="10"
            max="2048"
            style="width: 80px"
          />
        </SettingsRow>
        <SettingsRow
          anchor="openLogFolder"
          :label="t('settings.openLogFolderLabel')"
          :desc="t('settings.openLogFolderDesc')"
        >
          <button class="btn" @click="openLogFolder">
            <UiIcon name="Globe" :size="13" />{{ t('settings.btnOpen') }}
          </button>
        </SettingsRow>
      </div>
    </div>

    <div class="prop-section" style="margin-top: 12px">
      <div class="prop-head">
        {{ t('settings.groupCrashReports') }}
        <span class="head-hint">{{ t('settings.crashReportsHint') }}</span>
      </div>
      <div class="prop-body">
        <SettingsRow
          anchor="crashReporting"
          :label="t('settings.crashReportingLabel')"
          :desc="t('settings.crashReportingDesc')"
        >
          <SettingsToggle v-model="crashReporting" />
        </SettingsRow>
        <SettingsRow
          anchor="crashCount"
          :label="t('settings.crashReportsCountLabel')"
          :desc="
            crashReportsCount === 0
              ? t('settings.crashReportsEmptyDesc')
              : t('settings.crashReportsCountDesc', { count: crashReportsCount })
          "
        >
          <span class="tag" :class="{ ok: crashReportsCount === 0 }">{{ crashReportsCount }}</span>
        </SettingsRow>
        <SettingsRow
          anchor="submitCrash"
          :label="t('settings.crashReportSubmitLabel')"
          :desc="t('settings.crashReportSubmitDesc')"
        >
          <button
            class="btn"
            :disabled="crashBusy || crashReportsCount === 0"
            @click="$emit('submit-crash')"
          >
            <span v-if="crashBusy" class="spinner" style="width: 13px; height: 13px" />
            <UiIcon v-else name="Import" :size="13" />{{ t('settings.btnSubmit') }}
          </button>
        </SettingsRow>
        <SettingsRow
          anchor="clearCrash"
          :label="t('settings.crashReportClearLabel')"
          :desc="t('settings.crashReportClearDesc')"
        >
          <button
            class="btn danger"
            :disabled="crashBusy || crashReportsCount === 0"
            @click="$emit('clear-crash')"
          >
            <UiIcon name="X" :size="13" />{{ t('settings.btnClear') }}
          </button>
        </SettingsRow>
        <div v-if="crashMessage" class="profile-msg">
          {{ crashMessage }}
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
  padding: 4px 16px 12px;
}
.input {
  height: 32px;
  border-radius: 8px;
  padding: 0 10px;
  font-size: 13px;
}
.btn {
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
}
.btn.danger {
  @apply text-rose;
  border-color: rgba(255, 107, 138, 0.3);
}
.btn:disabled {
  @apply cursor-not-allowed opacity-50;
}
.tag {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-family: var(--font-mono);
}
.tag.ok {
  border-color: rgba(106, 213, 184, 0.3);
  color: var(--fg-mute);
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
