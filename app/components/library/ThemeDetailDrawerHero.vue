<script setup lang="ts">
/**
 * ThemeDetailDrawer の上段 (DESCRIPTION + ROLE COVERAGE) を担う子コンポーネント。
 *
 * 17 ロールカバレッジボタン + アクティブロールのプレビューはこの SFC 内で完結し、
 * 親 (ThemeDetailDrawer) は activeRole を知る必要がない (= props down 単方向)。
 *
 * `.ani` プレビューは `<CursorPreview kind="ani">` 経路で AniThumb (useAniPlayer 内蔵)
 * に合流させているので、ロール切替時に再マウントされて blob URL は自動 revoke される。
 */
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'
import type { CursorPreviewAsset } from '~/components/preview/CursorPreview.vue'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
  previewMap: Record<string, string> | null
  previewDetails?: Record<string, RolePreviewDetail> | null
}>()

const isSystem = computed(() => props.theme.kind === 'system')

const includedSet = computed(() => new Set(props.theme.includedRoles))
const coverage = computed(() => props.theme.includedRoles.length)

// 17 ロールのうちアクティブにプレビュー中のロール。
// `CURSOR_ROLES` の並び順 (= ロールカバレッジグリッドの並び順) で最初に
// 含まれているロールを既定とする。`includedRoles[0]` をそのまま使うと
// テーマパッケージ側の格納順に左右されて、UI のグリッド左上のボタンと
// アクティブ表示がズレる (ユーザー体感: 「最初のものが選ばれない」)。
const initialActiveRole =
  CURSOR_ROLES.find((r) => includedSet.value.has(r.id))?.id ?? CURSOR_ROLES[0]!.id
const activeRole = ref<string>(initialActiveRole)

const activeRoleDef = computed(
  () => CURSOR_ROLES.find((r) => r.id === activeRole.value) ?? CURSOR_ROLES[0]!,
)
const activeIncluded = computed(() => includedSet.value.has(activeRole.value))
const activePreviewUrl = computed(() => props.previewMap?.[activeRole.value] ?? null)
const activePreviewDetail = computed<RolePreviewDetail | null>(
  () => props.previewDetails?.[activeRole.value] ?? null,
)

const activePreviewAsset = computed<CursorPreviewAsset>(() => {
  const ani = activePreviewDetail.value?.aniFrames
  if (ani) {
    return {
      kind: 'ani',
      framePngs: ani.framePngs,
      sequence: ani.sequence,
      durations: ani.durations,
      nativeSize: ani.nativeSize,
    }
  }
  const url = activePreviewUrl.value
  if (url) return { kind: 'static', url, alt: activeRoleDef.value.jp }
  return { kind: 'empty' }
})

const previewHotspot = computed(() => activePreviewDetail.value?.hotspot ?? { x: 0.5, y: 0.5 })
const hideHotDot = computed(() => !activePreviewDetail.value)

function selectRole(id: string) {
  activeRole.value = id
}

const descriptionText = computed<string | null>(() => {
  if (props.theme.description) return props.theme.description
  if (isSystem.value) return t('themeDetail.systemSchemeDesc')
  return null
})

const tagsToShow = computed<string[]>(() => props.theme.tags ?? [])
const hasSigned = computed(() => props.theme.signed === true)

/**
 * 左ペイン (DESCRIPTION) に出すものが何かあるかどうか。
 * description / tags / signed バッジが全て無い場合は左ペインごと
 * 隠して、ROLE COVERAGE を 1 列で広げる。
 */
const hasLeftPaneContent = computed(
  () => descriptionText.value !== null || tagsToShow.value.length > 0 || hasSigned.value,
)
</script>

