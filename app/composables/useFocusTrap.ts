/**
 * モーダル / ダイアログ用 focus trap composable。
 *
 * - active=true の間、container 内の focusable 要素を querySelector で列挙し、
 *   Tab / Shift+Tab が container 末端に達したら先頭 / 末尾に wrap する。
 * - active が true に立ち上がった瞬間、container 内の最初の focusable へ自動 focus。
 * - active が false に落ちた瞬間、active=true 直前にフォーカスされていた
 *   要素へ復帰する (要素がまだ DOM 上に居る場合のみ)。
 *
 * scroll lock と Esc キーは別途 `useModalLifecycle` が担当する。本 composable は
 * 純粋にキーボードフォーカスのみを引き受ける。
 */
import type { Ref } from 'vue'

const FOCUSABLE_SELECTOR = [
  'button:not([disabled])',
  '[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

function focusableIn(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute('disabled') && el.offsetParent !== null,
  )
}

export function useFocusTrap(containerRef: Ref<HTMLElement | null>, active: Ref<boolean>): void {
  let previousActive: HTMLElement | null = null

  function onKeydown(e: KeyboardEvent) {
    if (!active.value || e.key !== 'Tab') return
    const container = containerRef.value
    if (!container) return
    const focusables = focusableIn(container)
    if (focusables.length === 0) {
      e.preventDefault()
      return
    }
    const first = focusables[0]
    const last = focusables[focusables.length - 1]
    const current = document.activeElement as HTMLElement | null
    if (e.shiftKey) {
      if (current === first || !container.contains(current)) {
        e.preventDefault()
        last.focus()
      }
    } else {
      if (current === last || !container.contains(current)) {
        e.preventDefault()
        first.focus()
      }
    }
  }

  function activate(): void {
    if (typeof document === 'undefined') return
    previousActive = (document.activeElement as HTMLElement | null) ?? null
    document.addEventListener('keydown', onKeydown)
    // container が既に DOM に乗っていればその場で focus、まだなら次の tick で再試行。
    // Teleport 配下の modal のように container の生成が後追いになるケースを許容する。
    const tryFocus = () => {
      const container = containerRef.value
      if (!container) return
      const focusables = focusableIn(container)
      if (focusables.length > 0) focusables[0].focus()
    }
    if (containerRef.value) {
      tryFocus()
    } else {
      nextTick(tryFocus)
    }
  }

  function deactivate(): void {
    if (typeof document === 'undefined') return
    document.removeEventListener('keydown', onKeydown)
    if (previousActive && document.body.contains(previousActive)) {
      previousActive.focus()
    }
    previousActive = null
  }

  watch(
    active,
    (val) => {
      if (val) activate()
      else deactivate()
    },
    { immediate: true },
  )

  onBeforeUnmount(deactivate)
}
