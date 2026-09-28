<script setup lang="ts">
/**
 * 一般設定ページ (Phase 5-7)
 *
 * design/general-settings.jsx を Vue 化したもの。
 * 8 セクション切替の左サイドナビ + 各セクションのフォーム UI。
 *
 * NOTE: 当 SFC 内で各セクションをインライン定義 (各セクションが独立した
 *       重い状態を持たないため SFC 分割するメリットが薄い)。
 *       将来セクションが肥大化したら個別 SFC に切り出す。
 */
import type { GithubAccount } from '~/types/githubAuth'

const { t } = useI18n()

type SectionId =
  | 'general'
  | 'startup'
  | 'library'
  | 'security'
  | 'keys'
  | 'logging'
  | 'updates'
  | 'about'

interface SectionDef {
  id: SectionId
  labelKey: string
  icon: string
}

const SECTIONS: SectionDef[] = [
  { id: 'general', labelKey: 'settings.sectionGeneral', icon: 'Settings' },
  { id: 'startup', labelKey: 'settings.sectionStartup', icon: 'Logo' },
  { id: 'library', labelKey: 'settings.sectionLibrary', icon: 'Library' },
  { id: 'security', labelKey: 'settings.sectionSecurity', icon: 'Shield' },
  { id: 'keys', labelKey: 'settings.sectionKeys', icon: 'Pkg' },
  { id: 'logging', labelKey: 'settings.sectionLogging', icon: 'Sort' },
  { id: 'updates', labelKey: 'settings.sectionUpdates', icon: 'Import' },
  { id: 'about', labelKey: 'settings.sectionAbout', icon: 'Globe' },
]

const section = ref<SectionId>('general')

const { config: appConfig, load: loadConfig } = useAppSettings()

/**
 * 設定フォーム (P08a S1: useSettingsForm に集約)。
 */
const {
  general,
  startup,
  library,
  security,
  logging,
  updates,
  githubAccount,
  dirty,
  saving,
  saveError,
  applyConfigToLocal,
  save,
  discardChanges,
} = useSettingsForm()
/**
 * カーソルサイズ設定 (P08a S2: useCursorSizeSettings に集約)。
 * OS 即時反映のため dirty/save フローには乗せない。
 */
const {
  cursorSizeSlider,
  cursorSizeBusy,
  cursorSizeError,
  cursorSizeSliderRaw,
  cursorTypeRaw,
  cursorAccessibilityActive,
  CURSOR_SIZE_MIN_SLIDER,
  CURSOR_SIZE_MAX_SLIDER,
  sliderToDword,
  onCursorSizeCommit,
  onRefreshCursorSizeFromOs,
  onOpenWindowsCursorSettings,
} = useCursorSizeSettings()
/**
 * キーストア操作 (P08a S3: useKeystoreSettingsActions に集約)。
 */
const {
  keystoreInfo,
  keystoreBusy,
  keystoreError,
  keystoreMessage,
  passphrasePrompt,
  refreshKeystore,
  onKeystoreGenerate,
  onKeystoreRegenerate,
  onKeystoreExport,
  onKeystoreImport,
  onKeystoreDelete,
  onPassphraseConfirm,
} = useKeystoreSettingsActions({ t })

/**
 * アップデータ設定 (P08a S4: useUpdaterSettings に集約)。
 */
const {
  updaterChecking,
  updaterDownloading,
  updaterAvailable,
  updaterMessage,
  updaterErrorDisplay,
  updaterProgress,
  updaterTotal,
  autoCheckHint,
  onForceRecheck,
  onCheckUpdate,
  onDownloadUpdate,
} = useUpdaterSettings({ t })

// クラッシュレポート (LoggingSection 用、P08a S5: useCrashReportsState に集約)。
// 件数は `list_crash_reports` の戻り長、メッセージは送信/クリア後の
// ユーザー向けトースト相当の文字列。
const {
  count: crashReportsCount,
  busy: crashBusy,
  message: crashMessage,
  load: loadCrashReports,
  submit: onSubmitCrashReports,
  clear: onClearCrashReports,
} = useCrashReportsState({ t, isOptedIn: () => general.value.crashReporting })

