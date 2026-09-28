/**
 * 設定画面のカーソルサイズ設定 (P08a Step 3 / S2)。
 *
 * マウスポインターのサイズ (1〜15 スライダー)。Windows 設定アプリ
 * 「マウスポインターとタッチ」と同じ HKCU\Control Panel\Cursors\CursorBaseSize
 * を更新するが、Accessibility\CursorSize は書かない invariant のため、Windows
 * 設定 UI 側スライダー位置とは意図的に同期しない
 * (specs/2026-05-23-cursor-size-redesign-v2)。値は Rust 側が single source of
 * truth で、本 UI は変更を即時 IPC で反映する (dirty/save フローには乗せない —
 * テーマ適用とは独立した OS 全体設定のため)。
 *
 * スライダー位置 ↔ DWORD の換算式は `registry::slider_position_to_base_size` と
 * 1:1 で同期: `dword = 32 + 16 * (slider - 1)` (slider=1 → 32, slider=15 → 256)。
 */
import { invokeTauri } from './useTauri'
import { useAccessibility } from './useAccessibility'
import { useExternalUrl } from './useExternalUrl'

export const CURSOR_SIZE_MIN_DWORD = 32
export const CURSOR_SIZE_STEP_DWORD = 16
export const CURSOR_SIZE_MIN_SLIDER = 1
export const CURSOR_SIZE_MAX_SLIDER = 15

