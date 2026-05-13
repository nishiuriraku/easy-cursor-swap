/**
 * UI のライト/ダーク表示モード切替。
 *
 * OS のダークモード設定や Rust 側の config とは独立した、
 * アプリ自身の見た目を Light/Dark/Auto から選ぶための composable。
 *
 * - Auto: prefers-color-scheme に追従
 * - Light: html.light を付与
 * - Dark : html.light を外す (既定)
 *
 * 永続化は localStorage (UI 設定なので config.json に汚染しない)。
 */
import { ref, watch, onMounted, onUnmounted } from 'vue'

export type UiThemeMode = 'auto' | 'light' | 'dark'

const STORAGE_KEY = 'easycursorswap.ui-theme'
const mode = ref<UiThemeMode>('dark')
let mql: MediaQueryList | null = null
let initialized = false

function applyClass() {
  if (typeof document === 'undefined') return
  // prefers-color-scheme: dark に matches=true なら OS は暗色を好んでいる
  const wantLight = mode.value === 'light' || (mode.value === 'auto' && mql ? !mql.matches : false)
  document.documentElement.classList.toggle('light', wantLight)
}

function init() {
  if (initialized || typeof window === 'undefined') return
  initialized = true
  try {
    const saved = localStorage.getItem(STORAGE_KEY) as UiThemeMode | null
    if (saved === 'light' || saved === 'dark' || saved === 'auto') {
      mode.value = saved
    }
  } catch {
    // localStorage 不可: 既定値のまま
  }
  mql = window.matchMedia('(prefers-color-scheme: dark)')
  applyClass()
  mql.addEventListener('change', applyClass)
}

export function useUiTheme() {
  onMounted(init)
  onUnmounted(() => {
    // listener は app 全体で 1 つだけにしたいので個別 unmount では外さない。
  })

  watch(mode, (m) => {
    try {
      localStorage.setItem(STORAGE_KEY, m)
    } catch {
      /* noop */
    }
    applyClass()
  })

  function cycle() {
    mode.value = mode.value === 'dark' ? 'light' : mode.value === 'light' ? 'auto' : 'dark'
  }

  return { mode, cycle }
}