const {
  busy: profileBusy,
  message: profileMessage,
  exportWithDialog,
  importWithDialog,
} = useProfileBackupDialog({ t })

async function exportProfile() {
  await exportWithDialog()
}

async function importProfile() {
  const needReload = await importWithDialog()
  if (needReload) {
    // 設定の再読み込み
    await loadConfig()
    applyConfigToLocal()
  }
}

// GitHub 連携解除 (Wave 3A / L1-5): 直 invoke 廃止 → useGithubAuth.revoke() 経由。
const { revoke: revokeGithubLink } = useGithubAuth()

async function onGithubUnlink() {
  // useGithubAuth.revoke() は `revoke_github_link` IPC の薄いラッパー。
  // 失敗時は throw されるので caller (このハンドラ) が後処理 (loadConfig 等) を
  // 行うか判断できる。
  await revokeGithubLink()
  // useAppSettings の load() は force=false 既定でキャッシュを返すため、
  // revoke 後にフロント ref に古い github_account が残ってしまう。force=true で再取得する。
  await loadConfig(true)
  applyConfigToLocal()
}

async function onConfigRestored() {
  // バックアップから復旧後: 設定を再読み込みして UI に反映
  await loadConfig()
  applyConfigToLocal()
}

const { replay: replayOnboarding } = useOnboarding()
function onReplayOnboarding() {
  void replayOnboarding()
}

// 設定検索コンテキスト (P08a V4: SettingsSearchBox に渡す)。
const searchContext = computed(() => ({
  hasKeystore: keystoreInfo.value?.has_keypair ?? false,
}))

onMounted(async () => {
  await loadConfig()
  applyConfigToLocal()
  await refreshKeystore()
  await loadCrashReports()
  // 起動時の同期完了を watch で検出してローカル参照に反映
  watch(appConfig, applyConfigToLocal)
})

const currentSection = computed(() => SECTIONS.find((s) => s.id === section.value) ?? SECTIONS[0]!)

function selectSection(id: SectionId) {
  section.value = id
}
</script>

