<script setup lang="ts">
/**
 * Creator モードの初期画面 (design/empty-states.jsx::CreatorStart の Vue 版)
 *
 * ヒーローブロック + 3 CTA (新規 / .cursorpack インポート / 既存複製) +
 * キーボードショートカット表示 + 最近のドラフト一覧。
 *
 * ドラフト一覧は将来の `get_drafts` IPC を想定し、現状は空配列で渡されたら
 * セクションごと非表示にするフォールバック設計。
 *
 * ヒーローの "CREATOR · vX.Y" 表示は `useAppInfo` 経由で Cargo.toml の実
 * version を表示する (旧実装はハードコード `v1.0`)。
 */
import { computed, onMounted } from 'vue'
import { useAppInfo } from '~/composables/useAppInfo'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()
const { info, load: loadAppInfo } = useAppInfo()

onMounted(() => {
  void loadAppInfo()
})

const eyebrow = computed(() => {
  const v = info.value?.version
  return v ? `CREATOR · v${v}` : 'CREATOR'
})

defineProps<{
  /** 最近編集中のドラフト一覧。空配列なら "RECENT DRAFTS" セクションを隠す。 */
  recentDrafts?: Array<{
    id: string
    name: string
    modified: string
    roleCount: number
    isDraft: boolean
  }>
}>()

const emit = defineEmits<{
  /** ヒーローの「新規作成」CTA。空のテーマで Creator モードに入る。 */
  startNew: []
  /** 既存テーマ複製。親で Library 選択モーダルを開いてから取込フローに流す。 */
  duplicateExisting: []
  /** 最近のドラフトを開く。 */
  openDraft: [id: string]
}>()
</script>

<template>
  <div class="es-stage">
    <div class="es-bg" />
    <div class="es-creator-hero">
      <div class="es-mark">
        <CursorIcon role="Arrow" :size="48" style="color: var(--accent)" />
      </div>
      <div class="es-eyebrow">{{ eyebrow }}</div>
      <h1>{{ t('creatorStart.title') }}</h1>
      <p>{{ t('creatorStart.description') }}</p>

      <div class="es-cta-row">
        <button class="btn primary" @click="emit('startNew')">
          <UiIcon name="Plus" :size="14" />
          {{ t('creatorStart.btnNew') }}
        </button>
        <button class="btn ghost" @click="emit('duplicateExisting')">
          <UiIcon name="Brush" :size="13" />{{ t('creatorStart.btnDuplicate') }}
        </button>
      </div>
    </div>

    <div v-if="recentDrafts && recentDrafts.length > 0" class="es-recent">
      <div class="es-recent-h">
        <span class="td-pane-k">RECENT DRAFTS</span>
        <span class="td-pane-link">all →</span>
      </div>
      <div class="es-recent-list">
        <button
          v-for="d in recentDrafts"
          :key="d.id"
          class="es-recent-item"
          @click="emit('openDraft', d.id)"
        >
          <div class="es-recent-thumb">
            <UiIcon :name="d.isDraft ? 'Brush' : 'Library'" :size="14" />
          </div>
          <div class="es-recent-meta">
            <div class="es-recent-name">
              {{ d.name }}
              <span v-if="d.isDraft" class="es-draft">DRAFT</span>
            </div>
            <div class="es-recent-sub">
              {{ d.modified }} · {{ t('creator.recentRoleCount', { count: d.roleCount }) }}
            </div>
          </div>
          <UiIcon
            name="ChevD"
            :size="12"
            style="color: var(--fg-mute); transform: rotate(-90deg)"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.es-stage {
  @apply relative grid flex-1 overflow-auto px-8 py-10;
  place-items: center;
}
.es-bg {
  @apply pointer-events-none absolute inset-0 z-0;
  background:
    radial-gradient(700px 400px at 50% 0%, rgba(124, 242, 212, 0.08), transparent 60%),
    radial-gradient(900px 500px at 50% 100%, rgba(139, 125, 255, 0.05), transparent 60%);
}
:where(html.light) .es-bg {
  background:
    radial-gradient(700px 400px at 50% 0%, rgba(15, 168, 133, 0.08), transparent 60%),
    radial-gradient(900px 500px at 50% 100%, rgba(106, 92, 255, 0.05), transparent 60%);
}
.es-eyebrow {
  @apply mb-2.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent;
}
.es-cta-row {
  @apply mt-6 flex flex-wrap justify-center gap-2.5;
}

