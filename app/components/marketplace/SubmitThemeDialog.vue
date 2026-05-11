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

const selectedTheme = computed(
  () => themes.value.find((t) => t.id === selectedThemeId.value) ?? null,
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
  setTimeout(() => {
    copyDone.value = false
  }, 2000)
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
    <div
      v-if="open"
      class="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="submit-dialog-title"
      @click.self="close"
    >
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
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

/* NOTE: --surface / --surface-raised / --surface-hover / --border / --warning は
 * 未定義トークン (元コードの leftover)。それらを参照する declaration は invalid
 * で discarded されていた。global に同名ルールが無いコンポーネント固有なので、
 * 視覚的な現状維持のためそれらの行はそのまま literal CSS で残置する。 */

.modal-backdrop {
  @apply fixed inset-0 z-[1000] flex items-center justify-center bg-black/55;
}

.modal {
  @apply flex max-h-[80vh] w-[560px] max-w-[90vw] flex-col rounded-[8px] border shadow-[0_16px_48px_rgba(0,0,0,0.4)];
  background: var(--surface);
  border-color: var(--border);
}

.modal-header {
  @apply flex items-center justify-between border-b px-5 pb-3 pt-4;
  border-bottom-color: var(--border);
}

.modal-header h2 {
  @apply m-0 text-[15px] font-semibold;
}

.close-btn {
  @apply flex cursor-pointer items-center border-none bg-transparent p-1 text-fg-mute;
}

.close-btn:hover {
  @apply text-fg;
}

.modal-body {
  @apply flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-4;
}

.modal-footer {
  @apply flex justify-end gap-2 border-t px-5 py-3;
  border-top-color: var(--border);
}

.field {
  @apply flex flex-col gap-1;
}

.field label {
  @apply text-[12px] font-medium text-fg-mute;
}

.field select,
.field input[type='text'],
.field input[type='url'] {
  @apply rounded border px-2 py-1.5 text-[13px];
  background: var(--surface-raised);
  border-color: var(--border);
  color: var(--fg);
}

.field-note {
  @apply text-[11px] text-fg-mute;
}

.hint {
  @apply m-0 text-[13px] text-fg-mute;
}

.hint.small {
  @apply text-[11px];
}

.json-preview {
  @apply max-h-[280px] overflow-auto rounded border;
  background: var(--surface-raised);
  border-color: var(--border);
}

.json-preview pre {
  @apply m-0 whitespace-pre p-3 font-mono text-[12px] leading-[1.5] text-fg;
}

.warn-box {
  @apply flex items-center gap-1.5 rounded border px-2.5 py-2 text-[12px];
  background: rgba(245, 158, 11, 0.1);
  border-color: rgba(245, 158, 11, 0.3);
  color: var(--warning, #f59e0b);
}

.btn {
  @apply flex cursor-pointer items-center gap-1.5 rounded border px-3.5 py-1.5 text-[13px];
  border-color: var(--border);
  background: var(--surface-raised);
  color: var(--fg);
}

.btn:hover {
  background: var(--surface-hover);
}

.btn.primary {
  @apply text-white;
  background: var(--accent);
  border-color: var(--accent);
}

.btn.primary:hover:not(:disabled) {
  @apply opacity-90;
}

.btn.primary:disabled {
  @apply cursor-not-allowed opacity-40;
}

.btn.ghost {
  @apply border-transparent bg-transparent;
}

.btn.ghost:hover {
  background: var(--surface-raised);
}
</style>
