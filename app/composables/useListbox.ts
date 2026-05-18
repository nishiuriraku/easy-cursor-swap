/**
 * `UiSelect` の listbox 状態機械 + キーボードナビ + Teleport 位置計算を
 * 担う composable。
 *
 * 経緯: ネイティブ `<select>` の OS 描画ではダークモード時に option リストが
 * 白浮きする問題があり、`UiSelect` で `<Teleport to="body">` + `position: fixed`
 * を使う自前 listbox を実装している。元の SFC は 528 行 / script 318 行
 * (audit F43-SIZE-001) と長大で、template / style の見通しを悪化させていたため、
 * 本 composable にロジックを集約して SFC を thin に保つ。
 *
 * 利用側 (`UiSelect.vue`) に残るのは:
 *  - props / emits 宣言
 *  - composable のインスタンス化
 *  - listbox / item の template + style
 */
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref } from 'vue'

export interface ListboxOption<V> {
  value: V
  label: string
  disabled?: boolean
}

export interface UseListboxOptions<V> {
  options: Ref<ListboxOption<V>[]>
  modelValue: Ref<V>
  placeholder: Ref<string>
  triggerRef: Ref<HTMLElement | null>
  listboxRef: Ref<HTMLUListElement | null>
  /** 値が選択された時のコールバック (emit 委譲) */
  onSelect: (value: V) => void
}

/** option 行高 / padding / max-height などの定数群。 */
const ITEM_HEIGHT_PX = 32
const LISTBOX_VPAD_PX = 10
const LISTBOX_MAX_HEIGHT_PX = 280
const LISTBOX_GAP_PX = 4
const LISTBOX_VIEWPORT_MARGIN_PX = 8

