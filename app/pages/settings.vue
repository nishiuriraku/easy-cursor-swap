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
import { computed, onMounted, ref, watch } from 'vue'
import { useAppSettings } from '~/composables/useAppSettings'
import { useKeystore } from '~/composables/useKeystore'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'
import { useUpdater } from '~/composables/useUpdater'

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
const searchQuery = ref('')

const { config: appConfig, load: loadConfig, update: persistConfig } = useAppSettings()

// バイト ⇄ GB / MB 変換ユーティリティ
const BYTES_PER_GB = 1024 * 1024 * 1024
const BYTES_PER_MB = 1024 * 1024

// UI 用ローカル ref。`appConfig` (Rust 側) との双方向同期を watch で実現する。
const general = ref({
  language: 'ja' as 'ja' | 'en' | 'auto',
  applyShadowControl: true,
  showApplyToast: true,
  hideMainOnLaunch: false,
})
const startup = ref({
  autoStart: true,
  startMinimized: true,
})
const library = ref({
  totalLimitWarnGb: 1,
  storageWarnEnabled: true,
})
const security = ref({
  requireSignedThemes: false,
  warnUnsignedImport: true,
})
const {
  info: keystoreInfo,
  busy: keystoreBusy,
  lastError: keystoreError,
  refresh: refreshKeystore,
  generate: generateKeystore,
  remove: removeKeystore,
  exportPrivate: exportPrivateKey,
  importPrivate: importPrivateKey,
} = useKeystore()

const {
  checking: updaterChecking,
  downloading: updaterDownloading,
  available: updaterAvailable,
  error: updaterError,
  progressBytes: updaterProgress,
  totalBytes: updaterTotal,
  check: checkForUpdate,
  downloadAndInstall: downloadUpdate,
  relaunch: relaunchApp,
} = useUpdater()
const updaterMessage = ref<string | null>(null)

// 利用可能なアップデート情報 (メジャー跨ぎ判定に使用)
const pendingUpdateVersion = ref<string | null>(null)

async function onCheckUpdate() {
  updaterMessage.value = null
  pendingUpdateVersion.value = null
  const info = await checkForUpdate()
  if (info) {
    pendingUpdateVersion.value = info.version
    updaterMessage.value = t('settings.updateNewVersion', {
      version: info.version,
      current: info.currentVersion,
    })
  } else {
    updaterMessage.value = t('settings.updateUpToDate')
  }
}

async function onDownloadUpdate() {
  updaterMessage.value = null

  // メジャーバージョン跨ぎ確認
  if (pendingUpdateVersion.value) {
    const appInfo = await invokeTauri<{ version: string }>('get_app_info')
    const isMajorJump = await invokeTauri<boolean>('check_update_is_major_jump', {
      currentVersion: appInfo.version,
      newVersion: pendingUpdateVersion.value,
    })
    if (isMajorJump) {
      const { ask } = await import('@tauri-apps/plugin-dialog')
      const proceed = await ask(
        t('settings.updateMajorJumpWarning', {
          version: pendingUpdateVersion.value,
        }),
        { title: t('settings.updateMajorJumpTitle'), kind: 'warning' },
      )
      if (!proceed) return
    }
  }

  const ok = await downloadUpdate()
  if (ok) {
    updaterMessage.value = t('settings.updateDownloadComplete')
    const { ask } = await import('@tauri-apps/plugin-dialog')
    const restart = await ask(t('settings.updateRelaunchAsk'), {
      title: t('settings.updateRelaunchTitle'),
      kind: 'info',
    })
    if (restart) await relaunchApp()
  }
}

// パスフレーズプロンプト制御
const passphrasePrompt = ref<{ mode: 'export' | 'import'; open: boolean }>({
  mode: 'export',
  open: false,
})
const keystoreMessage = ref<string | null>(null)
const logging = ref({
  logLevel: 'INFO' as 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR',
  retentionDays: 14,
  maxSizeMb: 100,
})
const updates = ref({
  autoUpdate: true,
  channel: 'stable' as 'stable' | 'beta',
})

const dirty = ref(false)
const saving = ref(false)
const saveError = ref<string | null>(null)

/** appConfig (Rust 側) → UI ローカル ref へ反映 */
function applyConfigToLocal() {
  const c = appConfig.value
  if (!c) return
  general.value.language = (c.general.language as 'ja' | 'en' | 'auto') ?? 'auto'
  startup.value.autoStart = c.general.auto_start
  updates.value.autoUpdate = c.general.auto_update

  library.value.totalLimitWarnGb = c.security.storage_warning_threshold / BYTES_PER_GB

  logging.value.logLevel = (c.logging.level as typeof logging.value.logLevel) ?? 'INFO'
  logging.value.retentionDays = c.logging.retention_days
  logging.value.maxSizeMb = c.logging.max_total_size / BYTES_PER_MB

  dirty.value = false
}

