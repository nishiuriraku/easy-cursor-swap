<script setup lang="ts" generic="T extends string | number | boolean | null">
/**
 * カスタム select ボックス。
 *
 * ネイティブ `<select>` の `<option>` リストはブラウザ/OS の描画になる関係で
 * ダークモード時に白背景の項目が浮いて見える問題があった。
 * UiSelect は CSS を完全コントロールできる div + 自前の listbox で実装し、
 * `--bg-2` 系のトークンに揃ったダーク/ライト両対応の見た目を提供する。
 *
 * - v-model 対応 (任意の primitive 値: string / number / boolean / null)
 * - キーボード操作: Space/Enter で開閉、↑↓ でハイライト、Enter で確定、Esc で閉じる
 * - ARIA: combobox + listbox / aria-expanded / aria-activedescendant
 * - 外側クリックと Esc で閉じる
 * - メニューはボタン直下に表示。スクロールが必要なケースだけ縦最大 280px
 * - `<UiIcon>` (Nuxt 自動 import) でシェブロン表示
 * - listbox は `<Teleport to="body">` + `position: fixed` で描画。
 *   `.prop-section` などの祖先が `overflow: hidden` でも listbox はクリップされない。
 */
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

interface Option<V> {
  value: V
  label: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue: T
    options: Option<T>[]
    placeholder?: string
    width?: string
    disabled?: boolean
    /** ボタン側に追加で当てる class (`input` と組み合わせ可能) */
    triggerClass?: string
    /** 表示位置に使う ID。複数置く場合の listbox 関連属性用 */
    id?: string
  }>(),
  {
    placeholder: '',
    width: '160px',
    disabled: false,
    triggerClass: '',
    id: undefined,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: T]
  change: [value: T]
}>()

const open = ref(false)
const highlightedIndex = ref(-1)
const triggerRef = ref<HTMLButtonElement | null>(null)
const listboxRef = ref<HTMLUListElement | null>(null)
/**
 * listbox の表示方向。`'down'` で trigger の下、`'up'` で上に開く。
 * Teleport で body 直下に描画するため、viewport を計測して decide する。
 */
const placement = ref<'down' | 'up'>('down')

/**
 * Teleport された listbox の `position: fixed` 用座標。
 * `updateListboxPosition()` で trigger の getBoundingClientRect() から導出。
 * scroll/resize 時にも追従更新する。
 */
const listboxStyle = ref<Record<string, string>>({})

/** listbox の推定高さ (px)。viewport 計算で auto-flip 判定に使う。
 * 実測すると初回 show 時にチラつくため、option 行高 (32px) × 件数 + padding/border から推定。
 * scoped CSS の max-height: 280px で頭打ち。 */
const ITEM_HEIGHT_PX = 32
const LISTBOX_VPAD_PX = 10 /* 上下 padding (4px) + border (1px*2) ≒ 余裕分 */
const LISTBOX_MAX_HEIGHT_PX = 280
const LISTBOX_GAP_PX = 4 /* trigger と listbox の隙間 */
const LISTBOX_VIEWPORT_MARGIN_PX = 8 /* viewport 端からの最小マージン */

function computeListboxHeight(): number {
  return Math.min(LISTBOX_MAX_HEIGHT_PX, props.options.length * ITEM_HEIGHT_PX + LISTBOX_VPAD_PX)
}

/**
 * trigger の現在位置を基準に、listbox を viewport のどちらに開くか決定し、
 * `position: fixed` 用の座標 (top/left/width/maxHeight) を `listboxStyle` に書き込む。
 *
 * Teleport で body 直下に描画されるため、ancestor の `overflow: hidden` に
 * クリップされない。位置は scroll/resize で随時 update。
 */
function updateListboxPosition() {
  const trigger = triggerRef.value
  if (!trigger || typeof window === 'undefined') return
  const rect = trigger.getBoundingClientRect()
  const listboxH = computeListboxHeight()
  const spaceBelow = window.innerHeight - rect.bottom - LISTBOX_VIEWPORT_MARGIN_PX
  const spaceAbove = rect.top - LISTBOX_VIEWPORT_MARGIN_PX

  // 下に収まれば 'down'、収まらず上に余裕があるなら 'up'。
  // どちらにも収まらない場合は広い方を採用し、その方向の使える高さを max-height にする。
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
    // 上開きは bottom を viewport 底からの距離で指定する。
    style.bottom = `${Math.round(window.innerHeight - rect.top + LISTBOX_GAP_PX)}px`
  }
  listboxStyle.value = style
}

// 動的追従用ハンドラ。`{ passive: true, capture: true }` でスクロール可能な
// 全 ancestor の scroll イベントも拾う (capture が無いと .settings-content など
// 内側スクロール container は拾えない)。
function onWindowChange() {
  if (open.value) updateListboxPosition()
}

