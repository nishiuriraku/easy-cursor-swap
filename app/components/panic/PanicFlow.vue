<script setup lang="ts">
/**
 * パニック復旧フロー (Phase 5-9)
 *
 * design/panic.jsx を Vue 化したもの。
 * 全画面オーバーレイで表示し、Ctrl+Alt+Shift+R グローバルホットキーから起動可能。
 *
 * 2 段階リセット:
 *  - Stage 1: Windows 既定 (`reset_to_default`)
 *  - Stage 2: インストール前スナップショット (`reset_to_initial`)
 *
 * 進行中はライブログ + 17 ロールグリッドで進捗を可視化。
 * 親 (default.vue) が `v-model:open` で表示制御。
 */
import { computed, ref, watch } from 'vue'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

interface LogEntry {
  /** 経過時間 (ms 文字列) */
  t?: string
  status: 'ok' | 'running' | 'pending'
  text: string
}

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [v: boolean]
  done: [stage: 1 | 2]
}>()

const stage = ref<1 | 2>(1)
const phase = ref<'idle' | 'running' | 'done' | 'error'>('idle')
const completedRoles = ref(0)
const logs = ref<LogEntry[]>([])
const startedAt = ref(0)

const stageLabel = computed(() =>
  stage.value === 1 ? t('panic.stage1Label') : t('panic.stage2Label'),
)
const progressPct = computed(() => Math.round((completedRoles.value / CURSOR_ROLES.length) * 100))
const remainingMs = computed(() => {
  if (phase.value !== 'running' || completedRoles.value === 0) return null
  const elapsed = Date.now() - startedAt.value
  const perRole = elapsed / completedRoles.value
  return Math.max(0, Math.round(perRole * (CURSOR_ROLES.length - completedRoles.value)))
})

function close() {
  if (phase.value === 'running') return // 実行中は閉じさせない
  emit('update:open', false)
}

function selectStage(s: 1 | 2) {
  stage.value = s
  logs.value = []
  completedRoles.value = 0
  phase.value = 'idle'
}

async function execute() {
  // 旧実装は 17 ロール × 40ms の擬似アニメ (合計 680ms) を IPC 呼出より前に
  // 流していた。実 IPC (reset_to_default / reset_to_initial) は Rust 側で 1
  // トランザクションとして完結するため、擬似ロール進捗を被せるとユーザーに
  // 「snapshot saved」「writing X」などの実際には起きていない処理が起きた
  // ように見せる deceptive UI になっていた (監査 A4-FAKE-001)。
  // 修正後は実 IPC を即時呼出し、完了時にロールグリッドを done 状態に一括
  // 反映する。失敗時は recoveryFailed ログを残し、UI に正直なエラー状態を伝える。
  phase.value = 'running'
  startedAt.value = Date.now()
  completedRoles.value = 0
  logs.value = [
    {
      status: 'running',
      text: t('panic.recoveryStarted'),
      t: '0',
    },
  ]

  try {
    const cmd = stage.value === 1 ? 'reset_to_default' : 'reset_to_initial'
    await invokeTauri<void>(cmd)
    const elapsed = Date.now() - startedAt.value
    // 視覚的に「完了」と認識できるよう、ロールグリッドを一気に done 状態へ。
    // Rust 側は 17 役割すべての registry エントリを書き換えるので意味的に妥当。
    completedRoles.value = CURSOR_ROLES.length
    logs.value[0]!.status = 'ok'
    logs.value.push({
      status: 'ok',
      text: t('panic.recoveryDone', { ms: elapsed }),
      t: String(elapsed),
    })
    phase.value = 'done'
    emit('done', stage.value)
  } catch (err) {
    const elapsed = Date.now() - startedAt.value
    logs.value[0]!.status = 'pending'
    logs.value.push({
      status: 'pending',
      text: t('panic.recoveryFailed', { reason: String(err) }),
      t: String(elapsed),
    })
    phase.value = 'error'
  }
}

watch(
  () => props.open,
  (v) => {
    if (v) {
      phase.value = 'idle'
      logs.value = []
      completedRoles.value = 0
    }
  },
)

function statusOf(i: number): 'done' | 'running' | 'pending' {
  if (i < completedRoles.value) return 'done'
  if (i === completedRoles.value && phase.value === 'running') return 'running'
  return 'pending'
}

function logMark(s: LogEntry['status']): string {
  if (s === 'ok') return '✓'
  if (s === 'running') return '▸'
  return '·'
}
</script>