/** UI ローカル ref → appConfig 形状にコピー */
function flushLocalToConfig() {
  return persistConfig((draft) => {
    draft.general.language = general.value.language
    draft.general.auto_start = startup.value.autoStart
    draft.general.auto_update = updates.value.autoUpdate

    draft.security.storage_warning_threshold = Math.round(
      library.value.totalLimitWarnGb * BYTES_PER_GB,
    )

    draft.logging.level = logging.value.logLevel
    draft.logging.retention_days = logging.value.retentionDays
    draft.logging.max_total_size = Math.round(logging.value.maxSizeMb * BYTES_PER_MB)
  })
}

async function save() {
  saving.value = true
  saveError.value = null
  try {
    await flushLocalToConfig()
    dirty.value = false
  } catch (err) {
    saveError.value = err instanceof Error ? err.message : String(err)
  } finally {
    saving.value = false
  }
}

function discardChanges() {
  applyConfigToLocal()
}

const profileBusy = ref(false)
const profileMessage = ref<string | null>(null)

async function exportProfile() {
  profileBusy.value = true
  profileMessage.value = null
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const today = new Date().toISOString().slice(0, 10)
    const target = await save({
      defaultPath: `easycursorswap-${today}.cursorprofile`,
      filters: [{ name: 'EasyCursorSwap Profile', extensions: ['cursorprofile'] }],
    })
    if (!target) return
    await invokeTauri<void>('export_profile', { path: target })
    profileMessage.value = t('settings.profileExportSuccess', { target })
  } catch (err) {
    profileMessage.value = t('settings.profileExportFail', {
      error: err instanceof Error ? err.message : String(err),
    })
  } finally {
    profileBusy.value = false
  }
}

async function importProfile() {
  profileBusy.value = true
  profileMessage.value = null
  try {
    const { open, ask } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: false,
      filters: [{ name: 'EasyCursorSwap Profile', extensions: ['cursorprofile'] }],
    })
    if (!selected || Array.isArray(selected)) return
    const overwrite = await ask(t('settings.profileImportAskMsg'), {
      title: t('settings.profileImportAskTitle'),
      kind: 'warning',
    })
    await invokeTauri<unknown>('import_profile', {
      path: selected,
      merge: !overwrite,
    })
    profileMessage.value = t('settings.profileImportSuccess', {
      target: selected,
    })
    // 設定の再読み込み
    await loadConfig()
    applyConfigToLocal()
  } catch (err) {
    profileMessage.value = t('settings.profileImportFail', {
      error: err instanceof Error ? err.message : String(err),
    })
  } finally {
    profileBusy.value = false
  }
}

async function onKeystoreGenerate() {
  await generateKeystore(false)
}
async function onKeystoreRegenerate() {
  // 既存鍵を上書き再生成。ユーザーには事前に dialog::ask で確認。
  const { ask } = await import('@tauri-apps/plugin-dialog')
  const proceed = await ask(t('settings.askRegenerateMsg'), {
    title: t('settings.askRegenerateTitle'),
    kind: 'warning',
  })
  if (proceed) await generateKeystore(true)
}
async function onPassphraseConfirm(passphrase: string) {
  const mode = passphrasePrompt.value.mode
  keystoreMessage.value = null
  if (mode === 'export') {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const today = new Date().toISOString().slice(0, 10)
    const target = await save({
      defaultPath: `easycursorswap-key-${today}.cfkey`,
      filters: [{ name: 'EasyCursorSwap Key', extensions: ['cfkey'] }],
    })
    if (!target) return
    const written = await exportPrivateKey(passphrase, target)
    if (written !== null) {
      keystoreMessage.value = t('settings.keyExportSuccess', {
        size: written,
        target,
      })
    }
  } else {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: false,
      filters: [{ name: 'EasyCursorSwap Key', extensions: ['cfkey'] }],
    })
    if (!selected || Array.isArray(selected)) return
    const result = await importPrivateKey(passphrase, selected)
    if (result) {
      keystoreMessage.value = t('settings.keyImportSuccess', {
        keyId: result.key_id ?? '?',
      })
    }
  }
}

function onKeystoreExport() {
  passphrasePrompt.value = { mode: 'export', open: true }
}

function onKeystoreImport() {
  passphrasePrompt.value = { mode: 'import', open: true }
}

async function onKeystoreDelete() {
  const { ask } = await import('@tauri-apps/plugin-dialog')
  const proceed = await ask(t('settings.askDeleteMsg'), {
    title: t('settings.askDeleteTitle'),
    kind: 'warning',
  })
  if (proceed) await removeKeystore()
}

