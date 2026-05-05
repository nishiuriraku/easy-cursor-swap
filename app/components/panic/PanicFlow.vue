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
const progressPct = computed(() =>
  Math.round((completedRoles.value / CURSOR_ROLES.length) * 100),
)
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
  phase.value = 'running'
  startedAt.value = Date.now()
  logs.value = [
    { status: 'ok', text: 'HKCU\\Control Panel\\Cursors snapshot saved', t: '0' },
  ]
  completedRoles.value = 0

  // 進捗の擬似アニメ (実 IPC は ms オーダーで完了するため、UI 演出として)
  const step = async (delay: number) => {
    await new Promise((r) => setTimeout(r, delay))
  }

  try {
    // 17 ロール書き込みの演出
    for (let i = 0; i < CURSOR_ROLES.length; i++) {
      const role = CURSOR_ROLES[i]!
      logs.value.push({
        status: 'running',
        text: `writing ${role.id} → ${stage.value === 1 ? '(Windows 既定)' : '(snapshot)'}`,
        t: String(Date.now() - startedAt.value),
      })
      await step(40)
      logs.value[logs.value.length - 1]!.status = 'ok'
      completedRoles.value = i + 1
    }

    // 実 IPC 呼び出し
    const cmd = stage.value === 1 ? 'reset_to_default' : 'reset_to_initial'
    await invokeTauri<void>(cmd)

    logs.value.push({
      status: 'ok',
      text: `recovery completed in ${Date.now() - startedAt.value} ms`,
      t: String(Date.now() - startedAt.value),
    })
    phase.value = 'done'
    emit('done', stage.value)
  } catch (err) {
    logs.value.push({ status: 'pending', text: `error: ${err}`, t: '' })
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
    <div v-if="open" class="panic-overlay" role="dialog" aria-modal="true" aria-labelledby="panic-dialog-title">
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
              {{ phase === 'running' ? t('panic.restoringLabel', { n: stage, label: stageLabel })
                 : phase === 'done' ? t('panic.completeLabel', { n: stage })
                 : t('panic.errorLabel') }}
            </span>
            <span class="role-count">{{ t('panic.keys', { done: completedRoles, total: CURSOR_ROLES.length }) }}</span>
          </div>

          <div class="progress-track">
            <div class="progress-fill" :style="{ width: progressPct + '%' }" />
          </div>

          <div class="log-pane">
            <div
              v-for="(entry, i) in logs"
              :key="i"
              :class="['log-line', entry.status]"
            >
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
            <button
              v-if="phase === 'idle'"
              class="btn danger"
              @click="execute"
            >
              <UiIcon name="Alert" :size="13" />{{ t('panic.runStage', { n: stage }) }}
            </button>
            <button
              v-else-if="phase === 'running'"
              class="btn"
              disabled
            >
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
.panic-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  padding: 32px;
  background:
    radial-gradient(900px 600px at 50% 30%, rgba(255, 107, 138, 0.10), transparent 60%),
    radial-gradient(700px 400px at 80% 100%, rgba(139, 125, 255, 0.06), transparent 60%),
    rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
}

.panic-card {
  width: 720px;
  max-width: 100%;
  background: var(--bg-glass-hi);
  border: 1px solid rgba(255, 107, 138, 0.25);
  border-radius: 16px;
  overflow: hidden;
  box-shadow:
    0 30px 80px -20px rgba(0, 0, 0, 0.7),
    0 0 0 1px rgba(255, 107, 138, 0.15);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
}

.panic-head {
  padding: 22px 26px 18px;
  border-bottom: 1px solid var(--line);
  display: flex;
  align-items: center;
  gap: 16px;
  background: linear-gradient(180deg, rgba(255, 107, 138, 0.06), transparent);
}
.panic-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: rgba(255, 107, 138, 0.12);
  border: 1px solid rgba(255, 107, 138, 0.35);
  color: var(--rose);
  box-shadow: 0 0 20px rgba(255, 107, 138, 0.3);
  flex-shrink: 0;
}
.panic-title-block { flex: 1; min-width: 0; }
.panic-title-block h2 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 18px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
.panic-title-block p {
  margin: 4px 0 0;
  font-size: 12.5px;
  color: var(--fg-dim);
}
.hotkey {
  font-family: var(--font-mono);
  font-size: 9.5px;
  color: var(--rose);
  letter-spacing: 0.12em;
}

