<script setup lang="ts">
/**
 * Win11 風タイトルバー。
 * Tauri ウィンドウコントロール (最小化/最大化/閉じる) を将来的に IPC で接続する。
 * 現状はクリックハンドラーのみ用意し、IPC 未接続時はコンソール警告のみ。
 */
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useUiTheme } from '~/composables/useUiTheme'

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

// Tauri v2 では `data-tauri-drag-region` 属性 (mousedown 監視ベース) を使うのが
// 公式の手段。CSS `app-region` は WebView2 + decorations:false で動作しないことが
// あるので、自前のハンドラに依存しない属性ベースで二重に保険をかける。
// data-tauri-drag-region をつけた要素が直接の mousedown ターゲットなら startDragging()
// + ダブルクリック最大化が自動発火する。子要素 (button) で mousedown した場合は
// target がボタンになるためドラッグは発火せず click が正常通る。
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <div class="tb-title" data-tauri-drag-region>
      <span class="tb-mark"><UiIcon name="Logo" :size="12" /></span>
      <span data-tauri-drag-region>{{ title }}</span>
      <span data-tauri-drag-region style="color: var(--fg-faint)">—</span>
      <span class="tb-meta" data-tauri-drag-region>{{ version }} · Win 11</span>
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
      <button type="button" class="tb-btn" aria-label="最小化" @click="call('minimize')">
        <UiIcon name="Min" :size="12" />
      </button>
      <button
        type="button"
        class="tb-btn"
        :aria-label="isMaximized ? '元に戻す' : '最大化'"
        @click="call('toggleMaximize')"
      >
        <UiIcon :name="isMaximized ? 'Restore' : 'Max'" :size="12" />
      </button>
      <button type="button" class="tb-btn close" aria-label="閉じる" @click="call('close')">
        <UiIcon name="X" :size="12" />
      </button>
    </div>
  </div>
</template>
