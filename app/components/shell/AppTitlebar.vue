<script setup lang="ts">
/**
 * Win11 風タイトルバー。
 * Tauri ウィンドウコントロール (最小化/最大化/閉じる) を将来的に IPC で接続する。
 * 現状はクリックハンドラーのみ用意し、IPC 未接続時はコンソール警告のみ。
 */
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useUiTheme } from '~/composables/useUiTheme'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

withDefaults(
  defineProps<{
    title?: string
    version?: string
  }>(),
  {
    title: 'EasyCursorSwap',
    version: 'v1.0.0',
  },
)

const { mode, cycle } = useUiTheme()
const themeIcon = computed(() =>
  mode.value === 'light' ? 'Sun' : mode.value === 'auto' ? 'Globe' : 'Moon',
)
const themeLabel = computed(() =>
  mode.value === 'light' ? 'Light' : mode.value === 'auto' ? 'Auto' : 'Dark',
)

// 最大化アイコンを Win11 既定動作に揃えるためにウィンドウ状態を購読する。
const isMaximized = ref(false)
let unlistenResize: (() => void) | null = null

/**
 * Tauri window API は SSR/Web 開発時に存在しないため動的 import するが、
 * mousedown のたびに import すると WebView2 のリクエスト解決が走って
 * クリックの数 frame ぶん UI がちらつく原因になっていた。
 * mount 時に一度だけ解決して以降は同期参照する。
 */
type TauriWindow = {
  minimize(): Promise<void>
  toggleMaximize(): Promise<void>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
  startDragging(): Promise<void>
  onResized(cb: () => void): Promise<() => void>
}
let tauriWindow: TauriWindow | null = null

async function loadTauriWindow(): Promise<TauriWindow | null> {
  if (tauriWindow) return tauriWindow
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    tauriWindow = getCurrentWindow() as TauriWindow
    return tauriWindow
  } catch {
    // Web 開発時は Tauri API が無いので無視
    return null
  }
}

async function refreshMaximizeState() {
  const w = await loadTauriWindow()
  if (!w) return
  isMaximized.value = await w.isMaximized()
}

onMounted(async () => {
  const w = await loadTauriWindow()
  if (!w) return
  isMaximized.value = await w.isMaximized()
  unlistenResize = await w.onResized(() => {
    void refreshMaximizeState()
  })
})

onBeforeUnmount(() => {
  if (unlistenResize) {
    unlistenResize()
    unlistenResize = null
  }
})

async function call(cmd: 'minimize' | 'toggleMaximize' | 'close') {
  const w = tauriWindow ?? (await loadTauriWindow())
  if (!w) {
    console.warn('[Titlebar] Tauri API unavailable')
    return
  }
  try {
    if (cmd === 'minimize') await w.minimize()
    else if (cmd === 'toggleMaximize') {
      await w.toggleMaximize()
      // 状態の永続化を待たずに即座に反映する。`onResized` も走るので二重更新になっても問題ない。
      isMaximized.value = await w.isMaximized()
    } else await w.close()
  } catch (err) {
    console.warn('[Titlebar] window command failed:', err)
  }
}

/**
 * タイトルバーのドラッグ移動 / ダブルクリック最大化をマニュアル実装する。
 *
 * Tauri v2 の `data-tauri-drag-region` 属性は内部で `closest()` を使って
 * ドラッグ対象を判定するため、属性持ち要素の子孫 (button) の mousedown でも
 * ドラッグが発火してしまい、click イベントが奪われる不具合があった。さらに
 * `decorations: false` + WebView2 release ビルドの組み合わせで startDragging が
 * 不安定なケースが報告されている。
 *
 * このハンドラを `.titlebar` に直接付けて以下を行う:
 *   - target がボタン (またはその子) なら何もしない → button の click 通常動作
 *   - 左クリック以外は無視
 *   - e.detail === 2 (ダブルクリックの 2 発目) → toggleMaximize()
 *   - それ以外 → ドラッグ閾値 (4px) を超えてから startDragging()
 *
 * **閾値方式の理由**: mousedown 直後に startDragging を呼ぶと、純粋なクリック
 * (移動量 0) でも OS の WM_NCLBUTTONDOWN が発火して即時抜け、WebView2 が
 * フォーカスを再取得 → Vue/CSS が re-layout してちらつき (= 「再レンダリング
 * されたような挙動」) になる。pointermove を待って 4px 以上動いたときだけ
 * startDragging することで、クリックでは何も起きないようにする。
 */