async function onConfigRestored() {
  // バックアップから復旧後: 設定を再読み込みして UI に反映
  await loadConfig()
  applyConfigToLocal()
}

onMounted(async () => {
  await loadConfig()
  applyConfigToLocal()
  await refreshKeystore()
  // 起動時の同期完了を watch で検出してローカル参照に反映
  watch(appConfig, applyConfigToLocal)
})

// 任意のローカル変更を dirty フラグ化
watch(
  [general, startup, library, security, logging, updates],
  () => {
    if (appConfig.value) dirty.value = true
  },
  { deep: true },
)

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
      <div class="search" style="max-width: 280px">
        <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
        <input
          v-model="searchQuery"
          :placeholder="t('settings.searchPlaceholder')"
          :aria-label="t('common.search')"
        />
      </div>
      <div class="tb-actions">
        <button class="btn ghost" :disabled="!dirty || saving" @click="discardChanges">
          {{ t('common.discard') }}
        </button>
        <button class="btn primary" :disabled="!dirty || saving" @click="save">
          <span v-if="saving" class="spinner" style="width: 13px; height: 13px" />
          <UiIcon v-else name="Check" :size="13" />
          {{ saving ? t('common.saving') : t('common.save') }}
        </button>
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

      <div class="settings-content">
        <!-- 一般 -->
        <GeneralSection
          v-if="section === 'general'"
          v-model:language="general.language"
          v-model:show-apply-toast="general.showApplyToast"
          v-model:apply-shadow-control="general.applyShadowControl"
          @config-restored="onConfigRestored"
        />

        <StartupSection
          v-else-if="section === 'startup'"
          v-model:auto-start="startup.autoStart"
          v-model:start-minimized="startup.startMinimized"
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
          @generate="onKeystoreGenerate"
          @regenerate="onKeystoreRegenerate"
          @delete="onKeystoreDelete"
          @export="onKeystoreExport"
          @import="onKeystoreImport"
        />

        <LoggingSection
          v-else-if="section === 'logging'"
          v-model:log-level="logging.logLevel"
          v-model:retention-days="logging.retentionDays"
          v-model:max-size-mb="logging.maxSizeMb"
        />

        <!-- アップデート -->
        <UpdatesSection
          v-else-if="section === 'updates'"
          v-model:auto-update="updates.autoUpdate"
          v-model:channel="updates.channel"
          :updater-checking="updaterChecking"
          :updater-downloading="updaterDownloading"
          :updater-available="updaterAvailable"
          :updater-message="updaterMessage"
          :updater-error="updaterError"
          :updater-progress="updaterProgress"
          :updater-total="updaterTotal"
          @check-update="onCheckUpdate"
          @download-update="onDownloadUpdate"
        />

        <!-- About -->
        <AboutSection v-else />
      </div>
    </div>

    <AppStatusbar
      :items="[
        { dot: true, text: `Settings · ${t(currentSection.labelKey)}` },
        { text: t('settings.schemaInfo') },
        { text: dirty ? t('settings.unsavedChanges') : t('settings.saved') },
        ...(saveError ? [{ text: t('settings.statusError', { message: saveError }) }] : []),
      ]"
    />

    <PassphrasePrompt
      :open="passphrasePrompt.open"
      :mode="passphrasePrompt.mode"
      @update:open="passphrasePrompt.open = $event"
      @confirm="onPassphraseConfirm"
    />
  </div>
</template>

<style scoped>
.settings-host {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.settings-grid {
  flex: 1;
  display: grid;
  grid-template-columns: 220px 1fr;
  min-height: 0;
}

.settings-sidenav {
  border-right: 1px solid var(--line);
  padding: 16px 10px;
  background: rgba(255, 255, 255, 0.01);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-title {
  margin: 0 8px 10px;
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--fg-mute);
  font-weight: 500;
}

.settings-content {
  overflow-y: auto;
  padding: 24px 28px 32px;
}
.section-head {
  margin-bottom: 22px;
}
.section-head h1 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 22px;
  font-weight: 600;
  letter-spacing: -0.02em;
}
.section-head p {
  margin: 4px 0 0;
  color: var(--fg-dim);
  font-size: 13px;
}

.head-hint {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--fg-mute);
  text-transform: none;
  letter-spacing: 0;
  font-weight: 400;
}

.prop-body {
  padding: 4px 16px !important;
}

.profile-msg {
  margin-top: 12px;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--fg-dim);
  background: rgba(124, 242, 212, 0.06);
  border: 1px solid var(--accent-line);
  border-radius: 6px;
  word-break: break-all;
}
</style>
