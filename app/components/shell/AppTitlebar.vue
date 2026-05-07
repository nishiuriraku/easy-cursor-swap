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

withDefaults(defineProps<{
  title?: string
  version?: string
}>(), {
  title: 'EasyCursorSwap',
  version: 'v1.0.0',
})

const { mode, cycle } = useUiTheme()
const themeIcon = computed(() => (mode.value === 'light' ? 'Sun' : mode.value === 'auto' ? 'Globe' : 'Moon'))
const themeLabel = computed(() => (mode.value === 'light' ? 'Light' : mode.value === 'auto' ? 'Auto' : 'Dark'))

// 最大化アイコンを Win11 既定動作に揃えるためにウィンドウ状態を購読する。
const isMaximized = ref(false)
let unlistenResize: (() => void) | null = null

async function refreshMaximizeState() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    isMaximized.value = await getCurrentWindow().isMaximized()
  } catch {
    // Web 開発時は Tauri API が無いので無視
  }
}

onMounted(async () => {
  await refreshMaximizeState()
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    unlistenResize = await getCurrentWindow().onResized(() => {
      void refreshMaximizeState()
    })
  } catch {
    // Tauri API 未接続環境では購読をスキップ
  }
})

onBeforeUnmount(() => {
  if (unlistenResize) {
    unlistenResize()
    unlistenResize = null
  }
})

async function call(cmd: 'minimize' | 'toggleMaximize' | 'close') {
  // Tauri v2 のウィンドウ API を遅延 import して SSR/Web 開発時のクラッシュを回避。
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const w = getCurrentWindow()
    if (cmd === 'minimize') await w.minimize()
    else if (cmd === 'toggleMaximize') {
      await w.toggleMaximize()
      // 状態の永続化を待たずに即座に反映する。`onResized` も走るので二重更新になっても問題ない。
      isMaximized.value = await w.isMaximized()
    }
    else await w.close()
  } catch (err) {
    console.warn('[Titlebar] Tauri API unavailable:', err)
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
 */
async function onTitlebarMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  const target = e.target as HTMLElement | null
  if (target?.closest('button')) return
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const w = getCurrentWindow()
    if (e.detail === 2) {
      await w.toggleMaximize()
      isMaximized.value = await w.isMaximized()
    } else {
      // startDragging は OS の WM_NCLBUTTONDOWN 経由でドラッグループに入るので
      // pointermove イベント等を自前で監視する必要はない。
      await w.startDragging()
    }
  } catch (err) {
    console.warn('[Titlebar] drag/maximize failed:', err)
  }
}
</script>

<template>
  <div class="titlebar" @mousedown="onTitlebarMouseDown">
    <div class="tb-title">
      <span class="tb-mark"><UiIcon name="Logo" :size="12" /></span>
      <span>{{ title }}</span>
      <span style="color: var(--fg-faint)">—</span>
      <span class="tb-meta">{{ version }} · Win 11</span>
    </div>
    <div class="tb-controls">
      <button
        type="button"
        class="tb-btn theme"
        :aria-label="`UI テーマ: ${themeLabel} (クリックで切替)`"
        :title="`Theme: ${themeLabel}`"
        @click="cycle"
      >
        <UiIcon :name="themeIcon" :size="12" />
      </button>
      <button type="button" class="tb-btn" :aria-label="t('titlebar.minimize')" @click="call('minimize')">
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
      <button type="button" class="tb-btn close" :aria-label="t('common.close')" @click="call('close')">
        <UiIcon name="X" :size="12" />
      </button>
    </div>
  </div>
</template>