const DRAG_THRESHOLD_PX = 4

function onTitlebarMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement | null
  if (target?.closest('button')) return

  // ダブルクリックは即時 toggleMaximize (Win11 と同じ挙動)
  if (e.detail === 2) {
    void (async () => {
      const w = tauriWindow ?? (await loadTauriWindow())
      if (!w) return
      await w.toggleMaximize()
      isMaximized.value = await w.isMaximized()
    })()
    return
  }

  // 単発 mousedown は閾値ベース: 4px 以上動いてから startDragging を発火
  const startX = e.clientX
  const startY = e.clientY
  let started = false

  const onMove = (me: MouseEvent) => {
    if (started) return
    const dx = Math.abs(me.clientX - startX)
    const dy = Math.abs(me.clientY - startY)
    if (dx > DRAG_THRESHOLD_PX || dy > DRAG_THRESHOLD_PX) {
      started = true
      cleanup()
      void (async () => {
        const w = tauriWindow ?? (await loadTauriWindow())
        if (!w) return
        // startDragging は OS の WM_NCLBUTTONDOWN 経由でドラッグループに入るので
        // pointermove を自前で追う必要はここから先にはない。
        await w.startDragging()
      })()
    }
  }
  const onUp = () => cleanup()
  const cleanup = () => {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}
</script>

<template>
  <div class="titlebar" @mousedown="onTitlebarMouseDown">
    <div class="tb-title">
      <span class="tb-mark"><UiIcon name="Logo" :size="12" /></span>
      <span>{{ title }}</span>
      <span class="tb-dash">—</span>
      <span class="tb-meta">{{ version }} · Win 11</span>
    </div>
    <div class="tb-controls">
      <button
        type="button"
        class="tb-btn"
        :aria-label="`UI テーマ: ${themeLabel} (クリックで切替)`"
        :title="`Theme: ${themeLabel}`"
        @click="cycle"
      >
        <UiIcon :name="themeIcon" :size="12" />
      </button>
      <button
        type="button"
        class="tb-btn"
        :aria-label="t('titlebar.minimize')"
        @click="call('minimize')"
      >
        <UiIcon name="Min" :size="12" />
      </button>
      <button
        type="button"
        class="tb-btn"
        :aria-label="isMaximized ? t('titlebar.restore') : t('titlebar.maximize')"
        @click="call('toggleMaximize')"
      >
        <UiIcon :name="isMaximized ? 'Restore' : 'Max'" :size="12" />
      </button>
      <button
        type="button"
        class="tb-btn tb-btn-close"
        :aria-label="t('common.close')"
        @click="call('close')"
      >
        <UiIcon name="X" :size="12" />
      </button>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.titlebar {
  @apply relative z-[5] grid h-9 select-none grid-cols-[1fr_auto] items-center border-b border-line bg-bg-titlebar pl-3.5 backdrop-blur-[20px];
  /* タイトルバーをドラッグ操作するとテキストが選択されてカーソルが I-beam になり
   * 移動 UX が崩れるため、全体でテキスト選択を無効化する。
   * (Safari 互換のため -webkit-user-select も明示) */
  -webkit-user-select: none;
}
.tb-title {
  @apply flex items-center gap-2.5 text-[12px] tracking-[0.02em] text-fg-dim;
}
.tb-mark {
  @apply grid size-3.5 place-items-center text-accent;
}
.tb-dash {
  @apply text-fg-faint;
}
.tb-meta {
  @apply font-mono text-[10.5px] text-fg-mute;
}
.tb-controls {
  @apply relative z-[6] flex;
}

/* Win11 風タイトルバーボタン。
 * hover の overlay 色がダーク/ライトで反転するため `:where(html.light)` で
 * scoped style 内に上書きルールを併置する。`tb-btn-close:hover` だけは赤背景固定。 */
.tb-btn {
  @apply grid h-9 w-[46px] place-items-center border-0 bg-transparent text-fg-dim;
  cursor: pointer;
  pointer-events: auto;
}
.tb-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--fg);
}
:where(html.light) .tb-btn:hover {
  background: rgba(15, 20, 35, 0.06);
}
.tb-btn-close:hover {
  background: #c42b1c;
  color: #fff;
}
:where(html.light) .tb-btn-close:hover {
  background: #c42b1c;
  color: #fff;
}
</style>