// 安定 ID。生成タイミングで毎回ユニーク値にする (pages/* 跨ぎでも衝突しない)。
const uid = props.id ?? `ui-select-${Math.random().toString(36).slice(2, 9)}`
const listboxId = `${uid}-listbox`

const selectedIndex = computed(() =>
  props.options.findIndex((o) => Object.is(o.value, props.modelValue)),
)

const displayLabel = computed(() => {
  const i = selectedIndex.value
  if (i < 0) return props.placeholder
  return props.options[i]?.label ?? props.placeholder
})

const isPlaceholder = computed(() => selectedIndex.value < 0)

function toggle() {
  if (props.disabled) return
  if (open.value) {
    close()
  } else {
    show()
  }
}

function show() {
  // 先に位置と方向を決定 (DOM 挿入前)。ボタンの現在位置を基準に算出するので open=true より前に呼ぶ。
  updateListboxPosition()
  open.value = true
  // 開いた瞬間は選択中をハイライト。未選択なら先頭の有効項目。
  const sel = selectedIndex.value
  highlightedIndex.value = sel >= 0 ? sel : firstEnabledIndex()
  void nextTick(() => {
    // Teleport で実 DOM に追加されたあとに再計算 (option の実寸を反映)
    updateListboxPosition()
    scrollHighlightIntoView()
  })
}

function close() {
  open.value = false
  highlightedIndex.value = -1
}

function firstEnabledIndex(): number {
  return props.options.findIndex((o) => !o.disabled)
}

function lastEnabledIndex(): number {
  for (let i = props.options.length - 1; i >= 0; i--) {
    if (!props.options[i]!.disabled) return i
  }
  return -1
}

function moveHighlight(delta: 1 | -1) {
  if (!props.options.length) return
  let i = highlightedIndex.value
  for (let step = 0; step < props.options.length; step++) {
    i = (i + delta + props.options.length) % props.options.length
    if (!props.options[i]!.disabled) {
      highlightedIndex.value = i
      void nextTick(() => scrollHighlightIntoView())
      return
    }
  }
}

/**
 * listbox 内のハイライト項目を listbox 内部だけスクロールして可視にする。
 *
 * 旧実装は `item.scrollIntoView({ block: 'nearest' })` を使っていたが、これは
 * listbox が viewport 外にある場合に scroll 伝播してページ全体までスクロール
 * させてしまう (= 閉じた後にトリガーの位置が動いて見える原因)。
 * scroll を listbox 自身に閉じ込めるため、`listbox.scrollTop` を直接調整する。
 */
function scrollHighlightIntoView() {
  const list = listboxRef.value
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

function pick(index: number) {
  const opt = props.options[index]
  if (!opt || opt.disabled) return
  emit('update:modelValue', opt.value)
  emit('change', opt.value)
  close()
  // フォーカスはトリガーへ戻して連続キーボード操作を可能にする
  triggerRef.value?.focus()
}

function onTriggerKeydown(e: KeyboardEvent) {
  if (props.disabled) return
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
      // Tab はフォーカス遷移を許可しつつ閉じる
      close()
      break
    default:
      break
  }
}

// 外側クリックで閉じる。document.click を on/off するシンプル戦略。
function onDocumentMouseDown(e: MouseEvent) {
  const target = e.target as Node | null
  if (!target) return
  if (triggerRef.value?.contains(target) || listboxRef.value?.contains(target)) {
    return
  }
  close()
}

watch(open, (v) => {
  if (typeof document === 'undefined') return
  if (v) {
    document.addEventListener('mousedown', onDocumentMouseDown)
    // capture: true で全 ancestor の scroll を拾い、内側スクロール container
    // (settings-content / drawer body 等) でも listbox 位置が trigger に追従する。
    window.addEventListener('scroll', onWindowChange, { passive: true, capture: true })
    window.addEventListener('resize', onWindowChange, { passive: true })
  } else {
    document.removeEventListener('mousedown', onDocumentMouseDown)
    window.removeEventListener('scroll', onWindowChange, { capture: true } as EventListenerOptions)
    window.removeEventListener('resize', onWindowChange)
  }
})

onBeforeUnmount(() => {
  if (typeof document !== 'undefined') {
    document.removeEventListener('mousedown', onDocumentMouseDown)
  }
  if (typeof window !== 'undefined') {
    window.removeEventListener('scroll', onWindowChange, { capture: true } as EventListenerOptions)
    window.removeEventListener('resize', onWindowChange)
  }
})
</script>

