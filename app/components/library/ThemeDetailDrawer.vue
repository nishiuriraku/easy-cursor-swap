<script setup lang="ts">
/**
 * テーマ詳細ドロワー (インライン展開)
 *
 * design/library-detail.jsx の `ThemeDetailDrawer` を Vue 化したもの。
 * ThemeCard 内のシェブロンを押すと、カードが 2 列分の幅にスパンして
 * このドロワーが展開する設計 (CSS: `.card.td-open`)。
 *
 * 5 ペイン構成:
 *   1. 説明 + チェンジログ (左)
 *   2. 17 ロールカバレッジ + ライブプレビュー (右)
 *   3. パッケージ / 署名 / 使用統計 / ペアリング (帯)
 *   4. アクション群 (フッター)
 *
 * 説明文・チェンジログ・ペアリング等のデータはまだ Rust 側に IPC 経路が
 * ないので、利用可能な ThemeCardData の値からフォールバック表示する。
 * (将来的に `get_theme_detail` IPC を追加する想定)
 *
 * Windows システムスキーム (`kind: 'system'`) は署名・統計・編集系操作を
 * 全て隠す。
 */
import { computed, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { useI18n } from '~/composables/useI18n'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'

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
 * ホットスポットドット位置スタイル。
 *
 * `<img>` は親 `.td-rp-stage` に対して 64x64 の固定サイズ + place-items: center
 * で中央配置されている。ステージ寸法は CSS で 110px (高) なので、画像は
 * stage 中央の 64x64 領域を占める。ドットは画像のホットスポット位置に
 * 表示したいので、stage 中心からのオフセットを「画像表示サイズ × 比率」で算出。
 *
 * width/height = 0 (PNG デコード失敗) や詳細未取得時は中央 (50%/50%) フォールバック。
 */
const HOT_DOT_DISPLAY_SIZE = 64
const hotspotStyle = computed(() => {
  const detail = activePreviewDetail.value
  if (!detail) {
    return { left: '50%', top: '50%' }
  }
  // detail.hotspot は ratio (0.0-1.0)。画像中心からの px オフセットに変換。
  const offsetX = (detail.hotspot.x - 0.5) * HOT_DOT_DISPLAY_SIZE
  const offsetY = (detail.hotspot.y - 0.5) * HOT_DOT_DISPLAY_SIZE
  return {
    left: `calc(50% + ${offsetX}px)`,
    top: `calc(50% + ${offsetY}px)`,
  }
})

function selectRole(id: string) {
  activeRole.value = id
}

const description = computed(() => {
  if (isSystem.value) {
    return 'Windows のマウスのプロパティに保存された配色スキームです。EasyCursorSwap では適用のみ可能で、編集・エクスポート・署名検証は行いません。'
  }
  return `カバレッジ ${coverage.value}/17 役割。詳細な説明は将来のリリースで .cursorpack のメタデータから読み込む予定です。`
})

const tags = computed<string[]>(() => {
  const out: string[] = []
  if (isSystem.value) out.push('system')
  else out.push('cursor')
  if (coverage.value === 17) out.push('complete')
  if (props.theme.applyCount > 0) out.push('used')
  return out
})
</script>

<template>
  <div class="td-drawer">
    <!-- 上段: 説明 / チェンジログ | 17 ロールカバレッジ -->
    <div class="td-grid">
      <section class="td-pane">
        <header class="td-pane-h">
          <span class="td-pane-k">DESCRIPTION</span>
        </header>
        <p class="td-desc">{{ description }}</p>

        <div class="td-tags">
          <span v-for="tag in tags" :key="tag" class="td-tag">{{ tag }}</span>
          <span v-if="!isSystem" class="td-tag td-tag-on">
            <UiIcon name="Shield" :size="10" />signed
          </span>
        </div>

        <header v-if="!isSystem" class="td-pane-h" style="margin-top: 18px">
          <span class="td-pane-k">VERSION</span>
        </header>
        <ul v-if="!isSystem" class="td-changelog">
          <li class="current">
            <span class="td-cv">v{{ theme.version }}</span>
            <span class="td-cd">{{ theme.date.slice(0, 10) }}</span>
            <span class="td-cm">{{ t('themePicker.latestVersionNote') }}</span>
          </li>
        </ul>
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
                <img
                  v-if="activePreviewUrl"
                  :src="activePreviewUrl"
                  :alt="activeRoleDef.jp"
                  draggable="false"
                  style="width: 64px; height: 64px; image-rendering: pixelated"
                />
                <CursorIcon v-else :role="activeRoleDef.id" :size="64" style="color: var(--fg)" />
                <span
                  v-if="activePreviewUrl && activePreviewDetail"
                  class="td-rp-hot"
                  :style="hotspotStyle"
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

    <!-- 中段: パッケージ情報帯 (system では一部のみ) -->
    <div class="td-strip">
      <div class="td-cell">
        <div class="td-cell-k">PACKAGE</div>
        <div class="td-cell-v mono">
          {{ isSystem ? '— (system scheme)' : `${theme.id.slice(0, 8)}.cursorpack` }}
        </div>
        <div class="td-cell-sub">
          <span>{{ coverage }} roles</span>
          <span class="td-dot">·</span>
          <span>{{ isSystem ? 'system' : 'schema v3.2' }}</span>
        </div>
      </div>

      <div class="td-cell">
        <div class="td-cell-k">
          SIGNATURE
          <span v-if="isSystem" class="td-pill">N/A</span>
          <span v-else class="td-pill ok"> <UiIcon name="Shield" :size="9" />Ed25519 </span>
        </div>
        <div class="td-cell-v mono trunc">
          {{ isSystem ? 'Windows OS スキーム' : 'key_id 検証中…' }}
        </div>
        <div class="td-cell-sub mono">
          {{ isSystem ? 'EasyCursorSwap の署名対象外' : '適用時にハッシュ照合' }}
        </div>
      </div>

      <div class="td-cell">
        <div class="td-cell-k">USAGE</div>
        <div class="td-cell-v">
          <span :style="{ color: theme.applyCount > 0 ? 'var(--fg)' : 'var(--fg-mute)' }">
            {{ theme.applyCount }}
          </span>
          <span style="color: var(--fg-dim); font-size: 12px; font-weight: 400; margin-left: 4px">
            回適用
          </span>
        </div>
        <div class="td-cell-sub">
          <span>{{ theme.isActive ? '現在適用中' : '未適用' }}</span>
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
          <span>{{ isSystem ? 'OS レジストリ' : `v${theme.version}` }}</span>
        </div>
      </div>
    </div>

    <!-- 下段: アクションレール -->
    <footer class="td-foot">
      <div class="td-foot-l">
        <template v-if="!isSystem">
          <button
            class="td-act"
            :aria-label="`${theme.name} を Creator で編集`"
            @click="emit('edit', theme.id)"
          >
            <UiIcon name="Brush" :size="13" />Creator で編集
          </button>
          <button
            class="td-act"
            :aria-label="`${theme.name} をエクスポート`"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />エクスポート
          </button>
          <button
            class="td-act"
            :aria-label="`${theme.name} を複製`"
            @click="emit('duplicate', theme.id)"
          >
            <UiIcon name="Plus" :size="13" />複製
          </button>
          <button
            class="td-act danger"
            :aria-label="`${theme.name} を削除`"
            @click="emit('delete', theme.id)"
          >
            削除
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
            :aria-label="`${theme.name} を .cursorpack としてエクスポート`"
            @click="emit('exportPack', theme.id)"
          >
            <UiIcon name="Export" :size="13" />.cursorpack に書き出し
          </button>
          <span class="td-source mono">
            <UiIcon name="Globe" :size="11" />システムスキームは編集・複製不可
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
          <UiIcon name="Check" :size="13" />適用中
        </button>
        <button v-else class="btn primary" style="height: 32px" @click="emit('apply', theme.id)">
          テーマを適用
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

.td-changelog {
  @apply m-0 flex flex-col overflow-hidden rounded-lg border border-line p-0;
  list-style: none;
}
.td-changelog li {
  @apply grid items-baseline gap-3 border-b border-line px-3 py-2.5 text-[12px];
  grid-template-columns: 60px 88px 1fr;
}
.td-changelog li:last-child {
  @apply border-b-0;
}
.td-changelog li.current {
  background: rgba(124, 242, 212, 0.04);
}
:where(html.light) .td-changelog li.current {
  background: rgba(15, 168, 133, 0.05);
}
.td-cv {
  @apply font-mono text-[11px] font-semibold text-fg;
}
.td-changelog li.current .td-cv {
  @apply text-accent;
}
.td-cd {
  @apply font-mono text-[10.5px] text-fg-mute;
}
.td-cm {
  @apply leading-[1.5] text-fg-dim;
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
.td-rp-hot {
  @apply pointer-events-none absolute size-[7px] -translate-x-1/2 -translate-y-1/2 rounded-full;
  border: 1.5px solid var(--accent);
  background: rgba(124, 242, 212, 0.3);
  box-shadow: 0 0 10px var(--accent);
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
  grid-template-columns: 1.2fr 1.4fr 1fr 1.2fr;
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
