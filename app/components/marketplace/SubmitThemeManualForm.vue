<script setup lang="ts">
/**
 * 手動 GitHub 提出フォーム (SubmitThemeDialog の Manual タブ抽出)。
 *
 * Step 1: テーマ選択 + GitHub ユーザー名 + ダウンロード URL + タグ
 * Step 2: 生成された entry JSON プレビュー + GitHub Web Editor へのリンク
 *
 * 親 (SubmitThemeDialog) は step / githubUsername / downloadUrl を v-model 互換の
 * props/emit で保持する。これにより親のフッターが state を見て表示制御できる。
 */
import { useI18n } from '~/composables/useI18n'

interface ThemeSummary {
  id: string
  name: string
  author: string | null
  version: string
  included_roles: string[]
  is_active: boolean
}

const props = defineProps<{
  themes: ThemeSummary[]
  selectedThemeId: string | null
  tags: string[]
  tagInput: string
  hasKeystore: boolean
  maxTags: number
  step: 'select' | 'preview'
  githubUsername: string
  downloadUrl: string
  entryJson: string
}>()

const emit = defineEmits<{
  'update:selectedThemeId': [v: string | null]
  'update:tagInput': [v: string]
  'update:githubUsername': [v: string]
  'update:downloadUrl': [v: string]
  'commit-tag': []
  'remove-tag': [i: number]
  'tag-keydown': [e: KeyboardEvent]
}>()

const { t } = useI18n()
</script>

<template>
  <!-- Step 1: テーマ選択 + GitHub ユーザー名 -->
  <template v-if="props.step === 'select'">
    <p class="hint">{{ t('marketplace.submitHint') }}</p>

    <div class="field">
      <label for="submit-theme">{{ t('marketplace.submitSelectTheme') }}</label>
      <UiSelect
        :model-value="props.selectedThemeId"
        width="100%"
        :placeholder="t('marketplace.submitSelectPlaceholder')"
        :options="
          props.themes.map((th) => ({
            value: th.id,
            label: `${th.name} (v${th.version})`,
          }))
        "
        @update:model-value="(v: string | null) => emit('update:selectedThemeId', v)"
      />
    </div>

    <div class="field">
      <label for="submit-github">{{ t('marketplace.submitGithubUser') }}</label>
      <input
        id="submit-github"
        :value="props.githubUsername"
        type="text"
        class="input"
        :placeholder="t('marketplace.submitGithubUserPlaceholder')"
        @input="emit('update:githubUsername', ($event.target as HTMLInputElement).value)"
      />
    </div>

    <div class="field">
      <label for="submit-dl-url">{{ t('marketplace.submitDownloadUrl') }}</label>
      <input
        id="submit-dl-url"
        :value="props.downloadUrl"
        type="url"
        class="input"
        :placeholder="t('marketplace.submitDownloadUrlPlaceholder')"
        @input="emit('update:downloadUrl', ($event.target as HTMLInputElement).value)"
      />
      <span class="field-note">{{ t('marketplace.submitDownloadUrlNote') }}</span>
    </div>

    <div class="field">
      <label for="submit-manual-tags">{{ t('marketplace.submitTagsLabel') }}</label>
      <div class="tag-input-row">
        <span v-for="(tg, i) in props.tags" :key="`${tg}-${i}`" class="tag-chip">
          {{ tg }}
          <button
            type="button"
            class="tag-chip-x"
            :aria-label="t('marketplace.submitTagRemoveAria', { tag: tg })"
            @click="emit('remove-tag', i)"
          >
            <UiIcon name="X" :size="10" />
          </button>
        </span>
        <input
          id="submit-manual-tags"
          :value="props.tagInput"
          type="text"
          class="tag-input"
          :placeholder="
            props.tags.length === 0
              ? t('marketplace.submitTagsPlaceholder')
              : t('marketplace.submitTagsPlaceholderAdd')
          "
          :disabled="props.tags.length >= props.maxTags"
          @input="emit('update:tagInput', ($event.target as HTMLInputElement).value)"
          @keydown="emit('tag-keydown', $event)"
          @blur="emit('commit-tag')"
        />
      </div>
      <span class="field-note">{{ t('marketplace.submitTagsNote') }}</span>
    </div>

    <div v-if="!props.hasKeystore" class="warn-box">
      <UiIcon name="AlertTriangle" :size="14" />
      {{ t('marketplace.submitNoKeystore') }}
    </div>
  </template>

  <!-- Step 2: JSON プレビュー -->
  <template v-else>
    <p class="hint">{{ t('marketplace.submitPreviewHint') }}</p>
    <div class="json-preview">
      <pre>{{ props.entryJson }}</pre>
    </div>
    <p class="hint small">{{ t('marketplace.submitFillInNote') }}</p>
  </template>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.field {
  @apply flex flex-col gap-1;
}

.field label {
  @apply text-[12px] font-medium text-fg-mute;
}

.field-note {
  @apply text-[11px] text-fg-mute;
}

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

.warn-box {
  @apply flex items-center gap-1.5 rounded-[8px] border px-2.5 py-2 text-[12px];
  background: rgba(245, 194, 107, 0.1);
  border-color: rgba(245, 194, 107, 0.3);
  color: var(--amber);
}
</style>
