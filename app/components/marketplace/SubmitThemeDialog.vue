<script setup lang="ts">
/**
 * テーマ公式インデックス提出ダイアログ (Phase 9-2)
 *
 * タブ構成:
 *   - 自動 (Auto): useMarketplaceSubmit + useGithubAuth Device Flow を使って PR を自動作成
 *   - 手動 (Manual): GitHub ユーザー名 + ダウンロード URL → JSON 生成 → GitHub Web Editor
 *
 * このコンテナはモーダル骨格 + タブ切替 + 共有 state (テーマ一覧 / keystore / タグ) を保持し、
 * Auto/Manual タブの本文は子コンポーネント (SubmitThemeAutoForm / SubmitThemeManualForm) に委譲する。
 * フッターは step / submitDone の状態を見て切り替えるため親に残す。
 */
import type { GithubAccount, SubmitStage } from '~/types/githubAuth'
import type { MarketplaceName } from '~/types/marketplace'

const { t } = useI18n()

interface Props {
  open: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

// ローカルテーマ一覧の型 (Rust ThemeSummary に対応)。
// `name` は Rust 側 `LocalizedString` の生形 (`string | { [locale]: string }`) で渡ってくるため
// 表示時は `pickLocalizedName` を介す。公式インデックス entry の `name` も localized object を
// 許容するので、`entryJson` ではそのまま JSON 化する (`MarketplaceEntry.name` 型と整合)。
interface ThemeSummary {
  id: string
  name: MarketplaceName
  author: string | null
  version: string
  included_roles: string[]
  is_active: boolean
}

const { info: keystoreInfo, refresh: refreshKeystore } = useKeystore()

// 共有状態
const themes = ref<ThemeSummary[]>([])
const selectedThemeId = ref<string | null>(null)
const loading = ref(false)

// タブ管理
const tab = ref<'auto' | 'manual'>('auto')

// ── 自動タブ ──────────────────────────────────────────
const githubAccount = ref<GithubAccount | null>(null)
const deviceFlowOpen = ref(false)
const submitter = useMarketplaceSubmit()
const submitDone = ref<{ prUrl: string } | null>(null)

// マーケットプレイス用タグ入力 (自動・手動共通)。Enter / カンマ / セミコロンで chip 化。
const {
  tagInput,
  tags,
  commitTag,
  onTagKeydown,
  removeTag,
  reset: resetTags,
  maxTags: MAX_TAGS,
} = useTagChipInput()

const STAGE_LABEL_KEY: Record<SubmitStage, string> = {
  build: 'marketplace.submitStageBuild',
  auth: 'marketplace.submitStageAuth',
  fork: 'marketplace.submitStageFork',
  sync_fork: 'marketplace.submitStageSyncFork',
  branch: 'marketplace.submitStageBranch',
  upload_pack: 'marketplace.submitStageUploadPack',
  upload_previews: 'marketplace.submitStageUploadPreviews',
  upload_entry: 'marketplace.submitStageUploadEntry',
  open_pr: 'marketplace.submitStageOpenPr',
}

const submitterStageLabel = computed(() => {
  const s = submitter.stage.value
  return s ? t(STAGE_LABEL_KEY[s]) : ''
})

async function loadGithubAccount() {
  try {
    const cfg = await invokeTauri<{ github_account: GithubAccount | null }>('get_config')
    githubAccount.value = cfg?.github_account ?? null
  } catch {
    githubAccount.value = null
  }
}

async function runAutoSubmit() {
  if (!selectedThemeId.value) return
  if (tagInput.value.trim().length > 0) commitTag()
  submitDone.value = null
  try {
    const r = await submitter.submit(selectedThemeId.value, tags.value)
    submitDone.value = { prUrl: r.prUrl }
  } catch {
    // submitter.errorMsg は composable 内でセット済み; UI でバナー表示
  }
}

async function onAutoSubmitClick() {
  if (!selectedThemeId.value) return
  if (!githubAccount.value) {
    deviceFlowOpen.value = true
    return
  }
  await runAutoSubmit()
}

async function onDeviceFlowReady() {
  await loadGithubAccount()
  await runAutoSubmit()
}

function openPr() {
  if (!submitDone.value) return
  void openExternalUrl(submitDone.value.prUrl)
}

// ── 手動タブ ──────────────────────────────────────────
const githubUsername = ref('')
const downloadUrl = ref('')
const step = ref<'select' | 'preview'>('select')
const copyDone = ref(false)

const selectedTheme = computed(
  () => themes.value.find((th) => th.id === selectedThemeId.value) ?? null,
)

const INDEX_REPO = 'https://github.com/nishiuriraku/easy-cursor-swap-index'

const entryJson = computed(() => {
  const th = selectedTheme.value
  if (!th) return ''
  const entry = {
    id: th.id,
    name: th.name ?? 'My Theme',
    author: th.author ?? githubUsername.value,
    author_github: githubUsername.value || 'FILL_IN_GITHUB_USERNAME',
    author_pubkey_id: keystoreInfo.value.key_id ?? 'FILL_IN_KEY_ID',
    sha256: 'FILL_IN_SHA256_OF_CURSORPACK',
    signature: 'FILL_IN_ED25519_SIGNATURE',
    download_url:
      downloadUrl.value ||
      'https://github.com/YOUR_USER/YOUR_REPO/releases/download/v1.0.0/YOUR_THEME.cursorpack',
    version: th.version,
    included_roles: th.included_roles,
    tags: tags.value,
  }
  return JSON.stringify(entry, null, 2)
})

const githubNewFileUrl = computed(() => {
  if (!selectedTheme.value) return ''
  const filename = `entries/${selectedTheme.value.id}.json`
  const encoded = encodeURIComponent(entryJson.value)
  return `${INDEX_REPO}/new/main?filename=${encodeURIComponent(filename)}&value=${encoded}`
})

const canProceed = computed(
  () => selectedThemeId.value !== null && githubUsername.value.trim().length > 0,
)

async function openGitHub() {
  const url = githubNewFileUrl.value
  if (!url) return
  await openExternalUrl(url)
}

async function copyJson() {
  await navigator.clipboard.writeText(entryJson.value)
  copyDone.value = true
  setTimeout(() => {
    copyDone.value = false
  }, 2000)
}

// ── 共通 ──────────────────────────────────────────────
async function loadThemes() {
  loading.value = true
  try {
    themes.value = await invokeTauri<ThemeSummary[]>('get_themes')
  } catch {
    themes.value = []
  } finally {
    loading.value = false
  }
}

function close() {
  emit('update:open', false)
  // 自動タブのリセット
  tab.value = 'auto'
  deviceFlowOpen.value = false
  submitDone.value = null
  // 手動タブのリセット
  step.value = 'select'
  selectedThemeId.value = null
  githubUsername.value = ''
  downloadUrl.value = ''
  // 共通のタグもリセット
  resetTags()
}

onMounted(async () => {
  await Promise.all([loadThemes(), refreshKeystore(), loadGithubAccount()])
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="modal-page"
      role="dialog"
      aria-modal="true"
      aria-labelledby="submit-dialog-title"
      @click.self="close"
    >
      <div class="modal submit-modal" @click.stop>
        <div class="modal-head">
          <div class="modal-icon" aria-hidden="true"><UiIcon name="Upload" :size="18" /></div>
          <div style="flex: 1; min-width: 0">
            <h2 id="submit-dialog-title">{{ t('marketplace.submitTitle') }}</h2>
          </div>
          <button type="button" class="btn icon" :aria-label="t('common.close')" @click="close">
            <UiIcon name="X" :size="14" />
          </button>
        </div>

        <div class="modal-body submit-body">
          <!-- タブバー -->
          <div class="submit-mode-label">{{ t('marketplace.submitMode') }}</div>
          <div
            class="tabs"
            style="border: none; background: transparent; padding: 0; height: auto"
            role="tablist"
          >
            <button
              type="button"
              :class="['tab', { active: tab === 'auto' }]"
              @click="tab = 'auto'"
            >
              {{ t('marketplace.submitModeAuto') }}
            </button>
            <button
              type="button"
              :class="['tab', { active: tab === 'manual' }]"
              @click="tab = 'manual'"
            >
              {{ t('marketplace.submitModeManual') }}
            </button>
          </div>

          <SubmitThemeAutoForm
            v-if="tab === 'auto'"
            :themes="themes"
            :selected-theme-id="selectedThemeId"
            :tags="tags"
            :tag-input="tagInput"
            :has-keystore="keystoreInfo.has_keypair"
            :max-tags="MAX_TAGS"
            :github-account="githubAccount"
            :submitter-busy="submitter.busy.value"
            :submitter-stage-label="submitterStageLabel"
            :submitter-error-msg="submitter.errorMsg.value"
            :submit-done="submitDone !== null"
            @update:selected-theme-id="(v) => (selectedThemeId = v)"
            @update:tag-input="(v) => (tagInput = v)"
            @commit-tag="commitTag"
            @remove-tag="removeTag"
            @tag-keydown="onTagKeydown"
          />

          <SubmitThemeManualForm
            v-else
            :themes="themes"
            :selected-theme-id="selectedThemeId"
            :tags="tags"
            :tag-input="tagInput"
            :has-keystore="keystoreInfo.has_keypair"
            :max-tags="MAX_TAGS"
            :step="step"
            :github-username="githubUsername"
            :download-url="downloadUrl"
            :entry-json="entryJson"
            @update:selected-theme-id="(v) => (selectedThemeId = v)"
            @update:tag-input="(v) => (tagInput = v)"
            @update:github-username="(v) => (githubUsername = v)"
            @update:download-url="(v) => (downloadUrl = v)"
            @commit-tag="commitTag"
            @remove-tag="removeTag"
            @tag-keydown="onTagKeydown"
          />
        </div>

        <div class="modal-foot">
          <!-- 自動タブのフッター -->
          <template v-if="tab === 'auto'">
            <button class="btn" @click="close">{{ t('common.cancel') }}</button>
            <button
              v-if="!submitDone"
              class="btn primary"
              :disabled="!selectedThemeId || submitter.busy.value"
              @click="onAutoSubmitClick"
            >
              {{
                githubAccount
                  ? t('marketplace.submitAutoSubmitBtn')
                  : t('marketplace.submitAutoLinkBtn')
              }}
            </button>
            <button v-else class="btn primary" @click="openPr">
              <UiIcon name="Globe" :size="14" />
              {{ t('marketplace.submitOpenPrBtn') }}
            </button>
          </template>

          <!-- 手動タブのフッター -->
          <template v-else>
            <span class="left-note">step {{ step === 'select' ? '1' : '2' }} / 2</span>
            <div class="actions">
              <button v-if="step === 'select'" class="btn" @click="close">
                {{ t('common.cancel') }}
              </button>
              <button
                v-if="step === 'select'"
                class="btn primary"
                :disabled="!canProceed"
                @click="step = 'preview'"
              >
                {{ t('marketplace.submitPreviewBtn') }}
              </button>

              <button v-if="step === 'preview'" class="btn" @click="step = 'select'">
                {{ t('common.back') }}
              </button>
              <button v-if="step === 'preview'" class="btn ghost" @click="copyJson">
                {{ copyDone ? t('common.copied') : t('common.copyJson') }}
              </button>
              <button v-if="step === 'preview'" class="btn primary" @click="openGitHub">
                <UiIcon name="Globe" :size="14" />
                {{ t('marketplace.submitOpenGithub') }}
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- Device Flow モーダル (自動タブ) -->
  <SubmitDeviceFlowModal v-model:open="deviceFlowOpen" @ready="onDeviceFlowReady" />
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* 共有 modal 系 (.modal-page / .modal / .modal-head / .modal-body / .modal-foot) と
 * .input / .btn は tailwind.css の top-level shared utility に集約済み。
 * フィールドレイアウト + タグ chip + JSON プレビュー枠は子コンポーネントの scoped style に移譲。 */

.submit-modal {
  @apply flex w-[640px] max-w-[92vw] flex-col;
  max-height: 80vh;
}

.submit-body {
  @apply flex max-h-[60vh] flex-col gap-3 overflow-y-auto;
}

.submit-mode-label {
  @apply text-[11px] font-medium uppercase tracking-wide text-fg-mute;
}
</style>
