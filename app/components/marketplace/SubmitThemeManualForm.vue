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

import type { MarketplaceName } from '~/types/marketplace'

interface ThemeSummary {
  id: string
  /** Rust 側 `LocalizedString` の生形 (`string | { [locale]: string }`)。表示時は `pickLocalizedName` で解決する。 */
  name: MarketplaceName
  author: string | null
  version: string
  included_roles: string[]
  is_active: boolean
}

const props = defineProps<{
  themes: ThemeSummary[]
  selectedThemeId: string | null
  tags: string[]
  allowedTags: readonly string[]
  hasKeystore: boolean
  step: 'select' | 'preview'
  githubUsername: string
  downloadUrl: string
  entryJson: string
}>()

const emit = defineEmits<{
  'update:selectedThemeId': [v: string | null]
  'update:githubUsername': [v: string]
  'update:downloadUrl': [v: string]
  'toggle-tag': [tag: string]
}>()

const { t, locale } = useI18n()

// Rust `LocalizedString` (`string | { [locale]: string }`) を現在の locale に解決して
// `${name} (v${version})` 形式に整形する。
// テンプレート内で `pickLocalizedName(...)` を直接呼ぶと unimport が script AST を見て
// import を注入できず実行時に未定義になるため、script 側で wrap する
// (`FeaturedCard.vue` の `displayName` と同じパターン)。
const themeOptions = computed(() =>
  props.themes.map((th) => ({
    value: th.id,
    label: `${pickLocalizedName(th.name, locale.value)} (v${th.version})`,
  })),
)
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
        :options="themeOptions"
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
      <span class="field-label">{{ t('marketplace.submitTagsLabel') }}</span>
      <div class="tag-toggle-grid" role="group" :aria-label="t('marketplace.submitTagsLabel')">
        <button
          v-for="tg in props.allowedTags"
          :key="tg"
          type="button"
          :class="['tag-toggle', { selected: props.tags.includes(tg) }]"
          :aria-pressed="props.tags.includes(tg)"
          :aria-label="t('marketplace.submitTagsToggleAria', { tag: tg })"
          @click="emit('toggle-tag', tg)"
        >
          <UiIcon :name="props.tags.includes(tg) ? 'Check' : 'Plus'" :size="11" />
          {{ tg }}
        </button>
      </div>
      <span class="field-note">
        {{ t('marketplace.submitTagsNote') }}
        &middot;
        {{
          t('marketplace.submitTagsCount', {
            n: props.tags.length,
            total: props.allowedTags.length,
          })
        }}
      </span>
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

.field-label,
.field label {
  @apply text-[12px] font-medium text-fg-mute;
}

.field-note {
  @apply text-[11px] text-fg-mute;
}

.tag-toggle-grid {
  @apply grid grid-cols-3 gap-1.5;
}

.tag-toggle {
  @apply inline-flex cursor-pointer items-center justify-center gap-1 rounded-full border border-line bg-transparent px-2.5 py-1 text-[12px] text-fg-dim transition-colors;
}
.tag-toggle:hover {
  border-color: var(--accent-line);
  color: var(--fg);
}
.tag-toggle.selected {
  background: var(--accent-dim);
  border-color: var(--accent-line);
  color: var(--accent);
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
