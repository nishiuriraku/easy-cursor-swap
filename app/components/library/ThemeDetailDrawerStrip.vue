<script setup lang="ts">
/**
 * ThemeDetailDrawer の中段帯 (PACKAGE / USAGE / SOURCE の 3 セル) を担う子コンポーネント。
 *
 * 純粋表示: theme から派生する displaySize / lastAppliedDate / coverage / isSystem の
 * 計算と、homepage の外部ブラウザオープン (useExternalUrl) のみを行う。
 */
import { computed } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'
import { openExternalUrl } from '~/composables/useExternalUrl'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const isSystem = computed(() => props.theme.kind === 'system')
const coverage = computed(() => props.theme.includedRoles.length)

const displaySize = computed<string | null>(() => {
  const b = props.theme.sizeBytes
  if (b == null || b === 0) return null
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${(b / (1024 * 1024)).toFixed(1)} MB`
})

const lastAppliedDate = computed<string | null>(() => {
  const d = props.theme.lastAppliedAt
  if (!d) return null
  return d.slice(0, 10)
})

async function openHomepage() {
  const url = props.theme.homepage
  if (!url) return
  await openExternalUrl(url)
}
</script>

<template>
  <div class="td-strip">
    <div class="td-cell">
      <div class="td-cell-k">PACKAGE</div>
      <div class="td-cell-v mono">
        <span>{{ coverage }} roles</span>
        <template v-if="displaySize">
          <span class="td-dot">·</span>
          <span>{{ displaySize }}</span>
        </template>
      </div>
      <div class="td-cell-sub">
        {{ isSystem ? 'system scheme' : `schema v${theme.schemaVersion ?? '?'}` }}
      </div>
    </div>

    <div class="td-cell">
      <div class="td-cell-k">USAGE</div>
      <div class="td-cell-v">
        <span :style="{ color: theme.applyCount > 0 ? 'var(--fg)' : 'var(--fg-mute)' }">
          {{ theme.applyCount }}
        </span>
        <span style="color: var(--fg-dim); font-size: 12px; font-weight: 400; margin-left: 4px">
          {{ t('themeDetail.applyCountSuffix') }}
        </span>
      </div>
      <div class="td-cell-sub">
        <span>
          {{
            theme.isActive
              ? t('themeDetail.usageActive')
              : theme.applyCount > 0
                ? t('themeDetail.usageInactive')
                : t('themeDetail.usageNever')
          }}
        </span>
        <template v-if="lastAppliedDate">
          <span class="td-dot">·</span>
          <span>{{ t('themePicker.lastAppliedPrefix') }} {{ lastAppliedDate }}</span>
        </template>
      </div>
    </div>

    <div class="td-cell">
      <div class="td-cell-k">SOURCE</div>
      <div class="td-cell-v">
        <UiIcon
          :name="isSystem ? 'Globe' : 'Pkg'"
          :size="11"
          :style="`color: var(${isSystem ? '--violet' : '--accent'}); margin-right: 6px`"
        />
        {{ isSystem ? 'HKCU\\Cursors\\Schemes' : `@${theme.author ?? 'unknown'}` }}
      </div>
      <div class="td-cell-sub">
        <span>{{ isSystem ? t('themeDetail.sourceOsRegistry') : `v${theme.version}` }}</span>
        <template v-if="!isSystem && theme.license">
          <span class="td-dot">·</span>
          <span>{{ theme.license }}</span>
        </template>
        <template v-if="!isSystem && theme.homepage">
          <span class="td-dot">·</span>
          <button
            type="button"
            class="td-pane-link"
            :aria-label="t('themePicker.openHomepage')"
            @click="openHomepage"
          >
            {{ t('themePicker.openHomepage') }}
          </button>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-pane-link {
  @apply cursor-pointer border-0 bg-transparent p-0 font-mono text-[11px] tracking-[0.04em] text-accent;
}
.td-pane-link:hover {
  color: var(--accent-hi);
}

.td-strip {
  @apply grid border-b border-line;
  grid-template-columns: 1.2fr 1.2fr 1.6fr;
}
.td-cell {
  @apply flex min-w-0 flex-col gap-1 border-r border-line px-[18px] py-3.5;
}
.td-cell:last-child {
  @apply border-r-0;
}
.td-cell-k {
  @apply mb-1 flex items-center gap-2 font-mono text-[9.5px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}
.td-cell-v {
  @apply flex items-center text-[14px] font-medium text-fg;
  letter-spacing: -0.005em;
}
.td-cell-v.mono {
  @apply font-mono text-[12px] font-medium;
}
.td-cell-sub {
  @apply flex flex-wrap items-center gap-[5px] text-[11px] text-fg-mute;
}
.td-dot {
  @apply text-fg-faint;
}
</style>
