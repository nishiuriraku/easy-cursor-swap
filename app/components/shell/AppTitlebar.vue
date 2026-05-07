<script setup lang="ts">
/**
 * Win11 風タイトルバー。
 * Tauri ウィンドウコントロール (最小化/最大化/閉じる) を将来的に IPC で接続する。
 * 現状はクリックハンドラーのみ用意し、IPC 未接続時はコンソール警告のみ。
 */
import { computed } from 'vue'
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

async function call(cmd: 'minimize' | 'toggleMaximize' | 'close') {
  // Tauri v2 のウィンドウ API を遅延 import して SSR/Web 開発時のクラッシュを回避。
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const w = getCurrentWindow()
    if (cmd === 'minimize') await w.minimize()
    else if (cmd === 'toggleMaximize') await w.toggleMaximize()
    else await w.close()
  } catch (err) {
    console.warn('[Titlebar] Tauri API unavailable:', err)
  }
}
</script>

<template>
  <div class="titlebar">
    <div class="tb-title">
      <span class="tb-mark"><UiIcon name="Logo" :size="12" /></span>
      <span>{{ title }}</span>
      <span style="color: var(--fg-faint)">—</span>
      <span class="tb-meta">{{ version }} · Win 11</span>
    </div>
    <div class="tb-controls">
      <button
        class="tb-btn theme"
        :aria-label="`UI テーマ: ${themeLabel} (クリックで切替)`"
        :title="`Theme: ${themeLabel}`"
        @click="cycle"
      >
        <UiIcon :name="themeIcon" :size="12" />
      </button>
      <button class="tb-btn" aria-label="最小化" @click="call('minimize')">
        <UiIcon name="Min" :size="12" />
      </button>
      <button class="tb-btn" aria-label="最大化" @click="call('toggleMaximize')">
        <UiIcon name="Max" :size="12" />
      </button>
      <button class="tb-btn close" aria-label="閉じる" @click="call('close')">
        <UiIcon name="X" :size="12" />
      </button>
    </div>
  </div>
</template>
