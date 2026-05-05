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
import { useAppConfig } from '~/composables/useAppConfig'
import { useKeystore } from '~/composables/useKeystore'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

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
  label: string
  icon: string
}

const SECTIONS: SectionDef[] = [
  { id: 'general', label: '一般', icon: 'Settings' },
  { id: 'startup', label: '起動・常駐', icon: 'Logo' },
  { id: 'library', label: 'テーマライブラリ', icon: 'Library' },
  { id: 'security', label: 'セキュリティ', icon: 'Shield' },
  { id: 'keys', label: '署名鍵 (Ed25519)', icon: 'Pkg' },
  { id: 'logging', label: 'ログ・診断', icon: 'Sort' },
  { id: 'updates', label: 'アップデート', icon: 'Import' },
  { id: 'about', label: 'About', icon: 'Globe' },
]

const section = ref<SectionId>('general')
const searchQuery = ref('')

const { config: appConfig, load: loadConfig, update: persistConfig } = useAppConfig()

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
const { info: keystoreInfo, busy: keystoreBusy, lastError: keystoreError, refresh: refreshKeystore, generate: generateKeystore, remove: removeKeystore, exportPrivate: exportPrivateKey, importPrivate: importPrivateKey } = useKeystore()

// パスフレーズプロンプト制御
const passphrasePrompt = ref<{ mode: 'export' | 'import', open: boolean }>({ mode: 'export', open: false })
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
      defaultPath: `cursorforge-${today}.cursorprofile`,
      filters: [{ name: 'CursorForge Profile', extensions: ['cursorprofile'] }],
    })
    if (!target) return
    await invokeTauri<void>('export_profile', { path: target })
    profileMessage.value = `エクスポート完了: ${target}`
  } catch (err) {
    profileMessage.value = `エクスポート失敗: ${err instanceof Error ? err.message : String(err)}`
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
      filters: [{ name: 'CursorForge Profile', extensions: ['cursorprofile'] }],
    })
    if (!selected || Array.isArray(selected)) return
    const overwrite = await ask(
      '既存テーマを完全に上書きしますか？\n「いいえ」でマージモード (新規分のみ反映) になります。',
      { title: 'プロファイル復元モード', kind: 'warning' },
    )
    await invokeTauri<unknown>('import_profile', { path: selected, merge: !overwrite })
    profileMessage.value = `インポート完了: ${selected}`
    // 設定の再読み込み
    await loadConfig()
    applyConfigToLocal()
  } catch (err) {
    profileMessage.value = `インポート失敗: ${err instanceof Error ? err.message : String(err)}`
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
  const proceed = await ask(
    '既存の鍵ペアを破棄して新しい鍵を生成します。\n既存の署名済みテーマは検証できなくなる可能性があります。',
    { title: '鍵を再生成', kind: 'warning' },
  )
  if (proceed) await generateKeystore(true)
}
async function onPassphraseConfirm(passphrase: string) {
  const mode = passphrasePrompt.value.mode
  keystoreMessage.value = null
  if (mode === 'export') {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const today = new Date().toISOString().slice(0, 10)
    const target = await save({
      defaultPath: `cursorforge-key-${today}.cfkey`,
      filters: [{ name: 'CursorForge Key', extensions: ['cfkey'] }],
    })
    if (!target) return
    const written = await exportPrivateKey(passphrase, target)
    if (written !== null) {
      keystoreMessage.value = `秘密鍵をエクスポートしました (${written} bytes) → ${target}`
    }
  } else {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: false,
      filters: [{ name: 'CursorForge Key', extensions: ['cfkey'] }],
    })
    if (!selected || Array.isArray(selected)) return
    const result = await importPrivateKey(passphrase, selected)
    if (result) {
      keystoreMessage.value = `秘密鍵をインポートしました key_id=${result.key_id ?? '?'}`
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
  const proceed = await ask(
    '鍵ペアを削除します。署名機能は利用できなくなります。',
    { title: '鍵を削除', kind: 'warning' },
  )
  if (proceed) await removeKeystore()
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

const currentSection = computed(() =>
  SECTIONS.find((s) => s.id === section.value) ?? SECTIONS[0]!,
)

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
        <span class="crumb active">{{ currentSection.label }}</span>
      </div>
      <div class="search" style="max-width: 280px">
        <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
        <input v-model="searchQuery" :placeholder="t('settings.searchPlaceholder')" :aria-label="t('common.search')" />
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
      <nav class="settings-sidenav">
        <h6 class="nav-title">Preferences</h6>
        <button
          v-for="s in SECTIONS"
          :key="s.id"
          :class="['nav-item', { active: section === s.id }]"
          @click="selectSection(s.id)"
        >
          <UiIcon :name="s.icon" />
          <span>{{ s.label }}</span>
        </button>
      </nav>

      <div class="settings-content">
        <!-- 一般 -->
        <section v-if="section === 'general'">
          <header class="section-head">
            <h1>一般</h1>
            <p>言語、通知、起動時の挙動など、アプリ全体の基本設定。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">表示言語</div>
            <div class="prop-body">
              <SettingsRow
                label="UI 言語"
                desc="OS のロケールから自動判定。手動で固定可能。"
              >
                <select v-model="general.language" class="input" style="width: 140px; height: 32px">
                  <option value="ja">日本語</option>
                  <option value="en">English</option>
                </select>
              </SettingsRow>
            </div>
          </div>

          <div class="prop-section">
            <div class="prop-head">通知</div>
            <div class="prop-body">
              <SettingsRow
                label="適用結果のトースト表示"
                desc="Win32 COM 経由の Windows トーストで適用成功/失敗を告知"
              >
                <SettingsToggle v-model="general.showApplyToast" />
              </SettingsRow>
              <SettingsRow
                label="OS 標準ポインター影を制御"
                desc="テーマの requires_os_shadow に従い SPI_SETCURSORSHADOW を呼び出す"
              >
                <SettingsToggle v-model="general.applyShadowControl" />
              </SettingsRow>
            </div>
          </div>
        </section>

        <!-- 起動・常駐 -->
        <section v-else-if="section === 'startup'">
          <header class="section-head">
            <h1>起動・常駐</h1>
            <p>OS 起動時の自動実行とトレイ常駐の挙動。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">
              自動起動
              <span class="head-hint">HKCU\…\Run</span>
            </div>
            <div class="prop-body">
              <SettingsRow
                label="OS 起動時にサイレントで起動"
                desc="メイン画面は出さず、トレイのみで常駐 (ダークモード自動切替を有効化)"
              >
                <SettingsToggle v-model="startup.autoStart" />
              </SettingsRow>
              <SettingsRow
                label="メイン画面を最小化で起動"
                desc="ユーザー起動時もウィンドウを表示せずトレイへ"
              >
                <SettingsToggle v-model="startup.startMinimized" />
              </SettingsRow>
            </div>
          </div>
        </section>

        <!-- ライブラリ -->
        <section v-else-if="section === 'library'">
          <header class="section-head">
            <h1>テーマライブラリ</h1>
            <p>~/.custom_cursors/ のストレージ警告と .cursorprofile バックアップ。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">ストレージ警告</div>
            <div class="prop-body">
              <SettingsRow
                label="合計サイズの警告閾値"
                desc="超過時にトレイ通知でテーマ削除 UI へ誘導 (強制削除はしません)"
              >
                <select v-model.number="library.totalLimitWarnGb" class="input" style="width: 100px; height: 32px">
                  <option :value="0.5">0.5 GB</option>
                  <option :value="1">1 GB</option>
                  <option :value="2">2 GB</option>
                  <option :value="5">5 GB</option>
                </select>
              </SettingsRow>
              <SettingsRow
                label="警告を有効化"
                desc="OFF にすると合計サイズの監視が停止します"
              >
                <SettingsToggle v-model="library.storageWarnEnabled" />
              </SettingsRow>
            </div>
          </div>

          <div class="prop-section">
            <div class="prop-head">
              .cursorprofile バックアップ
              <span class="head-hint">設定 + 全テーマの Zip</span>
            </div>
            <div class="prop-body">
              <SettingsRow
                label="現在の設定とテーマを書き出し"
                desc="PC 移行 / OS 再インストール時の復元用 Zip を生成"
              >
                <button class="btn" :disabled="profileBusy" @click="exportProfile">
                  <span v-if="profileBusy" class="spinner" style="width: 13px; height: 13px" />
                  <UiIcon v-else name="Export" :size="13" />エクスポート
                </button>
              </SettingsRow>
              <SettingsRow
                label="バックアップから復元"
                desc=".cursorprofile ファイルを読み込んで設定とテーマをマージ / 上書き"
              >
                <button class="btn" :disabled="profileBusy" @click="importProfile">
                  <span v-if="profileBusy" class="spinner" style="width: 13px; height: 13px" />
                  <UiIcon v-else name="Import" :size="13" />インポート
                </button>
              </SettingsRow>
              <div v-if="profileMessage" class="profile-msg">{{ profileMessage }}</div>
            </div>
          </div>
        </section>

        <!-- セキュリティ -->
        <section v-else-if="section === 'security'">
          <header class="section-head">
            <h1>セキュリティ</h1>
            <p>署名検証、未署名テーマの扱い、検証閾値。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">テーマ検証</div>
            <div class="prop-body">
              <SettingsRow
                label="署名済みテーマのみインポート許可"
                desc="未署名 .cursorpack を完全にブロック (公式インデックス由来のみ許可)"
              >
                <SettingsToggle v-model="security.requireSignedThemes" />
              </SettingsRow>
              <SettingsRow
                label="未署名テーマのインポート時に警告"
                desc="ローカルファイルから取り込む際に強警告ダイアログを表示"
              >
                <SettingsToggle v-model="security.warnUnsignedImport" />
              </SettingsRow>
            </div>
          </div>
        </section>

        <!-- 署名鍵 -->
        <section v-else-if="section === 'keys'">
          <header class="section-head">
            <h1>署名鍵 (Ed25519)</h1>
            <p>クリエイターとしてテーマに署名する Ed25519 鍵の管理。秘密鍵は DPAPI 暗号化で保存。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">
              鍵ペア
              <span class="head-hint">~/.custom_cursors/_keys/</span>
            </div>
            <div class="prop-body">
              <template v-if="keystoreInfo.has_keypair">
                <SettingsRow label="key_id (公開鍵 SHA-256 先頭 16 文字)" mono>
                  <span class="tag ok">{{ keystoreInfo.key_id ?? '—' }}</span>
                </SettingsRow>
                <SettingsRow
                  v-if="keystoreInfo.public_key_b64"
                  label="公開鍵 (Base64)"
                  desc="Marketplace の authors/{user}.json に登録する値"
                  mono
                >
                  <span class="tag" style="max-width: 320px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-block;">
                    {{ keystoreInfo.public_key_b64 }}
                  </span>
                </SettingsRow>
                <SettingsRow
                  label="秘密鍵をエクスポート"
                  desc="パスフレーズで暗号化したバックアップ (.cfkey)"
                >
                  <button class="btn" :disabled="keystoreBusy" @click="onKeystoreExport">
                    <UiIcon name="Export" :size="13" />エクスポート
                  </button>
                </SettingsRow>
                <SettingsRow
                  label="鍵を再生成"
                  desc="既存テーマの署名は検証不能になります"
                >
                  <button class="btn danger" :disabled="keystoreBusy" @click="onKeystoreRegenerate">
                    <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
                    <UiIcon v-else name="Alert" :size="13" />再生成
                  </button>
                </SettingsRow>
                <SettingsRow
                  label="鍵ペアを削除"
                  desc="署名機能を停止。再度生成すれば key_id は変わります"
                >
                  <button class="btn danger" :disabled="keystoreBusy" @click="onKeystoreDelete">
                    <UiIcon name="X" :size="13" />削除
                  </button>
                </SettingsRow>
              </template>
              <template v-else>
                <SettingsRow
                  label="鍵ペアを生成"
                  desc="Ed25519 鍵ペアを生成し、秘密鍵を DPAPI で暗号化保存"
                >
                  <button class="btn primary" :disabled="keystoreBusy" @click="onKeystoreGenerate">
                    <span v-if="keystoreBusy" class="spinner" style="width: 13px; height: 13px" />
                    <UiIcon v-else name="Plus" :size="13" />鍵を生成
                  </button>
                </SettingsRow>
                <SettingsRow
                  label="既存秘密鍵をインポート"
                  desc="他 PC で生成した .cfkey ファイルをパスフレーズ付きで取り込み"
                >
                  <button class="btn" :disabled="keystoreBusy" @click="onKeystoreImport">
                    <UiIcon name="Import" :size="13" />インポート
                  </button>
                </SettingsRow>
              </template>
              <div v-if="keystoreMessage" class="profile-msg">{{ keystoreMessage }}</div>
              <div v-if="keystoreError" class="profile-msg" style="background: rgba(255,107,138,0.06); border-color: rgba(255,107,138,0.4); color: #ffb8c5;">
                {{ keystoreError }}
              </div>
            </div>
          </div>
        </section>

        <!-- ログ -->
        <section v-else-if="section === 'logging'">
          <header class="section-head">
            <h1>ログ・診断</h1>
            <p>%LOCALAPPDATA%\CursorForge\logs\ に保存されるログの保持と粒度。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">ログ出力</div>
            <div class="prop-body">
              <SettingsRow label="ログレベル" desc="リリース版は INFO 推奨。トラブル時は DEBUG へ">
                <select v-model="logging.logLevel" class="input" style="width: 140px; height: 32px">
                  <option>TRACE</option>
                  <option>DEBUG</option>
                  <option>INFO</option>
                  <option>WARN</option>
                  <option>ERROR</option>
                </select>
              </SettingsRow>
              <SettingsRow label="保持期間 (日)" desc="超過したログファイルは自動削除">
                <input v-model.number="logging.retentionDays" type="number" class="input" min="1" max="365" style="width: 80px" />
              </SettingsRow>
              <SettingsRow label="合計上限サイズ (MB)" desc="超過時は古いものから削除">
                <input v-model.number="logging.maxSizeMb" type="number" class="input" min="10" max="2048" style="width: 80px" />
              </SettingsRow>
              <SettingsRow
                label="現在のログフォルダーを開く"
                desc="エクスプローラーで `%LOCALAPPDATA%\CursorForge\logs\` を開く"
              >
                <button class="btn">
                  <UiIcon name="Globe" :size="13" />開く
                </button>
              </SettingsRow>
            </div>
          </div>
        </section>

        <!-- アップデート -->
        <section v-else-if="section === 'updates'">
          <header class="section-head">
            <h1>アップデート</h1>
            <p>自動アップデートの有効化と更新チャンネル。メジャーバージョン跨ぎは常に手動。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">自動アップデート</div>
            <div class="prop-body">
              <SettingsRow
                label="バックグラウンド更新を有効化"
                desc="Tauri Updater で署名検証付きの差分更新を取得"
              >
                <SettingsToggle v-model="updates.autoUpdate" />
              </SettingsRow>
              <SettingsRow label="チャンネル" desc="beta は実験機能を含む可能性があります">
                <select v-model="updates.channel" class="input" style="width: 140px; height: 32px">
                  <option value="stable">stable</option>
                  <option value="beta">beta</option>
                </select>
              </SettingsRow>
              <SettingsRow label="今すぐ確認">
                <button class="btn">
                  <UiIcon name="Import" :size="13" />更新を確認
                </button>
              </SettingsRow>
            </div>
          </div>
        </section>

        <!-- About -->
        <section v-else>
          <header class="section-head">
            <h1>About</h1>
            <p>バージョン、ライセンス、システム情報。</p>
          </header>
          <div class="prop-section">
            <div class="prop-head">
              CursorForge
              <span class="head-hint">v1.0.0 · MIT License</span>
            </div>
            <div class="prop-body">
              <SettingsRow label="ホームページ" mono>
                <a class="btn ghost" href="https://github.com/cursorforge" target="_blank" rel="noopener">
                  <UiIcon name="Globe" :size="13" />github.com/cursorforge
                </a>
              </SettingsRow>
              <SettingsRow label="Issue / バグ報告" mono>
                <a class="btn ghost" href="https://github.com/cursorforge/cursor-forge/issues" target="_blank" rel="noopener">
                  <UiIcon name="Alert" :size="13" />Issues
                </a>
              </SettingsRow>
              <SettingsRow label="OSS ライセンス一覧">
                <button class="btn">表示</button>
              </SettingsRow>
            </div>
          </div>
        </section>
      </div>
    </div>

    <AppStatusbar
      :items="[
        { dot: true, text: `Settings · ${currentSection.label}` },
        { text: 'config.json schema v3.2' },
        { text: dirty ? t('settings.unsavedChanges') : t('settings.saved') },
        ...(saveError ? [{ text: `エラー: ${saveError}` }] : []),
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
