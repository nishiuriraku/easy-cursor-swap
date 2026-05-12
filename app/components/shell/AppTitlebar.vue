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
 *   - それ以外 → startDragging() で OS にウィンドウ移動を委譲
 *
 * tauriWindow は onMounted で一度解決済みなので、本ハンドラは同期的に呼び出せて
 * mousedown → startDragging の往復にちらつき (Vue 再描画 + WebView 再 layout)
 * が挟まらない。Tauri API 未解決の場合のみ遅延 import にフォールバックする。
 */
function onTitlebarMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement | null
  if (target?.closest('button')) return
  const w = tauriWindow
  if (!w) {
    // mount 完了前の極端なタイミング: 同期処理を諦めて遅延 import 経由で実行する。
    void (async () => {
      const lazy = await loadTauriWindow()
      if (!lazy) return
      if (e.detail === 2) {
        await lazy.toggleMaximize()
        isMaximized.value = await lazy.isMaximized()
      } else {
        await lazy.startDragging()
      }
    })()
    return
  }
  if (e.detail === 2) {
    void w.toggleMaximize().then(async () => {
      isMaximized.value = await w.isMaximized()
    })
  } else {
    // startDragging は OS の WM_NCLBUTTONDOWN 経由でドラッグループに入るので
    // pointermove イベント等を自前で監視する必要はない。
    void w.startDragging()
  }
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
  @apply relative z-[5] grid h-9 grid-cols-[1fr_auto] items-center border-b border-line bg-bg-titlebar pl-3.5 backdrop-blur-[20px];
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
