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
  allowedTags: readonly string[]
  hasKeystore: boolean
  githubAccount: GithubAccount | null
  submitterBusy: boolean
  submitterStageLabel: string
  submitterErrorMsg: string | null
  submitDone: boolean
}>()

const emit = defineEmits<{
  'update:selectedThemeId': [v: string | null]
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
        t('marketplace.submitTagsCount', { n: props.tags.length, total: props.allowedTags.length })
      }}
    </span>
  </div>

  <div v-if="props.githubAccount" class="hint">
    {{ t('marketplace.submitAutoLinkedAs', { login: props.githubAccount.login }) }}
  </div>
  <div v-else class="hint">{{ t('marketplace.submitAutoNeedsLink') }}</div>

  <div v-if="props.submitterBusy" class="hint" aria-live="polite">
    {{ props.submitterStageLabel }}
  </div>
  <UiAlert v-if="props.submitterErrorMsg" tone="warn">
    {{ props.submitterErrorMsg }}
  </UiAlert>
  <div v-if="props.submitDone" class="hint">
    {{ t('marketplace.submitDone') }}
  </div>

  <UiAlert v-if="!props.hasKeystore" tone="warn">
    {{ t('marketplace.submitNoKeystore') }}
  </UiAlert>
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
</style>