.es-creator-hero {
  @apply relative z-10 flex w-[720px] max-w-full flex-col items-center text-center;
}
.es-mark {
  @apply relative mb-[22px] grid size-24 place-items-center rounded-3xl border border-accent-line;
  background:
    radial-gradient(70% 70% at 30% 30%, rgba(124, 242, 212, 0.25), rgba(124, 242, 212, 0.05) 60%),
    rgba(0, 0, 0, 0.3);
  box-shadow:
    0 24px 60px -20px rgba(124, 242, 212, 0.45),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}
:where(html.light) .es-mark {
  /* ライトテーマでは黒地の radial gradient が暗く見えるため、
     light tokens の accent (#0fa885) に寄せた淡い背景に差し替える。 */
  background:
    radial-gradient(70% 70% at 30% 30%, rgba(15, 168, 133, 0.18), rgba(15, 168, 133, 0.04) 60%),
    rgba(255, 255, 255, 0.7);
  box-shadow:
    0 24px 60px -20px rgba(15, 168, 133, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.5);
}
.es-mark::after {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: 25px;
  background: conic-gradient(
    from 0deg,
    transparent 0%,
    var(--accent) 25%,
    transparent 50%,
    var(--violet) 75%,
    transparent 100%
  );
  opacity: 0.35;
  z-index: -1;
  filter: blur(14px);
}
:where(html.light) .es-mark::after {
  /* ライトテーマでは accent token が #0fa885 になるので
     conic gradient はトークン値で自動的に追従するが、blur の不透明度を上げて視認性を確保する。 */
  opacity: 0.5;
}
.es-creator-hero h1 {
  @apply m-0 font-display text-[32px] font-semibold tracking-[-0.025em];
}
.es-creator-hero p {
  @apply mx-0 mb-0 mt-3 max-w-[520px] text-[14px] leading-[1.7] text-fg-dim;
  text-wrap: pretty;
}
.es-creator-hero p code {
  @apply rounded border border-accent-line bg-accent-dim px-[5px] py-px font-mono text-[12.5px] text-accent;
}

.es-recent {
  @apply relative z-10 mt-9 w-[720px] max-w-full overflow-hidden rounded-xl border border-line bg-bg-glass backdrop-blur-2xl;
}
.es-recent-h {
  @apply flex items-center justify-between border-b border-line px-4 py-3;
}
.es-recent-list {
  @apply flex flex-col;
}
.es-recent-item {
  @apply grid cursor-pointer items-center gap-3 border-0 border-b border-line bg-transparent px-4 py-3 text-left text-fg;
  grid-template-columns: 36px 1fr 14px;
  transition: background 0.12s;
}
.es-recent-item:last-child {
  @apply border-b-0;
}
.es-recent-item:hover {
  background: rgba(255, 255, 255, 0.03);
}
:where(html.light) .es-recent-item:hover {
  background: rgba(15, 20, 35, 0.025);
}
.es-recent-thumb {
  @apply grid size-9 place-items-center rounded-lg border border-line text-fg-dim;
  background: rgba(255, 255, 255, 0.04);
}
.es-recent-name {
  @apply flex items-center gap-2 text-[13px] font-medium;
}
.es-recent-sub {
  @apply mt-0.5 text-[11px] text-fg-mute;
}
.es-draft {
  @apply rounded-[3px] px-[5px] py-0 font-mono text-[9px] tracking-[0.12em];
  color: var(--amber);
  background: rgba(245, 194, 107, 0.1);
  border: 1px solid rgba(245, 194, 107, 0.3);
}

.td-pane-k {
  @apply font-mono text-[9.5px] font-medium uppercase tracking-[0.16em] text-fg-mute;
}
.td-pane-link {
  @apply cursor-pointer border-0 bg-transparent p-0 font-mono text-[11px] tracking-[0.04em] text-accent;
}
.td-pane-link:hover {
  color: var(--accent-hi);
}
</style>
