<script setup lang="ts">
/**
 * テーマ公式インデックス提出ダイアログ (Phase 9-2)
 *
 * フロー:
 *  1. ローカルライブラリからテーマを選択
 *  2. GitHub ユーザー名を入力
 *  3. entry JSON テンプレートを生成 (sha256/signature/download_url は TODO として表示)
 *  4. 「GitHub で PR を開く」→ github.com/nishiuriraku/easy-cursor-swap-index/new/main でファイルを作成
 *
 * sha256 / signature / download_url はユーザーが cursorpack を自分の GitHub Release 等に
 * アップロードした後、手動で埋めてもらう形式とする。
 * app 内 export_cursorpack でエクスポートした値を使う手順をダイアログ内で説明する。
 */
import { computed, onMounted, ref } from 'vue'
import { invokeTauri } from '~/composables/useTauri'
import { useKeystore } from '~/composables/useKeystore'
import { useI18n } from '~/composables/useI18n'

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

const themes = ref<ThemeSummary[]>([])
const selectedThemeId = ref<string | null>(null)
const githubUsername = ref('')
const downloadUrl = ref('')
const step = ref<'select' | 'preview'>('select')
const loading = ref(false)
const copyDone = ref(false)

const selectedTheme = computed(() =>
  themes.value.find((t) => t.id === selectedThemeId.value) ?? null,
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
    download_url: downloadUrl.value || 'https://github.com/YOUR_USER/YOUR_REPO/releases/download/v1.0.0/YOUR_THEME.cursorpack',
    version: th.version,
    included_roles: th.included_roles,
    tags: [],
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

async function openGitHub() {
  const url = githubNewFileUrl.value
  if (!url) return
  try {
    await invokeTauri<void>('open_url', { url })
  } catch {
    // Tauri コンテキスト外 (nuxt dev) のフォールバック
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}

async function copyJson() {
  await navigator.clipboard.writeText(entryJson.value)
  copyDone.value = true
  setTimeout(() => { copyDone.value = false }, 2000)
}

function close() {
  emit('update:open', false)
  step.value = 'select'
  selectedThemeId.value = null
  githubUsername.value = ''
  downloadUrl.value = ''
}

onMounted(async () => {
  await Promise.all([loadThemes(), refreshKeystore()])
})
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="submit-dialog-title" @click.self="close">
      <div class="modal">
        <div class="modal-header">
          <h2 id="submit-dialog-title">{{ t('marketplace.submitTitle') }}</h2>
          <button class="close-btn" :aria-label="t('common.close')" @click="close">
            <UiIcon name="X" :size="16" />
          </button>
        </div>

        <div class="modal-body">
          <!-- Step 1: テーマ選択 + GitHub ユーザー名 -->
          <template v-if="step === 'select'">
            <p class="hint">{{ t('marketplace.submitHint') }}</p>

            <div class="field">
              <label for="submit-theme">{{ t('marketplace.submitSelectTheme') }}</label>
              <UiSelect
                v-model="selectedThemeId"
                width="100%"
                :placeholder="t('marketplace.submitSelectPlaceholder')"
                :options="themes.map((th) => ({
                  value: th.id,
                  label: `${th.name} (v${th.version})`,
                }))"
              />
            </div>

            <div class="field">
              <label for="submit-github">{{ t('marketplace.submitGithubUser') }}</label>
              <input
                id="submit-github"
                v-model="githubUsername"
                type="text"
                :placeholder="t('marketplace.submitGithubUserPlaceholder')"
              />
            </div>

            <div class="field">
              <label for="submit-dl-url">{{ t('marketplace.submitDownloadUrl') }}</label>
              <input
                id="submit-dl-url"
                v-model="downloadUrl"
                type="url"
                :placeholder="t('marketplace.submitDownloadUrlPlaceholder')"
              />
              <span class="field-note">{{ t('marketplace.submitDownloadUrlNote') }}</span>
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
        </div>

        <div class="modal-footer">
          <button v-if="step === 'select'" class="btn" @click="close">{{ t('common.cancel') }}</button>
          <button v-if="step === 'select'" class="btn primary" :disabled="!canProceed" @click="step = 'preview'">
            {{ t('marketplace.submitPreviewBtn') }}
          </button>

          <button v-if="step === 'preview'" class="btn" @click="step = 'select'">{{ t('common.back') }}</button>
          <button v-if="step === 'preview'" class="btn ghost" @click="copyJson">
            {{ copyDone ? t('common.copied') : t('common.copyJson') }}
          </button>
          <button v-if="step === 'preview'" class="btn primary" @click="openGitHub">
            <UiIcon name="Globe" :size="14" />
            {{ t('marketplace.submitOpenGithub') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  width: 560px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px 12px;
  border-bottom: 1px solid var(--border);
}

.modal-header h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--fg-mute);
  padding: 4px;
  display: flex;
  align-items: center;
}

.close-btn:hover {
  color: var(--fg);
}

.modal-body {
  padding: 16px 20px;
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.modal-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border);
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field label {
  font-size: 12px;
  font-weight: 500;
  color: var(--fg-mute);
}

.field select,
.field input[type='text'],
.field input[type='url'] {
  background: var(--surface-raised);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--fg);
  padding: 6px 8px;
  font-size: 13px;
}

.field-note {
  font-size: 11px;
  color: var(--fg-mute);
}

.hint {
  font-size: 13px;
  color: var(--fg-mute);
  margin: 0;
}

.hint.small {
  font-size: 11px;
}

.json-preview {
  background: var(--surface-raised);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: auto;
  max-height: 280px;
}

.json-preview pre {
  margin: 0;
  padding: 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  color: var(--fg);
  white-space: pre;
}

.warn-box {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--warning, #f59e0b);
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: 4px;
  padding: 8px 10px;
}

.btn {
  padding: 6px 14px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface-raised);
  color: var(--fg);
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn:hover {
  background: var(--surface-hover);
}

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.btn.primary:hover:not(:disabled) {
  opacity: 0.88;
}

.btn.primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn.ghost {
  background: transparent;
  border-color: transparent;
}

.btn.ghost:hover {
  background: var(--surface-raised);
}
</style>
