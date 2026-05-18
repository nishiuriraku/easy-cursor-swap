<script setup lang="ts">
/**
 * テーマ公式インデックス提出ダイアログ (Phase 9-2)
 *
 * タブ構成:
 *   - 自動 (Auto): useMarketplaceSubmit + useGithubAuth Device Flow を使って PR を自動作成
 *   - 手動 (Manual): GitHub ユーザー名 + ダウンロード URL → JSON 生成 → GitHub Web Editor
 */
import { computed, onMounted, ref } from 'vue'
import { invokeTauri } from '~/composables/useTauri'
import { openExternalUrl } from '~/composables/useExternalUrl'
import { useKeystore } from '~/composables/useKeystore'
import { useI18n } from '~/composables/useI18n'
import { useMarketplaceSubmit } from '~/composables/useMarketplaceSubmit'
import SubmitDeviceFlowModal from './SubmitDeviceFlowModal.vue'
import type { GithubAccount, SubmitStage } from '~/types/githubAuth'

const { t } = useI18n()

interface Props {
  open: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

// ローカルテーマ一覧の型 (Rust ThemeSummary に対応)
interface ThemeSummary {
  id: string
  name: string
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
const tagInput = ref('')
const tags = ref<string[]>([])
const MAX_TAGS = 8
const MAX_TAG_LEN = 24

function commitTag() {
  const raw = tagInput.value.trim()
  if (!raw) return
  // カンマ・セミコロン区切りで複数同時投入も許可
  const parts = raw
    .split(/[,;]/)
    .map((s) => s.trim().toLowerCase())
    .filter((s) => s.length > 0 && s.length <= MAX_TAG_LEN)
  for (const p of parts) {
    if (tags.value.length >= MAX_TAGS) break
    if (!tags.value.includes(p)) tags.value.push(p)
  }
  tagInput.value = ''
}

function onTagKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ',' || e.key === ';') {
    e.preventDefault()
    commitTag()
  } else if (e.key === 'Backspace' && tagInput.value === '' && tags.value.length > 0) {
    tags.value.pop()
  }
}

function removeTag(i: number) {
  tags.value.splice(i, 1)
}

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

function stageLabel(s: SubmitStage | null): string {
  if (!s) return ''
  return t(STAGE_LABEL_KEY[s])
}

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
  // 入力中の未確定タグも自動で確定させる
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