<template>
  <Transition name="fade">
    <div
      v-if="open"
      class="panic-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="panic-dialog-title"
    >
      <div class="panic-card">
        <header class="panic-head">
          <div class="panic-icon" aria-hidden="true">
            <UiIcon name="Alert" :size="22" />
          </div>
          <div class="panic-title-block">
            <h2 id="panic-dialog-title">{{ t('panic.title') }}</h2>
            <p>{{ t('panic.description') }}</p>
          </div>
          <span class="hotkey">⌃⌥⇧R</span>
        </header>

        <!-- アイドル時: ステージ選択 -->
        <div v-if="phase === 'idle'" class="stage-select">
          <button
            type="button"
            :class="['stage-card', { selected: stage === 1 }]"
            @click="selectStage(1)"
          >
            <div class="stage-meta">
              <span class="step">{{ t('panic.step01') }}</span>
              <span class="badge danger">{{ t('panic.badgeStage1') }}</span>
            </div>
            <h3>{{ t('panic.stage1Title') }}</h3>
            <p>{{ t('panic.stage1Desc') }}</p>
          </button>

          <button
            type="button"
            :class="['stage-card', { selected: stage === 2 }]"
            @click="selectStage(2)"
          >
            <div class="stage-meta">
              <span class="step">{{ t('panic.step02') }}</span>
              <span class="badge">{{ t('panic.badgeStage2') }}</span>
            </div>
            <h3>{{ t('panic.stage2Title') }}</h3>
            <p>{{ t('panic.stage2Desc') }}</p>
          </button>
        </div>

        <!-- 実行中 / 完了 -->
        <div v-else class="trace-block">
          <div class="trace-head">
            <span :class="['phase-dot', phase]" />
            <span class="phase-label">
              {{
                phase === 'running'
                  ? t('panic.restoringLabel', { n: stage, label: stageLabel })
                  : phase === 'done'
                    ? t('panic.completeLabel', { n: stage })
                    : t('panic.errorLabel')
              }}
            </span>
            <span class="role-count">{{
              t('panic.keys', { done: completedRoles, total: CURSOR_ROLES.length })
            }}</span>
          </div>

          <div class="progress-track">
            <div class="progress-fill" :style="{ width: progressPct + '%' }" />
          </div>

          <div class="log-pane">
            <div v-for="(entry, i) in logs" :key="i" :class="['log-line', entry.status]">
              <span class="log-mark">{{ logMark(entry.status) }}</span>
              <span class="log-time">{{ entry.t ? `${entry.t}ms` : '' }}</span>
              <span class="log-text">{{ entry.text }}</span>
            </div>
          </div>

          <div class="role-grid">
            <div
              v-for="(role, i) in CURSOR_ROLES"
              :key="role.id"
              :class="['rg-cell', statusOf(i)]"
              :title="role.jp"
            >
              <CursorIcon :role="role.id" :size="11" />
            </div>
          </div>
        </div>

        <footer class="panic-foot">
          <span class="foot-note">
            <template v-if="phase === 'running' && remainingMs !== null">
              {{ t('panic.estRemaining', { seconds: (remainingMs / 1000).toFixed(1) }) }}
            </template>
            <template v-else-if="phase === 'idle'">
              {{ t('panic.idleHint') }}
            </template>
            <template v-else-if="phase === 'done'">
              {{ t('panic.doneHint') }}
            </template>
            <template v-else>
              {{ t('panic.errorHint') }}
            </template>
          </span>
          <div class="foot-actions">
            <button v-if="phase === 'idle'" class="btn ghost" @click="close">
              {{ t('common.cancel') }}
            </button>
            <button v-if="phase === 'idle'" class="btn primary" @click="execute">
              <UiIcon name="Check" :size="13" />{{ t('panic.runStage', { n: stage }) }}
            </button>
            <button v-else-if="phase === 'running'" class="btn" disabled>
              <span class="spinner" style="width: 13px; height: 13px" />
              {{ t('panic.running') }}
            </button>
            <button v-else class="btn primary" @click="close">
              <UiIcon name="Check" :size="13" />{{ t('common.close') }}
            </button>
          </div>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.panic-overlay {
  @apply fixed inset-0 z-[100] grid place-items-center p-8 backdrop-blur-[10px];
  background:
    radial-gradient(900px 600px at 50% 30%, rgba(124, 242, 212, 0.08), transparent 60%),
    radial-gradient(700px 400px at 80% 100%, rgba(139, 125, 255, 0.06), transparent 60%),
    rgba(0, 0, 0, 0.55);
}

.panic-card {
  @apply w-[720px] max-w-full overflow-hidden rounded-2xl border border-line-hi bg-bg-glass-hi backdrop-blur-[24px];
  box-shadow:
    0 30px 80px -20px rgba(0, 0, 0, 0.7),
    0 0 0 1px rgba(255, 255, 255, 0.04);
}

