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
          <button
            type="button"
            class="btn ghost btn-close"
            :aria-label="t('common.close')"
            @click="close"
          >
            <UiIcon name="X" :size="14" />
          </button>
        </div>

        <div class="modal-body submit-body">
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

        <div class="modal-foot">
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
        </div>
      </div>
    </div>
  </Teleport>
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

.btn-close {
  @apply size-7 shrink-0 px-0;
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

.warn-box {
  @apply flex items-center gap-1.5 rounded-[8px] border px-2.5 py-2 text-[12px];
  background: rgba(245, 194, 107, 0.1);
  border-color: rgba(245, 194, 107, 0.3);
  color: var(--amber);
}
</style>
