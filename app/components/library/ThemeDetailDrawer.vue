<script setup lang="ts">
/**
 * テーマ詳細ドロワー (モーダル本体)
 *
 * design/library-detail.jsx の `ThemeDetailDrawer` を Vue 化したもの。
 * 3 段構成:
 *   1. DESCRIPTION ペイン (説明文 + tags + signed pill) | ROLE COVERAGE ペイン
 *   2. PACKAGE / USAGE / SOURCE の 3 セル strip
 *   3. アクション群 (フッター)
 *
 * 表示する全ての値は ThemeCardData (= Rust ThemeSummary) から導出する。
 * description / license / homepage / lastAppliedAt 等が無いフィールドは
 * 行ごと非表示にし、プレースホルダ文字列を出さない。
 *
 * Windows システムスキーム (`kind: 'system'`) は署名・統計・編集系操作を
 * 全て隠す。
 */
import { computed, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { useI18n } from '~/composables/useI18n'
import { invokeTauri } from '~/composables/useTauri'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'
import type { CursorPreviewAsset } from '~/components/preview/CursorPreview.vue'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon のフォールバックを表示。 */
  previewMap: Record<string, string> | null
  /**
   * 役割名 → ホットスポット詳細 (寸法 + ホットスポット座標) のマップ。
   * `previewMap` と組で渡されることを想定。null や未取得ロールはホットスポットドット非表示。
   */
  previewDetails?: Record<string, RolePreviewDetail> | null
}>()

