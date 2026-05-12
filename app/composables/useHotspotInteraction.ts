/**
 * ホットスポット (ratio 0..1) の編集インタラクション composable。
 *
 * `<CursorPreview editable>` の内部で使われ、pointer ドラッグと keyboard nudge を提供する。
 * 中心の座標計算 (`pointerToHotspotRatio` / `applyKeyboardNudge`) は純粋関数として export し、
 * vitest で単体検証する。
 */
import { ref, type Ref } from 'vue'
import type { Hotspot } from './useCreatorAssets'

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value))
}

/**
 * pointer の clientX/clientY を、コンテナ内に displayPct% で中央配置された画像領域の
 * ratio (0..1) に変換する。画像領域の外側はクランプ。
 *
 * - コンテナ幅 200px / displayPct=90 → 内側 180px (左右各 10px の余白) を ratio 0..1 にマップ。
 */
export function pointerToHotspotRatio(
  e: { clientX: number; clientY: number },
  rect: DOMRect,
  displayPct: number,
): Hotspot {
  const margin = (100 - displayPct) / 200 // 90 → 0.05
  const innerLeft = rect.left + rect.width * margin
  const innerTop = rect.top + rect.height * margin
  const innerWidth = rect.width * (displayPct / 100)
  const innerHeight = rect.height * (displayPct / 100)
  if (innerWidth <= 0 || innerHeight <= 0) {
    return { x: 0, y: 0 }
  }
  const x = clamp01((e.clientX - innerLeft) / innerWidth)
  const y = clamp01((e.clientY - innerTop) / innerHeight)
  return { x, y }
}

/**
 * keyboard nudge: 矢印 / Home / End / PgUp / PgDn に対応。Shift で 10px ステップ。
 *
 * 1 ステップ = 1px / refPx の ratio。refPx <= 0 のときは step=0 で位置不変。
 * 対応外のキーは null を返す (呼び出し側で preventDefault しない判定に使う)。
 */
export function applyKeyboardNudge(
  current: Hotspot,
  key: string,
  shift: boolean,
  refPx: number,
): Hotspot | null {
  const px = shift ? 10 : 1
  const step = refPx > 0 ? px / refPx : 0
  let { x, y } = current

  switch (key) {
    case 'ArrowLeft':
      x -= step
      break
    case 'ArrowRight':
      x += step
      break
    case 'ArrowUp':
      y -= step
      break
    case 'ArrowDown':
      y += step
      break
    case 'Home':
      x = 0
      break
    case 'End':
      x = 1
      break
    case 'PageUp':
      y = 0
      break
    case 'PageDown':
      y = 1
      break
    default:
      return null
  }
  return { x: clamp01(x), y: clamp01(y) }
}

/**
 * ホットスポット dot の left/top スタイル (画像中央 ± (h - 0.5) × displayPct%)。
 * `<CursorPreview>` の dot 配置と drawer の hotspot 表示で共通の式。
 */
export function hotspotDotStyle(
  hotspot: Hotspot,
  displayPct: number,
): { left: string; top: string } {
  return {
    left: `calc(50% + ${(hotspot.x - 0.5) * displayPct}%)`,
    top: `calc(50% + ${(hotspot.y - 0.5) * displayPct}%)`,
  }
}

/**
 * `<CursorPreview editable>` 内部から呼ぶインタラクションハンドラ束ね。
 *
 * - `el` は `<CursorPreview>` のルート要素 ref。focus / setPointerCapture / releasePointerCapture を呼ぶ。
 * - `hotspot` は現在値の getter (Ref)。`onUpdate` で次値を伝える。
 * - `displayPct` / `referencePx` は ref で渡し、props 変化に追従させる。
 */
export function useHotspotInteraction(options: {
  el: Ref<HTMLElement | null>
  hotspot: Ref<Hotspot>
  displayPct: Ref<number>
  referencePx: Ref<number>
  onUpdate: (next: Hotspot) => void
}) {
  const dragging = ref(false)

  function commitFromPointer(e: PointerEvent) {
    const el = options.el.value
    if (!el) return
    const next = pointerToHotspotRatio(e, el.getBoundingClientRect(), options.displayPct.value)
    options.onUpdate(next)
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return
    dragging.value = true
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture?.(e.pointerId)
    el.focus({ preventScroll: true })
    commitFromPointer(e)
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging.value) return
    commitFromPointer(e)
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging.value) return
    dragging.value = false
    ;(e.currentTarget as Element).releasePointerCapture?.(e.pointerId)
  }

  function onKeydown(e: KeyboardEvent) {
    // テキスト入力中の矢印操作と衝突しないように。
    const tag = (e.target as HTMLElement).tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return

    const next = applyKeyboardNudge(
      options.hotspot.value,
      e.key,
      e.shiftKey,
      options.referencePx.value,
    )
    if (!next) return
    e.preventDefault()
    options.onUpdate(next)
  }

  return {
    dragging,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onKeydown,
  }
}