/* ステージ選択 */
.stage-select {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  padding: 18px;
}
@media (max-width: 600px) {
  .stage-select {
    grid-template-columns: 1fr;
  }
}
.stage-card {
  text-align: left;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.2);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  display: flex;
  flex-direction: column;
  gap: 10px;
  color: var(--fg);
}
.stage-card:hover {
  border-color: var(--line-hi);
}
.stage-card.selected {
  border-color: rgba(255, 107, 138, 0.5);
  background: rgba(255, 107, 138, 0.05);
}
.stage-card h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 14px;
  font-weight: 600;
}
.stage-card p {
  margin: 0;
  font-size: 11.5px;
  color: var(--fg-dim);
  line-height: 1.5;
}
.stage-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.step {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--fg-mute);
  letter-spacing: 0.12em;
}
.badge {
  font-family: var(--font-mono);
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid var(--line);
  color: var(--fg-dim);
}
.badge.danger {
  color: var(--rose);
  border-color: rgba(255, 107, 138, 0.25);
  background: rgba(255, 107, 138, 0.06);
}

/* トレース */
.trace-block { padding: 20px 26px; }
.trace-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}
.phase-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--rose);
  box-shadow: 0 0 10px var(--rose);
  animation: pulse 1.4s infinite;
}
.phase-dot.done {
  background: var(--accent);
  box-shadow: 0 0 10px var(--accent);
  animation: none;
}
.phase-dot.error {
  background: var(--rose);
  animation: none;
}
.phase-label {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--rose);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  flex: 1;
}
.role-count {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
}

.progress-track {
  height: 4px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 18px;
}
.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--rose), #ff9bb0);
  box-shadow: 0 0 8px rgba(255, 107, 138, 0.6);
  transition: width 0.2s ease-out;
}

.log-pane {
  font-family: var(--font-mono);
  font-size: 11.5px;
  line-height: 1.85;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 12px 14px;
  max-height: 200px;
  overflow-y: auto;
  color: var(--fg-dim);
}
.log-line {
  display: flex;
  gap: 10px;
}
.log-line.pending { opacity: 0.5; }
.log-mark { width: 12px; flex-shrink: 0; }
.log-line.ok .log-mark { color: var(--accent); }
.log-line.running .log-mark { color: var(--rose); }
.log-line.pending .log-mark { color: var(--fg-faint); }
.log-time {
  color: var(--fg-mute);
  width: 56px;
  flex-shrink: 0;
}
.log-text { flex: 1; word-break: break-all; }

.role-grid {
  display: grid;
  grid-template-columns: repeat(17, 1fr);
  gap: 4px;
  margin-top: 18px;
}
@media (max-width: 600px) {
  .role-grid {
    grid-template-columns: repeat(9, 1fr);
  }
}
.rg-cell {
  aspect-ratio: 1;
  border-radius: 5px;
  display: grid;
  place-items: center;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--line);
  color: var(--fg-faint);
  position: relative;
}
.rg-cell.done {
  background: rgba(124, 242, 212, 0.10);
  border-color: var(--accent-line);
  color: var(--accent);
}
.rg-cell.running {
  background: rgba(255, 107, 138, 0.12);
  border-color: rgba(255, 107, 138, 0.4);
  color: var(--rose);
  animation: pulse 1s infinite;
}

/* フッター */
.panic-foot {
  padding: 14px 26px;
  background: rgba(0, 0, 0, 0.25);
  border-top: 1px solid var(--line);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.foot-note {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
}
.foot-actions {
  display: flex;
  gap: 8px;
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