const emit = defineEmits<{
  apply: [id: string]
  close: []
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
  openSource: [id: string]
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

/**
 * `<CursorPreview>` に渡す asset。preview URL があれば static、なければ empty。
 * Drawer では ANI 描画は行わない (PNG プレビューのみ)。
 */
const activePreviewAsset = computed<CursorPreviewAsset>(() => {
  const url = activePreviewUrl.value
  if (url) return { kind: 'static', url, alt: activeRoleDef.value.jp }
  return { kind: 'empty' }
})

/**
 * `<CursorPreview>` に渡すホットスポット。詳細が未取得時は中央 (0.5, 0.5) フォールバック。
 */
const previewHotspot = computed(() => activePreviewDetail.value?.hotspot ?? { x: 0.5, y: 0.5 })

/**
 * 詳細が未取得 (width/height = 0 等) のときは dot を隠す。
 */
const hideHotDot = computed(() => !activePreviewDetail.value)

function selectRole(id: string) {
  activeRole.value = id
}

const descriptionText = computed<string | null>(() => {
  if (props.theme.description) return props.theme.description
  if (isSystem.value) {
    return t('themeDetail.systemSchemeDesc')
  }
  return null
})

const tagsToShow = computed<string[]>(() => props.theme.tags ?? [])

const hasSigned = computed(() => props.theme.signed === true)

/**
 * 左ペイン (DESCRIPTION) に出すものが何かあるかどうか。
 * description / tags / signed バッジが全て無い場合は左ペインごと
 * 隠して、ROLE COVERAGE を 1 列で広げる (CSS 側 .td-grid-single)。
 * これがないと「見出しだけ出るが中身が無い」見た目になる (空ライブラリ
 * テーマで頻発: theme.json に description フィールドが無いケース)。
 */
const hasLeftPaneContent = computed(
  () => descriptionText.value !== null || tagsToShow.value.length > 0 || hasSigned.value,
)

// バイト → 表示文字列。null は呼び出し側で行ごと省略する。
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
  try {
    await invokeTauri<void>('open_url', { url })
  } catch {
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}
</script>

<template>
  <div class="td-drawer">
    <!-- 上段: 説明 / チェンジログ | 17 ロールカバレッジ -->
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

    <!-- 中段: パッケージ情報帯 (実データ駆動、3 セル構成) -->
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

    <!-- 下段: アクションレール -->
    <footer class="td-foot">
      <div class="td-foot-l">
        <template v-if="!isSystem">
          <button
            class="td-act"
            :aria-label="t('themeDetail.editAria', { name: theme.name })"
            @click="emit('edit', theme.id)"
          >
            <UiIcon name="Brush" :size="13" />{{ t('themeDetail.editLabel') }}
          </button>
          <button
            class="td-act"
            :aria-label="t('themeDetail.exportAria', { name: theme.name })"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />{{ t('themeDetail.exportLabel') }}
          </button>
          <button
            class="td-act"
            :aria-label="t('themeDetail.duplicateAria', { name: theme.name })"
            @click="emit('duplicate', theme.id)"
          >
            <UiIcon name="Plus" :size="13" />{{ t('themeDetail.duplicateLabel') }}
          </button>
          <button
            class="td-act danger"
            :aria-label="t('themeDetail.deleteAria', { name: theme.name })"
            @click="emit('delete', theme.id)"
          >
            {{ t('themeDetail.deleteLabel') }}
          </button>
        </template>
        <template v-else>
          <!--
            システムスキームは編集・複製・削除はできないが、`.cursorpack`
            として書き出して別環境へ持ち運ぶことはできる。Rust 側の
            `export_windows_scheme_as_cursorpack` が `%SystemRoot%\cursors\*`
            を読み取って zip 化するため、ローカルディレクトリは不要。
          -->
          <button
            class="td-act"
            :aria-label="t('themeDetail.exportSchemeAria', { name: theme.name })"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />{{ t('themeDetail.exportSchemeLabel') }}
          </button>
          <span class="td-source mono">
            <UiIcon name="Globe" :size="11" />{{ t('themeDetail.systemSchemeReadOnly') }}
          </span>
        </template>
      </div>
      <div class="td-foot-r">
        <button
          v-if="theme.isActive"
          class="btn"
          disabled
          style="opacity: 0.6; cursor: default; height: 32px"
        >
          <UiIcon name="Check" :size="13" />{{ t('themeDetail.applyingNow') }}
        </button>
        <button v-else class="btn primary" style="height: 32px" @click="emit('apply', theme.id)">
          {{ t('themeDetail.applyTheme') }}
        </button>
      </div>
    </footer>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-drawer {
  @apply flex flex-col;
  background:
    linear-gradient(180deg, rgba(124, 242, 212, 0.025), transparent 40%), rgba(0, 0, 0, 0.18);
}
:where(html.light) .td-drawer {
  background:
    linear-gradient(180deg, rgba(15, 168, 133, 0.04), transparent 40%), rgba(15, 20, 35, 0.02);
}

.td-grid {
  @apply grid border-b border-line;
  grid-template-columns: 1fr 1.15fr;
}
/* description / tags / signed が無いテーマでは左ペインを丸ごと隠し、
 * ROLE COVERAGE を 1 列で広げる。see hasLeftPaneContent. */
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
.td-pane-link {
  @apply cursor-pointer border-0 bg-transparent p-0 font-mono text-[11px] tracking-[0.04em] text-accent;
}
.td-pane-link:hover {
  color: var(--accent-hi);
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
.td-cell-v.trunc {
  @apply overflow-hidden truncate whitespace-nowrap;
}
.td-cell-sub {
  @apply flex flex-wrap items-center gap-[5px] text-[11px] text-fg-mute;
}
.td-cell-sub.mono {
  @apply font-mono text-[10.5px];
}
.td-dot {
  @apply text-fg-faint;
}
.td-pill {
  @apply inline-flex h-4 items-center gap-1 rounded-[3px] border border-line px-1.5 font-mono text-[9px] font-medium tracking-[0.08em] text-fg-dim;
  background: rgba(255, 255, 255, 0.03);
}
:where(html.light) .td-pill {
  background: rgba(15, 20, 35, 0.03);
}
.td-pill.ok {
  @apply border-accent-line bg-accent-dim text-accent;
}
.td-pill.warn {
  color: var(--rose);
  border-color: rgba(255, 107, 138, 0.3);
  background: rgba(255, 107, 138, 0.08);
}

.td-foot {
  @apply flex items-center justify-between gap-3 px-[18px] py-3;
  background: rgba(0, 0, 0, 0.18);
}
:where(html.light) .td-foot {
  background: rgba(15, 20, 35, 0.025);
}
.td-foot-l {
  @apply flex items-center gap-1.5;
}
.td-foot-r {
  @apply flex items-center gap-3;
}

.td-act {
  @apply inline-flex h-[30px] cursor-pointer items-center gap-1.5 rounded-md border border-transparent bg-transparent px-[11px] text-[12px] font-medium text-fg-dim;
  transition: all 0.12s;
}
.td-act:hover {
  @apply border-line-hi text-fg;
  background: rgba(255, 255, 255, 0.05);
}
:where(html.light) .td-act:hover {
  background: rgba(15, 20, 35, 0.04);
}
.td-act.danger {
  color: var(--rose);
}
.td-act.danger:hover {
  background: rgba(255, 107, 138, 0.08);
  border-color: rgba(255, 107, 138, 0.3);
  color: #fff;
}
:where(html.light) .td-act.danger:hover {
  color: var(--rose);
}

.td-source {
  @apply inline-flex items-center gap-1.5 font-mono text-[10.5px] tracking-[0.02em] text-fg-mute;
}
</style>
