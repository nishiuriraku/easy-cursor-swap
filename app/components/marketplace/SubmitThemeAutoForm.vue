<script setup lang="ts">
/**
 * 自動 GitHub 提出フォーム (SubmitThemeDialog の Auto タブ抽出)。
 *
 * Device Flow リンク済みなら直接 PR を作成、未リンクなら親に linkGithub を emit してモーダルを開かせる。
 * submitter (useMarketplaceSubmit) は親が保持し、reactive な値を props で受け取るだけにする
 * (useMarketplaceSubmit は singleton ではないため、子側で呼び直すと state が分裂する)。
 */
import type { GithubAccount } from '~/types/githubAuth'
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
  tagInput: string
  hasKeystore: boolean
  maxTags: number
  githubAccount: GithubAccount | null
  submitterBusy: boolean
  submitterStageLabel: string
  submitterErrorMsg: string | null
  submitDone: boolean
}>()

const emit = defineEmits<{
  'update:selectedThemeId': [v: string | null]
  'update:tagInput': [v: string]
  'commit-tag': []
  'remove-tag': [i: number]
  'tag-keydown': [e: KeyboardEvent]
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
  <p class="hint">{{ t('marketplace.submitAutoIntro') }}</p>

  <div class="field">
    <label for="submit-auto-theme">{{ t('marketplace.submitSelectTheme') }}</label>
    <UiSelect
      id="submit-auto-theme"
      :model-value="props.selectedThemeId"
      width="100%"
      :placeholder="t('marketplace.submitSelectPlaceholder')"
      :options="themeOptions"
      @update:model-value="(v: string | null) => emit('update:selectedThemeId', v)"
    />
  </div>

  <div class="field">
    <label for="submit-auto-tags">{{ t('marketplace.submitTagsLabel') }}</label>
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
        id="submit-auto-tags"
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

  <div v-if="props.githubAccount" class="hint">
    {{ t('marketplace.submitAutoLinkedAs', { login: props.githubAccount.login }) }}
  </div>
  <div v-else class="hint">{{ t('marketplace.submitAutoNeedsLink') }}</div>

  <div v-if="props.submitterBusy" class="hint" aria-live="polite">
    {{ props.submitterStageLabel }}
  </div>
  <div v-if="props.submitterErrorMsg" class="warn-box">
    <UiIcon name="AlertTriangle" :size="14" />
    {{ props.submitterErrorMsg }}
  </div>
  <div v-if="props.submitDone" class="hint">
    {{ t('marketplace.submitDone') }}
  </div>

  <div v-if="!props.hasKeystore" class="warn-box">
    <UiIcon name="AlertTriangle" :size="14" />
    {{ t('marketplace.submitNoKeystore') }}
  </div>
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

.warn-box {
  @apply flex items-center gap-1.5 rounded-[8px] border px-2.5 py-2 text-[12px];
  background: rgba(245, 194, 107, 0.1);
  border-color: rgba(245, 194, 107, 0.3);
  color: var(--amber);
}
</style>