<template>
  <div class="ui-select" :class="{ disabled }" :style="{ width }">
    <button
      :id="uid"
      ref="triggerRef"
      type="button"
      :class="['ui-select-trigger input', triggerClass]"
      :aria-haspopup="'listbox'"
      :aria-expanded="open"
      :aria-controls="listboxId"
      :aria-activedescendant="
        open && highlightedIndex >= 0 ? `${uid}-opt-${highlightedIndex}` : undefined
      "
      :disabled="disabled"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span :class="['ui-select-label', { placeholder: isPlaceholder }]">
        {{ displayLabel }}
      </span>
      <UiIcon name="ChevD" :size="11" class="ui-select-caret" :class="{ open }" />
    </button>

    <!-- listbox は祖先の overflow:hidden に影響されないよう body 直下へ Teleport。
         位置は updateListboxPosition() が computed する fixed 座標で制御する。 -->
    <Teleport to="body">
      <Transition :name="placement === 'up' ? 'ui-select-fade-up' : 'ui-select-fade'">
        <ul
          v-if="open"
          :id="listboxId"
          ref="listboxRef"
          :class="['ui-select-listbox', placement === 'up' ? 'up' : 'down']"
          :style="listboxStyle"
          role="listbox"
          :aria-labelledby="uid"
          tabindex="-1"
        >
          <li
            v-for="(opt, i) in options"
            :id="`${uid}-opt-${i}`"
            :key="String(opt.value) + ':' + i"
            role="option"
            :class="[
              'ui-select-item',
              {
                selected: Object.is(opt.value, modelValue),
                highlighted: i === highlightedIndex,
                disabled: opt.disabled,
              },
            ]"
            :aria-selected="Object.is(opt.value, modelValue)"
            :aria-disabled="opt.disabled || undefined"
            @mouseenter="!opt.disabled && (highlightedIndex = i)"
            @click="pick(i)"
          >
            <span class="ui-select-item-label">{{ opt.label }}</span>
            <UiIcon
              v-if="Object.is(opt.value, modelValue)"
              name="Check"
              :size="12"
              class="ui-select-item-check"
            />
          </li>
        </ul>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.ui-select {
  position: relative;
  display: inline-block;
}
.ui-select.disabled {
  opacity: 0.55;
  pointer-events: none;
}

/* trigger は既存の .input トークンに合わせて違和感なく統一。
   ただし select 用にカーソルとレイアウトだけ上書き。 */
.ui-select-trigger {
  width: 100%;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 10px;
  cursor: pointer;
  font-family: inherit;
  font-size: 12.5px;
  color: var(--fg);
  text-align: left;
}
.ui-select-trigger:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.ui-select-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ui-select-label.placeholder {
  color: var(--fg-mute);
}
.ui-select-caret {
  flex-shrink: 0;
  color: var(--fg-dim);
  transition: transform 0.16s ease;
}
.ui-select-caret.open {
  transform: rotate(180deg);
  color: var(--accent);
}

/* listbox は <Teleport to="body"> + position: fixed (inline style) で配置するため、
 * top/left/right/bottom/width/max-height はコンポーネント側の updateListboxPosition()
 * が computed する。ここでは表示のみ担当。
 * `up` / `down` クラスは Transition 切替と debugging 用の placement 識別子。
 * `--bg-glass-hi` のトークンを採用 (元 select の白背景を解消) */
.ui-select-listbox {
  margin: 0;
  padding: 4px;
  list-style: none;
  background: var(--bg-2);
  border: 1px solid var(--line-hi);
  border-radius: 8px;
  box-shadow:
    0 12px 32px -12px rgba(0, 0, 0, 0.55),
    0 0 0 1px rgba(0, 0, 0, 0.25);
  overflow-y: auto;
  z-index: 80;
  /* glassmorphism の代わりにフラットで読みやすさ優先 */
}
html.light .ui-select-listbox {
  background: var(--bg-1);
  box-shadow:
    0 12px 32px -12px rgba(15, 20, 35, 0.18),
    0 0 0 1px rgba(15, 20, 35, 0.08);
}

.ui-select-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 7px 10px;
  font-size: 12.5px;
  color: var(--fg);
  border-radius: 6px;
  cursor: pointer;
  user-select: none;
}
.ui-select-item.highlighted {
  background: rgba(124, 242, 212, 0.1);
  color: var(--accent);
}
html.light .ui-select-item.highlighted {
  background: rgba(15, 168, 133, 0.1);
  color: var(--accent);
}
.ui-select-item.selected {
  color: var(--accent);
}
.ui-select-item.selected.highlighted {
  background: var(--accent-dim);
}
.ui-select-item.disabled {
  color: var(--fg-faint);
  cursor: not-allowed;
}
.ui-select-item-check {
  flex-shrink: 0;
  color: var(--accent);
}
.ui-select-item-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 出現/消失アニメーション (down: 上から降りてくる、up: 下から上がってくる) */
.ui-select-fade-enter-active,
.ui-select-fade-leave-active,
.ui-select-fade-up-enter-active,
.ui-select-fade-up-leave-active {
  transition:
    opacity 0.12s ease,
    transform 0.12s ease;
}
.ui-select-fade-enter-from,
.ui-select-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
.ui-select-fade-up-enter-from,
.ui-select-fade-up-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