.panic-head {
  @apply flex items-center gap-4 border-b border-line px-[26px] pb-[18px] pt-[22px];
  background: linear-gradient(180deg, rgba(124, 242, 212, 0.05), transparent);
}
.panic-icon {
  @apply grid size-12 shrink-0 place-items-center rounded-[12px] border border-accent-line text-accent;
  background: rgba(124, 242, 212, 0.1);
  box-shadow: 0 0 20px rgba(124, 242, 212, 0.22);
}
.panic-title-block {
  @apply min-w-0 flex-1;
}
.panic-title-block h2 {
  @apply m-0 font-display text-[18px] font-semibold tracking-[-0.01em];
}
.panic-title-block p {
  @apply mt-1 text-[12.5px] text-fg-dim;
  margin-left: 0;
  margin-right: 0;
  margin-bottom: 0;
}
.hotkey {
  @apply font-mono text-[9.5px] tracking-[0.12em] text-fg-mute;
}

/* ステージ選択 */
.stage-select {
  @apply grid grid-cols-2 gap-3.5 p-[18px];
}
@media (max-width: 600px) {
  .stage-select {
    @apply grid-cols-1;
  }
}
.stage-card {
  @apply flex cursor-pointer flex-col gap-2.5 rounded-[10px] border border-line p-4 text-left text-fg;
  background: var(--color-bg-3);
  transition:
    border-color 0.15s,
    background 0.15s;
}
.stage-card:hover {
  @apply border-line-hi;
}
.stage-card.selected {
  border-color: var(--accent-line);
  background: rgba(124, 242, 212, 0.06);
}
.stage-card h3 {
  @apply m-0 font-display text-[14px] font-semibold;
}
.stage-card p {
  @apply m-0 text-[11.5px] leading-[1.5] text-fg-dim;
}
.stage-meta {
  @apply flex items-center justify-between;
}
.step {
  @apply font-mono text-[10px] tracking-[0.12em] text-fg-mute;
}
.badge {
  @apply rounded border border-line px-2 py-0.5 font-mono text-[10px] text-fg-dim;
}
.badge.danger {
  @apply text-accent;
  border-color: var(--accent-line);
  background: rgba(124, 242, 212, 0.06);
}

/* トレース */
.trace-block {
  @apply px-[26px] py-5;
}
.trace-head {
  @apply mb-3.5 flex items-center gap-2.5;
}
.phase-dot {
  @apply size-2.5 rounded-full bg-accent;
  box-shadow: 0 0 10px var(--accent);
  animation: pulse 1.4s infinite;
}
.phase-dot.done {
  @apply bg-accent;
  box-shadow: 0 0 10px var(--accent);
  animation: none;
}
.phase-dot.error {
  @apply bg-rose;
  animation: none;
}
.phase-label {
  @apply flex-1 font-mono text-[11px] uppercase tracking-[0.08em] text-accent;
}
.role-count {
  @apply font-mono text-[10.5px] text-fg-mute;
}

.progress-track {
  @apply mb-[18px] h-1 overflow-hidden rounded-sm bg-white/5;
}
.progress-fill {
  @apply h-full;
  background: linear-gradient(90deg, var(--accent), #5dd9bd);
  box-shadow: 0 0 8px rgba(124, 242, 212, 0.5);
  transition: width 0.2s ease-out;
}

.log-pane {
  @apply max-h-[200px] overflow-y-auto rounded-[8px] border border-line bg-black/30 p-3 font-mono text-[11.5px] leading-[1.85] text-fg-dim;
}
.log-line {
  @apply flex gap-2.5;
}
.log-line.pending {
  @apply opacity-50;
}
.log-mark {
  @apply w-3 shrink-0;
}
.log-line.ok .log-mark {
  @apply text-accent;
}
.log-line.running .log-mark {
  @apply text-accent;
}
.log-line.pending .log-mark {
  @apply text-fg-faint;
}
.log-time {
  @apply w-14 shrink-0 text-fg-mute;
}
.log-text {
  @apply flex-1 break-all;
}

.role-grid {
  @apply mt-[18px] grid grid-cols-[repeat(17,1fr)] gap-1;
}
@media (max-width: 600px) {
  .role-grid {
    @apply grid-cols-[repeat(9,1fr)];
  }
}
.rg-cell {
  @apply relative grid place-items-center rounded-[5px] border border-line bg-white/[0.02] text-fg-faint;
  aspect-ratio: 1;
}
.rg-cell.done {
  @apply border-accent-line text-accent;
  background: rgba(124, 242, 212, 0.1);
}
.rg-cell.running {
  @apply text-accent;
  background: rgba(124, 242, 212, 0.12);
  border-color: var(--accent-line);
  animation: pulse 1s infinite;
}

/* フッター */
.panic-foot {
  @apply flex items-center justify-between gap-3 border-t border-line px-[26px] py-3.5;
  background: rgba(0, 0, 0, 0.18);
}
:where(html.light) .panic-foot {
  background: rgba(15, 20, 35, 0.025);
}
.foot-note {
  @apply font-mono text-[10.5px] text-fg-mute;
}
.foot-actions {
  @apply flex gap-2;
}

/* トランジション */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