<template>
  <div :class="['td-grid', { 'td-grid-single': !hasLeftPaneContent }]">
    <section v-if="hasLeftPaneContent" class="td-pane">
      <header v-if="descriptionText" class="td-pane-h">
        <span class="td-pane-k">DESCRIPTION</span>
      </header>
      <p v-if="descriptionText" class="td-desc">{{ descriptionText }}</p>

      <div v-if="tagsToShow.length > 0 || hasSigned" class="td-tags">
        <span v-for="tag in tagsToShow" :key="tag" class="td-tag">{{ tag }}</span>
        <span v-if="hasSigned" class="td-tag td-tag-on">
          <UiIcon name="Shield" :size="10" />{{ t('themeDetail.officialBadge') }}
        </span>
      </div>
    </section>

    <section class="td-pane">
      <header class="td-pane-h">
        <span class="td-pane-k">ROLE COVERAGE</span>
        <span class="td-pane-r">
          <span style="color: var(--accent)">{{ coverage }}</span>
          <span style="color: var(--fg-faint)">/</span>
          <span>17</span>
        </span>
      </header>

      <div class="td-cov">
        <div class="td-rolegrid">
          <button
            v-for="role in CURSOR_ROLES"
            :key="role.id"
            :class="[
              'td-rolebtn',
              {
                empty: !includedSet.has(role.id),
                active: activeRole === role.id,
              },
            ]"
            :title="role.jp"
            @click="selectRole(role.id)"
          >
            <CursorIcon v-if="includedSet.has(role.id)" :role="role.id" :size="14" />
            <span v-else class="td-rb-x">×</span>
          </button>
        </div>

        <div class="td-rolepreview">
          <div class="td-rp-stage">
            <template v-if="activeIncluded">
              <CursorPreview
                :asset="activePreviewAsset"
                :hotspot="previewHotspot"
                :role-id="activeRoleDef.id"
                :display-pct="100"
                :fallback-icon-size="64"
                :hide-dot="hideHotDot"
                class="td-rp-preview"
              />
            </template>
            <div v-else class="td-rp-missing">
              <UiIcon name="Alert" :size="20" />
              <span>{{ t('themePicker.roleMissing') }}</span>
            </div>
          </div>
          <div class="td-rp-meta">
            <div class="td-rp-name">{{ activeRoleDef.jp }}</div>
            <div class="td-rp-key">
              <code>{{ activeRoleDef.id }}</code>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-grid {
  @apply grid border-b border-line;
  grid-template-columns: 1fr 1.15fr;
}
.td-grid.td-grid-single {
  grid-template-columns: 1fr;
}
.td-pane {
  @apply min-w-0 px-[22px] pb-5 pt-[18px];
}
.td-pane + .td-pane {
  @apply border-l border-line;
}
.td-pane-h {
  @apply mb-3 flex items-center justify-between;
}
.td-pane-k {
  @apply font-mono text-[9.5px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}
.td-pane-r {
  @apply inline-flex items-baseline gap-[3px] font-mono text-[11px] tracking-[0.04em] text-fg;
}
.td-pane-r > :first-child {
  @apply text-[14px] font-semibold;
}

.td-desc {
  @apply m-0 text-[13px] leading-[1.6] text-fg-dim;
  text-wrap: pretty;
}

.td-tags {
  @apply mt-3.5 flex flex-wrap gap-1.5;
}
.td-tag {
  @apply inline-flex h-[22px] items-center gap-[5px] rounded border border-line px-[9px] font-mono text-[10px] tracking-[0.04em] text-fg-dim;
  background: rgba(255, 255, 255, 0.025);
}
:where(html.light) .td-tag {
  background: rgba(15, 20, 35, 0.025);
}
.td-tag-on {
  @apply border-accent-line bg-accent-dim text-accent;
}

.td-cov {
  @apply grid items-start gap-[18px];
  grid-template-columns: 1fr 240px;
}
.td-rolegrid {
  @apply grid gap-1;
  grid-template-columns: repeat(9, 1fr);
}
.td-rolebtn {
  @apply relative grid cursor-pointer place-items-center rounded-md border border-line p-0 text-fg;
  aspect-ratio: 1;
  background: rgba(255, 255, 255, 0.03);
  transition: all 0.12s;
}
:where(html.light) .td-rolebtn {
  background: rgba(15, 20, 35, 0.025);
}
.td-rolebtn:hover {
  @apply border-line-strong;
  background: rgba(255, 255, 255, 0.06);
}
.td-rolebtn.empty {
  @apply text-fg-faint;
  background: repeating-linear-gradient(
    -45deg,
    rgba(255, 255, 255, 0.025),
    rgba(255, 255, 255, 0.025) 3px,
    transparent 3px,
    transparent 6px
  );
  border-color: rgba(255, 255, 255, 0.05);
}
:where(html.light) .td-rolebtn.empty {
  background: repeating-linear-gradient(
    -45deg,
    rgba(15, 20, 35, 0.04),
    rgba(15, 20, 35, 0.04) 3px,
    transparent 3px,
    transparent 6px
  );
}
.td-rolebtn.active {
  @apply bg-accent-dim text-accent;
  border-color: var(--accent);
  box-shadow:
    0 0 0 1px var(--accent),
    0 0 16px -4px var(--accent);
}
.td-rb-x {
  @apply font-mono text-[14px] text-fg-faint;
}
.td-rolepreview {
  @apply flex flex-col gap-3 rounded-[10px] border border-line-hi p-3.5;
  background:
    radial-gradient(80% 80% at 50% 0%, rgba(124, 242, 212, 0.04), transparent 60%),
    rgba(0, 0, 0, 0.2);
}
:where(html.light) .td-rolepreview {
  background:
    radial-gradient(80% 80% at 50% 0%, rgba(15, 168, 133, 0.05), transparent 60%),
    rgba(15, 20, 35, 0.02);
}
.td-rp-stage {
  @apply relative grid h-[110px] place-items-center rounded-lg border border-line;
  background:
    repeating-conic-gradient(rgba(255, 255, 255, 0.025) 0% 25%, transparent 0% 50%) 0 / 12px 12px,
    rgba(0, 0, 0, 0.25);
}
:where(html.light) .td-rp-stage {
  background:
    repeating-conic-gradient(rgba(15, 20, 35, 0.035) 0% 25%, transparent 0% 50%) 0 / 12px 12px,
    rgba(255, 255, 255, 0.5);
}
.td-rp-preview {
  @apply size-[64px];
}
.td-rp-missing {
  @apply flex flex-col items-center gap-1.5 font-mono text-[11px] tracking-[0.08em];
  color: var(--rose);
}
.td-rp-meta {
  @apply flex flex-col gap-1.5;
}
.td-rp-name {
  @apply font-display text-[14px] font-semibold tracking-[-0.01em];
}
.td-rp-key code {
  @apply font-mono text-[10.5px] tracking-[0.04em] text-fg-mute;
}
</style>