export function useCursorSizeSettings() {
  const cursorSizeSlider = ref<number>(CURSOR_SIZE_MIN_SLIDER)
  const cursorSizeBusy = ref(false)
  const cursorSizeError = ref<string | null>(null)
  // Windows 側の生の Accessibility\CursorSize (slider 1-15)。`cursorSizeSlider` は
  // CursorBaseSize から算出された slider 位置だが、こちらは Accessibility key の直値。
  // eoa pipeline 状態 (== Windows Settings でサイズが拡大されている状態) の検出に使う。
  const cursorSizeSliderRaw = ref<number>(1)
  // Windows 側の生の Accessibility\CursorType (0=白, 1=黒, 2=反転, 3=白拡大時, 6=カスタム ...)。
  // gate 判定には使わないが UI メッセージのバリエーション切替に使う。
  const cursorTypeRaw = ref<number>(0)
  // eoa pipeline 起動中の判定: `cursorSizeSliderRaw != 1`。
  // 本アプリの slider はこのとき disabled にする (spec: case E + CursorSize gate)。
  const cursorAccessibilityActive = computed(() => cursorSizeSliderRaw.value !== 1)

  function dwordToSlider(dword: number): number {
    const clamped = Math.max(
      CURSOR_SIZE_MIN_DWORD,
      Math.min(
        CURSOR_SIZE_MIN_DWORD + CURSOR_SIZE_STEP_DWORD * (CURSOR_SIZE_MAX_SLIDER - 1),
        dword,
      ),
    )
    // 四捨五入で最近接スライダー位置に snap
    const offset = clamped - CURSOR_SIZE_MIN_DWORD
    const zeroBased = Math.round(offset / CURSOR_SIZE_STEP_DWORD)
    return CURSOR_SIZE_MIN_SLIDER + zeroBased
  }

  function sliderToDword(slider: number): number {
    const clamped = Math.max(CURSOR_SIZE_MIN_SLIDER, Math.min(CURSOR_SIZE_MAX_SLIDER, slider))
    return CURSOR_SIZE_MIN_DWORD + CURSOR_SIZE_STEP_DWORD * (clamped - 1)
  }

  /** スライダー確定時 (`@change`): DWORD に変換して即時 IPC 反映する。 */
  async function onCursorSizeCommit(next: number) {
    cursorSizeBusy.value = true
    cursorSizeError.value = null
    try {
      const written = await invokeTauri<number>('set_cursor_base_size', {
        size: sliderToDword(next),
      })
      // Rust 側でクランプされた値を slider に反映 (snap)
      cursorSizeSlider.value = dwordToSlider(written ?? sliderToDword(next))
    } catch (err) {
      cursorSizeError.value = err instanceof Error ? err.message : String(err)
      // 失敗時は OS 側を再取得してロールバック表示
      await refreshCursorSizeFromOs()
    } finally {
      cursorSizeBusy.value = false
    }
  }

  async function refreshCursorSizeFromOs() {
    try {
      const a11y = await useAccessibility().getAccessibilityConflicts()
      cursorSizeSlider.value = dwordToSlider(a11y?.cursor_base_size ?? CURSOR_SIZE_MIN_DWORD)
      cursorSizeSliderRaw.value = a11y?.cursor_size_slider ?? 1
      cursorTypeRaw.value = a11y?.cursor_type ?? 0
    } catch {
      // 取得失敗時は既定 (slider=1, type=0) のまま
      cursorSizeSlider.value = CURSOR_SIZE_MIN_SLIDER
      cursorSizeSliderRaw.value = 1
      cursorTypeRaw.value = 0
    }
  }

  /**
   * GeneralSection の「OS から再取得」リンクから明示的に呼ばれる手動 refresh。
   */
  function onRefreshCursorSizeFromOs() {
    void refreshCursorSizeFromOs()
  }

  /**
   * GeneralSection の「Windows 設定を開く」ボタンから呼ばれる。eoa pipeline 解除のために
   * Windows のアクセシビリティ設定 (マウスポインターとタッチ) へ deep-link する。
   */
  async function onOpenWindowsCursorSettings() {
    try {
      // **正しい URI** は `easeofaccess-mousepointer` (= マウスポインターとタッチ)。
      // `easeofaccess-cursor` は **テキストカーソル (挿入点)** ページのため別ページに飛ぶ。
      // Microsoft 公式: https://learn.microsoft.com/windows/apps/develop/launch/launch-settings
      await useExternalUrl().openExternalUrl('ms-settings:easeofaccess-mousepointer')
    } catch (err) {
      console.warn('[Settings] open ms-settings:easeofaccess-mousepointer failed:', err)
    }
  }

  /**
   * Windows 側 (アクセシビリティ「マウスポインターとタッチ」スライダー / コントロール
   * パネル / 他アプリ) でカーソルサイズが変更されたあと、本アプリへフォーカスが戻った
   * タイミングで OS 状態を再取得し、`cursorAccessibilityActive` を更新する。
   *
   * 2026-05-22 の case B+ 刷新で一度撤去したが、続く 2026-05-23 の case E 採用で
   * 撤去の元になっていた「徐々に大きくなる」現象は eoa pipeline 起因と判明したため、
   * 再導入しても再発しない:
   *  - eoa active 時は UI が disabled → ユーザーは slider 操作できない → ループ起点なし
   *  - eoa inactive 時は CursorSize==1 → 我々の write が標準 pipeline で完結 →
   *    Windows が auto-correct する経路がない
   *
   * Windows Settings 経由で CursorSize=1 に戻ったあと、focus 戻りで自動的に slider が
   * enabled に切替わる UX を提供するためにも必要。
   */
  function onWindowFocus() {
    void refreshCursorSizeFromOs()
  }
  function onVisibilityChange() {
    if (typeof document === 'undefined') return
    if (document.visibilityState === 'visible') void refreshCursorSizeFromOs()
  }

  /**
   * eoa pipeline 解除 (= Windows Settings で size=1) を検出するため、focus 戻り /
   * visibilitychange を購読して自動再同期する。case E + CursorSize gate により、
   * 過去発生した「徐々に大きくなる」ループは再発しない (詳細は onWindowFocus の docstring)。
   */
  function startAutoResync() {
    if (typeof window !== 'undefined') {
      window.addEventListener('focus', onWindowFocus)
    }
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', onVisibilityChange)
    }
  }

  function stopAutoResync() {
    if (typeof window !== 'undefined') {
      window.removeEventListener('focus', onWindowFocus)
    }
    if (typeof document !== 'undefined') {
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }
  }

  onMounted(() => {
    void refreshCursorSizeFromOs()
    startAutoResync()
  })
  onUnmounted(stopAutoResync)

  return {
    cursorSizeSlider,
    cursorSizeBusy,
    cursorSizeError,
    cursorSizeSliderRaw,
    cursorTypeRaw,
    cursorAccessibilityActive,
    CURSOR_SIZE_MIN_SLIDER,
    CURSOR_SIZE_MAX_SLIDER,
    sliderToDword,
    dwordToSlider,
    onCursorSizeCommit,
    onRefreshCursorSizeFromOs,
    refreshCursorSizeFromOs,
    onOpenWindowsCursorSettings,
    startAutoResync,
    stopAutoResync,
  }
}