export function useListbox<V>(opts: UseListboxOptions<V>) {
  const open = ref(false)
  const highlightedIndex = ref(-1)
  const placement = ref<'down' | 'up'>('down')
  const listboxStyle = ref<Record<string, string>>({})

  const selectedIndex = computed(() =>
    opts.options.value.findIndex((o) => Object.is(o.value, opts.modelValue.value)),
  )

  const displayLabel = computed(() => {
    const i = selectedIndex.value
    if (i < 0) return opts.placeholder.value
    return opts.options.value[i]?.label ?? opts.placeholder.value
  })

  const isPlaceholder = computed(() => selectedIndex.value < 0)

  function computeListboxHeight(): number {
    return Math.min(
      LISTBOX_MAX_HEIGHT_PX,
      opts.options.value.length * ITEM_HEIGHT_PX + LISTBOX_VPAD_PX,
    )
  }

  /**
   * trigger の現在位置を基準に listbox を viewport のどちらに開くか決定し、
   * `position: fixed` 用の座標を `listboxStyle` に書き込む。
   * Teleport で body 直下に描画されるので、ancestor の overflow:hidden に
   * クリップされない。scroll/resize で随時 update。
   */
  function updateListboxPosition() {
    const trigger = opts.triggerRef.value
    if (!trigger || typeof window === 'undefined') return
    const rect = trigger.getBoundingClientRect()
    const listboxH = computeListboxHeight()
    const spaceBelow = window.innerHeight - rect.bottom - LISTBOX_VIEWPORT_MARGIN_PX
    const spaceAbove = rect.top - LISTBOX_VIEWPORT_MARGIN_PX

    let dir: 'down' | 'up'
    let maxHeight: number
    if (spaceBelow >= listboxH + LISTBOX_GAP_PX) {
      dir = 'down'
      maxHeight = Math.min(LISTBOX_MAX_HEIGHT_PX, spaceBelow - LISTBOX_GAP_PX)
    } else if (spaceAbove >= listboxH + LISTBOX_GAP_PX) {
      dir = 'up'
      maxHeight = Math.min(LISTBOX_MAX_HEIGHT_PX, spaceAbove - LISTBOX_GAP_PX)
    } else if (spaceBelow >= spaceAbove) {
      dir = 'down'
      maxHeight = Math.max(LISTBOX_VPAD_PX + ITEM_HEIGHT_PX, spaceBelow - LISTBOX_GAP_PX)
    } else {
      dir = 'up'
      maxHeight = Math.max(LISTBOX_VPAD_PX + ITEM_HEIGHT_PX, spaceAbove - LISTBOX_GAP_PX)
    }
    placement.value = dir

    const style: Record<string, string> = {
      position: 'fixed',
      left: `${Math.round(rect.left)}px`,
      width: `${Math.round(rect.width)}px`,
      maxHeight: `${Math.round(maxHeight)}px`,
    }
    if (dir === 'down') {
      style.top = `${Math.round(rect.bottom + LISTBOX_GAP_PX)}px`
    } else {
      style.bottom = `${Math.round(window.innerHeight - rect.top + LISTBOX_GAP_PX)}px`
    }
    listboxStyle.value = style
  }

  function onWindowChange() {
    if (open.value) updateListboxPosition()
  }

  function firstEnabledIndex(): number {
    return opts.options.value.findIndex((o) => !o.disabled)
  }

  function lastEnabledIndex(): number {
    const arr = opts.options.value
    for (let i = arr.length - 1; i >= 0; i--) {
      if (!arr[i]!.disabled) return i
    }
    return -1
  }

  /**
   * listbox 内のハイライト項目を listbox 内部だけスクロールして可視にする。
   * `Element.scrollIntoView` を使うと scroll が祖先まで伝播してページ全体が
   * 動いてしまうため、`listbox.scrollTop` を直接調整する。
   */
  function scrollHighlightIntoView() {
    const list = opts.listboxRef.value
    if (!list) return
    const item = list.children[highlightedIndex.value] as HTMLElement | undefined
    if (!item) return
    const listRect = list.getBoundingClientRect()
    const itemRect = item.getBoundingClientRect()
    if (itemRect.top < listRect.top) {
      list.scrollTop -= listRect.top - itemRect.top
    } else if (itemRect.bottom > listRect.bottom) {
      list.scrollTop += itemRect.bottom - listRect.bottom
    }
  }

  function moveHighlight(delta: 1 | -1) {
    const arr = opts.options.value
    if (!arr.length) return
    let i = highlightedIndex.value
    for (let step = 0; step < arr.length; step++) {
      i = (i + delta + arr.length) % arr.length
      if (!arr[i]!.disabled) {
        highlightedIndex.value = i
        void nextTick(() => scrollHighlightIntoView())
        return
      }
    }
  }

  function show() {
    updateListboxPosition()
    open.value = true
    const sel = selectedIndex.value
    highlightedIndex.value = sel >= 0 ? sel : firstEnabledIndex()
    void nextTick(() => {
      updateListboxPosition()
      scrollHighlightIntoView()
    })
  }

  function close() {
    open.value = false
    highlightedIndex.value = -1
  }

  function toggle() {
    if (open.value) close()
    else show()
  }

  function pick(index: number) {
    const opt = opts.options.value[index]
    if (!opt || opt.disabled) return
    opts.onSelect(opt.value)
    close()
    opts.triggerRef.value?.focus()
  }

  function onTriggerKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Enter':
      case ' ':
      case 'ArrowDown':
        e.preventDefault()
        if (!open.value) show()
        else if (e.key === 'ArrowDown') moveHighlight(1)
        else if (highlightedIndex.value >= 0) pick(highlightedIndex.value)
        break
      case 'ArrowUp':
        e.preventDefault()
        if (!open.value) show()
        else moveHighlight(-1)
        break
      case 'Home':
        if (open.value) {
          e.preventDefault()
          highlightedIndex.value = firstEnabledIndex()
          void nextTick(() => scrollHighlightIntoView())
        }
        break
      case 'End':
        if (open.value) {
          e.preventDefault()
          highlightedIndex.value = lastEnabledIndex()
          void nextTick(() => scrollHighlightIntoView())
        }
        break
      case 'Escape':
        if (open.value) {
          e.preventDefault()
          close()
        }
        break
      case 'Tab':
        close()
        break
    }
  }

  function onDocumentMouseDown(e: MouseEvent) {
    const target = e.target as Node | null
    if (!target) return
    if (opts.triggerRef.value?.contains(target) || opts.listboxRef.value?.contains(target)) return
    close()
  }

  watch(open, (v) => {
    if (typeof document === 'undefined') return
    if (v) {
      document.addEventListener('mousedown', onDocumentMouseDown)
      window.addEventListener('scroll', onWindowChange, { passive: true, capture: true })
      window.addEventListener('resize', onWindowChange, { passive: true })
    } else {
      document.removeEventListener('mousedown', onDocumentMouseDown)
      window.removeEventListener('scroll', onWindowChange, {
        capture: true,
      } as EventListenerOptions)
      window.removeEventListener('resize', onWindowChange)
    }
  })

  onBeforeUnmount(() => {
    if (typeof document !== 'undefined') {
      document.removeEventListener('mousedown', onDocumentMouseDown)
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('scroll', onWindowChange, {
        capture: true,
      } as EventListenerOptions)
      window.removeEventListener('resize', onWindowChange)
    }
  })

  return {
    open,
    highlightedIndex,
    placement,
    listboxStyle,
    selectedIndex,
    displayLabel,
    isPlaceholder,
    toggle,
    show,
    close,
    pick,
    moveHighlight,
    onTriggerKeydown,
  }
}