// GitHub インデックスリポジトリのベース URL
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
  tags.value = []
  tagInput.value = ''
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

          <!-- 自動タブ -->
          <template v-if="tab === 'auto'">
            <p class="hint">{{ t('marketplace.submitAutoIntro') }}</p>

            <div class="field">
              <label for="submit-auto-theme">{{ t('marketplace.submitSelectTheme') }}</label>
              <UiSelect
                id="submit-auto-theme"
                v-model="selectedThemeId"
                width="100%"
                :placeholder="t('marketplace.submitSelectPlaceholder')"
                :options="
                  themes.map((th) => ({
                    value: th.id,
                    label: `${th.name} (v${th.version})`,
                  }))
                "
              />
            </div>

            <div class="field">
              <label for="submit-auto-tags">{{ t('marketplace.submitTagsLabel') }}</label>
              <div class="tag-input-row">
                <span v-for="(tg, i) in tags" :key="`${tg}-${i}`" class="tag-chip">
                  {{ tg }}
                  <button
                    type="button"
                    class="tag-chip-x"
                    :aria-label="t('marketplace.submitTagRemoveAria', { tag: tg })"
                    @click="removeTag(i)"
                  >
                    <UiIcon name="X" :size="10" />
                  </button>
                </span>
                <input
                  id="submit-auto-tags"
                  v-model="tagInput"
                  type="text"
                  class="tag-input"
                  :placeholder="
                    tags.length === 0
                      ? t('marketplace.submitTagsPlaceholder')
                      : t('marketplace.submitTagsPlaceholderAdd')
                  "
                  :disabled="tags.length >= MAX_TAGS"
                  @keydown="onTagKeydown"
                  @blur="commitTag"
                />
              </div>
              <span class="field-note">{{ t('marketplace.submitTagsNote') }}</span>
            </div>

            <div v-if="githubAccount" class="hint">
              {{ t('marketplace.submitAutoLinkedAs', { login: githubAccount.login }) }}
            </div>
            <div v-else class="hint">{{ t('marketplace.submitAutoNeedsLink') }}</div>

            <div v-if="submitter.busy.value" class="hint" aria-live="polite">
              {{ stageLabel(submitter.stage.value) }}
            </div>
            <div v-if="submitter.errorMsg.value" class="warn-box">
              <UiIcon name="AlertTriangle" :size="14" />
              {{ submitter.errorMsg.value }}
            </div>
            <div v-if="submitDone" class="hint">
              {{ t('marketplace.submitDone') }}
            </div>

            <div v-if="!keystoreInfo.has_keypair" class="warn-box">
              <UiIcon name="AlertTriangle" :size="14" />
              {{ t('marketplace.submitNoKeystore') }}
            </div>
          </template>

          <!-- 手動タブ -->
          <template v-else>
            <!-- Step 1: テーマ選択 + GitHub ユーザー名 -->
            <template v-if="step === 'select'">
              <p class="hint">{{ t('marketplace.submitHint') }}</p>

              <div class="field">
                <label for="submit-theme">{{ t('marketplace.submitSelectTheme') }}</label>
                <UiSelect
                  v-model="selectedThemeId"
                  width="100%"
                  :placeholder="t('marketplace.submitSelectPlaceholder')"
                  :options="
                    themes.map((th) => ({
                      value: th.id,
                      label: `${th.name} (v${th.version})`,
                    }))
                  "
                />
              </div>

              <div class="field">
                <label for="submit-github">{{ t('marketplace.submitGithubUser') }}</label>
                <input
                  id="submit-github"
                  v-model="githubUsername"
                  type="text"
                  class="input"
                  :placeholder="t('marketplace.submitGithubUserPlaceholder')"
                />
              </div>

              <div class="field">
                <label for="submit-dl-url">{{ t('marketplace.submitDownloadUrl') }}</label>
                <input
                  id="submit-dl-url"
                  v-model="downloadUrl"
                  type="url"
                  class="input"
                  :placeholder="t('marketplace.submitDownloadUrlPlaceholder')"
                />
                <span class="field-note">{{ t('marketplace.submitDownloadUrlNote') }}</span>
              </div>

              <div class="field">
                <label for="submit-manual-tags">{{ t('marketplace.submitTagsLabel') }}</label>
                <div class="tag-input-row">
                  <span v-for="(tg, i) in tags" :key="`${tg}-${i}`" class="tag-chip">
                    {{ tg }}
                    <button
                      type="button"
                      class="tag-chip-x"
                      :aria-label="t('marketplace.submitTagRemoveAria', { tag: tg })"
                      @click="removeTag(i)"
                    >
                      <UiIcon name="X" :size="10" />
                    </button>
                  </span>
                  <input
                    id="submit-manual-tags"
                    v-model="tagInput"
                    type="text"
                    class="tag-input"
                    :placeholder="
                      tags.length === 0
                        ? t('marketplace.submitTagsPlaceholder')
                        : t('marketplace.submitTagsPlaceholderAdd')
                    "
                    :disabled="tags.length >= MAX_TAGS"
                    @keydown="onTagKeydown"
                    @blur="commitTag"
                  />
                </div>
                <span class="field-note">{{ t('marketplace.submitTagsNote') }}</span>
              </div>

              <div v-if="!keystoreInfo.has_keypair" class="warn-box">
                <UiIcon name="AlertTriangle" :size="14" />
                {{ t('marketplace.submitNoKeystore') }}
              </div>
            </template>

            <!-- Step 2: JSON プレビュー -->
            <template v-else>
              <p class="hint">{{ t('marketplace.submitPreviewHint') }}</p>
              <div class="json-preview">
                <pre>{{ entryJson }}</pre>
              </div>
              <p class="hint small">{{ t('marketplace.submitFillInNote') }}</p>
            </template>
          </template>
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
 * 本コンポーネントは独自フィールドレイアウト + JSON プレビュー枠のみ scoped で持つ。 */

.submit-modal {
  @apply flex w-[640px] max-w-[92vw] flex-col;
  max-height: 80vh;
}

.submit-body {
  @apply flex max-h-[60vh] flex-col gap-3 overflow-y-auto;
}

.field {
  @apply flex flex-col gap-1;
}

.field label {
  @apply text-[12px] font-medium text-fg-mute;
}

.field-note {
  @apply text-[11px] text-fg-mute;
}

/* タグ chip + 末尾の text input を 1 行に並べる input-like ラッパ。 */
.tag-input-row {
  @apply flex flex-wrap items-center gap-1.5 rounded-md border border-line px-2 py-1.5;
  background: rgba(255, 255, 255, 0.03);
  min-height: 34px;
}
.tag-input-row:focus-within {
  border-color: var(--accent-line);
}
:where(html.light) .tag-input-row {
  background: rgba(15, 20, 35, 0.025);
}
.tag-chip {
  @apply inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11.5px];
  background: var(--accent-dim);
  color: var(--accent);
}
.tag-chip-x {
  @apply inline-flex cursor-pointer items-center justify-center border-0 bg-transparent p-0 text-current;
  line-height: 0;
}
.tag-chip-x:hover {
  opacity: 0.75;
}
.tag-input {
  @apply min-w-[120px] flex-1 border-0 bg-transparent p-0 text-[12.5px] text-fg outline-none;
}
.tag-input::placeholder {
  color: var(--fg-mute);
}

.hint {
  @apply m-0 text-[13px] text-fg-dim;
}

.hint.small {
  @apply text-[11px] text-fg-mute;
}

.json-preview {
  @apply max-h-[300px] overflow-auto rounded-[8px] border border-line bg-black/30;
}

.json-preview pre {
  @apply m-0 whitespace-pre p-3 font-mono text-[12px] leading-[1.5] text-fg;
}

:where(html.light) .json-preview {
  background: rgba(15, 20, 35, 0.04);
}

.submit-mode-label {
  @apply text-[11px] font-medium uppercase tracking-wide text-fg-mute;
}

.warn-box {
  @apply flex items-center gap-1.5 rounded-[8px] border px-2.5 py-2 text-[12px];
  background: rgba(245, 194, 107, 0.1);
  border-color: rgba(245, 194, 107, 0.3);
  color: var(--amber);
}
</style>