<template>
  <div class="settings-host">
    <!-- ツールバー -->
    <div class="toolbar">
      <div class="bcrumb">
        <span class="crumb">{{ t('settings.breadcrumb') }}</span>
        <span class="sep">/</span>
        <span class="crumb active">{{ t(currentSection.labelKey) }}</span>
      </div>
      <SettingsSearchBox v-model:section="section" :context="searchContext" />
      <div class="tb-actions">
        <UiButton variant="ghost" :disabled="!dirty || saving" @click="discardChanges">
          {{ t('common.discard') }}
        </UiButton>
        <UiButton
          variant="primary"
          :loading="saving"
          :disabled="!dirty"
          icon-left="Check"
          @click="save"
        >
          {{ saving ? t('common.saving') : t('common.save') }}
        </UiButton>
      </div>
    </div>

    <!-- 2 カラム: 設定サイドナビ + コンテンツ -->
    <div class="settings-grid">
      <nav class="settings-sidenav" :aria-label="t('settings.navTitle')">
        <h6 class="nav-title" aria-hidden="true">
          {{ t('settings.navTitle') }}
        </h6>
        <button
          v-for="s in SECTIONS"
          :key="s.id"
          :class="['nav-item', { active: section === s.id }]"
          :aria-current="section === s.id ? 'page' : undefined"
          @click="selectSection(s.id)"
        >
          <UiIcon :name="s.icon" aria-hidden="true" />
          <span>{{ t(s.labelKey) }}</span>
        </button>
      </nav>

      <div class="settings-content" data-settings-scroll>
        <!-- 一般 -->
        <GeneralSection
          v-if="section === 'general'"
          v-model:language="general.language"
          v-model:show-apply-toast="general.showApplyToast"
          v-model:apply-shadow-control="general.applyShadowControl"
          :cursor-size-slider="cursorSizeSlider"
          :cursor-size-min="CURSOR_SIZE_MIN_SLIDER"
          :cursor-size-max="CURSOR_SIZE_MAX_SLIDER"
          :cursor-size-px="sliderToDword(cursorSizeSlider)"
          :cursor-size-busy="cursorSizeBusy"
          :cursor-size-error="cursorSizeError"
          :cursor-accessibility-active="cursorAccessibilityActive"
          :cursor-current-windows-slider="cursorSizeSliderRaw"
          :cursor-current-windows-type="cursorTypeRaw"
          @update:cursor-size-slider="onCursorSizeCommit"
          @refresh-cursor-size-from-os="onRefreshCursorSizeFromOs"
          @open-windows-cursor-settings="onOpenWindowsCursorSettings"
          @replay-onboarding="onReplayOnboarding"
          @config-restored="onConfigRestored"
        />

        <StartupSection
          v-else-if="section === 'startup'"
          v-model:auto-start="startup.autoStart"
          v-model:start-minimized="startup.startMinimized"
          :is-msix-packaged="isMsixPackaged"
        />

        <LibrarySection
          v-else-if="section === 'library'"
          v-model:total-limit-warn-gb="library.totalLimitWarnGb"
          v-model:storage-warn-enabled="library.storageWarnEnabled"
          :profile-busy="profileBusy"
          :profile-message="profileMessage"
          @export-profile="exportProfile"
          @import-profile="importProfile"
        />

        <SecuritySection
          v-else-if="section === 'security'"
          v-model:require-signed-themes="security.requireSignedThemes"
          v-model:warn-unsigned-import="security.warnUnsignedImport"
        />

        <KeysSection
          v-else-if="section === 'keys'"
          :keystore-info="keystoreInfo"
          :keystore-busy="keystoreBusy"
          :keystore-error="keystoreError"
          :keystore-message="keystoreMessage"
          :github-account="githubAccount"
          @generate="onKeystoreGenerate"
          @regenerate="onKeystoreRegenerate"
          @delete="onKeystoreDelete"
          @export="onKeystoreExport"
          @import="onKeystoreImport"
          @github-unlink="onGithubUnlink"
        />

        <LoggingSection
          v-else-if="section === 'logging'"
          v-model:log-level="logging.logLevel"
          v-model:retention-days="logging.retentionDays"
          v-model:max-size-mb="logging.maxSizeMb"
          v-model:crash-reporting="general.crashReporting"
          :crash-reports-count="crashReportsCount"
          :crash-busy="crashBusy"
          :crash-message="crashMessage"
          @submit-crash="onSubmitCrashReports"
          @clear-crash="onClearCrashReports"
        />

        <!-- アップデート -->
        <UpdatesSection
          v-else-if="section === 'updates'"
          v-model:auto-update="updates.autoUpdate"
          :auto-check-hint="autoCheckHint"
          :updater-checking="updaterChecking"
          :updater-downloading="updaterDownloading"
          :updater-available="updaterAvailable"
          :updater-message="updaterMessage"
          :updater-error="updaterErrorDisplay"
          :updater-progress="updaterProgress"
          :updater-total="updaterTotal"
          :is-msix-packaged="isMsixPackaged"
          @check-update="onCheckUpdate"
          @download-update="onDownloadUpdate"
          @force-recheck="onForceRecheck"
        />

        <!-- About -->
        <AboutSection v-else />
      </div>
    </div>

    <PassphrasePrompt
      :open="passphrasePrompt.open"
      :mode="passphrasePrompt.mode"
      @update:open="passphrasePrompt.open = $event"
      @confirm="onPassphraseConfirm"
    />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.settings-host {
  @apply flex h-full flex-col;
}

.settings-grid {
  @apply grid min-h-0 flex-1 grid-cols-[220px_1fr];
}

.settings-sidenav {
  @apply flex flex-col gap-0.5 overflow-y-auto border-r border-line bg-white/[0.01] px-2.5 py-4;
}
.nav-title {
  @apply mb-2.5 ml-2 mr-2 mt-0 font-mono text-[9.5px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}

.settings-content {
  @apply overflow-y-auto px-7 pb-8 pt-6;
}
/* (P08a: .section-head/.head-hint/.prop-body/.profile-msg は各 Section
 * コンポーネント側に同等スタイルがあるためページ側の複製を削除) */
</style>
