<script setup lang="ts">
/**
 * 設定 → アップデート セクション。
 *
 * 自動更新トグル + 自動チェック状態 + 確認ボタン + ダウンロード進捗。
 * 状態 (checking / downloading / available / error / progress) は親が `useUpdater()`
 * から受け取って渡す。autoCheckHint は親側で localStorage の last_check_at を見て計算した
 * 表示文字列。子は表示と emit 通知のみ。
 */

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

/** 更新内容モーダルの開閉。body 全文はモーダル内で pre-wrap 表示する。 */
const showNotes = ref(false)
</script>

<template>
  <section>
    <header class="section-head">
      <h1>{{ t('settings.sectionUpdates') }}</h1>
      <p>{{ t('settings.descUpdates') }}</p>
    </header>
    <div v-if="isMsixPackaged" class="msix-banner">
      <p>{{ t('settings.storeManaged') }}</p>
      <a href="ms-settings:apps-features-app">{{ t('settings.storeManagedLink') }}</a>
    </div>
    <div class="prop-section">
      <div class="prop-head">{{ t('settings.groupAutoUpdate') }}</div>
      <div class="prop-body">
        <SettingsRow
          anchor="autoUpdate"
          :label="t('settings.autoUpdateLabel')"
          :desc="t('settings.autoUpdateDesc')"
        >
          <SettingsToggle v-model="autoUpdate" :label="t('settings.autoUpdateLabel')" />
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
          <UiButton
            variant="primary"
            :loading="updaterChecking"
            :disabled="updaterDownloading"
            icon-left="Import"
            @click="$emit('check-update')"
          >
            {{ updaterChecking ? t('settings.btnChecking') : t('settings.btnCheckUpdate') }}
          </UiButton>
        </SettingsRow>
        <SettingsRow
          v-if="updaterAvailable"
          :label="
            t('settings.updateAvailableLabel', {
              version: updaterAvailable.version,
            })
          "
        >
          <template v-if="updaterAvailable.body" #desc>
            <button type="button" class="notes-link" @click="showNotes = true">
              {{ updaterAvailable.body }}
            </button>
          </template>
          <UiButton
            variant="primary"
            :loading="updaterDownloading"
            icon-left="Import"
            @click="$emit('download-update')"
          >
            {{
              updaterDownloading
                ? t('settings.btnDownloading', {
                    percent:
                      updaterTotal > 0 ? Math.round((updaterProgress / updaterTotal) * 100) : 0,
                  })
                : t('settings.btnDownloadInstall')
            }}
          </UiButton>
        </SettingsRow>
        <!-- LD7: ダウンロードの確定バー (バックエンドがバイト進捗を出すので確定モード)。
             contentLength 不明 (updaterTotal<=0) のときは UiProgress が自動で不確定アニメに倒す。 -->
        <UiProgress
          v-if="updaterDownloading"
          class="mt-3"
          :value="updaterProgress"
          :max="updaterTotal"
          :aria-label="t('settings.btnDownloadInstall')"
        />
        <div v-if="updaterMessage" class="profile-msg">
          {{ updaterMessage }}
        </div>
        <UiAlert v-if="updaterError" tone="danger" class="mt-2">
          {{ updaterError }}
        </UiAlert>
      </div>
    </div>
    <UiModal
      v-if="updaterAvailable"
      :open="showNotes"
      :title="t('settings.updateNotesTitle', { version: updaterAvailable.version })"
      icon="Import"
      size="md"
      @close="showNotes = false"
    >
      <UiMarkdown :source="updaterAvailable.body ?? ''" />
      <template #actions>
        <UiButton variant="ghost" @click="showNotes = false">{{ t('common.close') }}</UiButton>
      </template>
    </UiModal>
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
/* 更新内容の1行省略リンク。クリックで全文モーダルを開く。 */
.notes-link {
  @apply mt-[3px] block w-full border-0 bg-transparent p-0 text-left text-[11.5px] leading-[1.5] text-fg-mute;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
}
.notes-link:hover {
  color: rgb(var(--accent));
  text-decoration: underline;
  text-underline-offset: 2px;
}
.msix-banner {
  @apply mb-3 rounded-[10px] border px-4 py-3 text-[13px];
  background: rgba(255, 213, 79, 0.08);
  border-color: rgba(255, 213, 79, 0.4);
}
.msix-banner p {
  @apply m-0 mb-1;
}
.msix-banner a {
  color: rgb(var(--accent));
}
</style>
