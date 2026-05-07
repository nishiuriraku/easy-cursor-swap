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
  open.value = true
  // 開いた瞬間は選択中をハイライト。未選択なら先頭の有効項目。
  const sel = selectedIndex.value
  highlightedIndex.value = sel >= 0 ? sel : firstEnabledIndex()
  void nextTick(() => scrollHighlightIntoView())
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

function scrollHighlightIntoView() {
  const list = listboxRef.value
  if (!list) return
  const item = list.children[highlightedIndex.value] as HTMLElement | undefined
  item?.scrollIntoView({ block: 'nearest' })
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
  if (
    triggerRef.value?.contains(target) ||
    listboxRef.value?.contains(target)
  ) {
    return
  }
  close()
}

watch(open, (v) => {
  if (typeof document === 'undefined') return
  if (v) {
    document.addEventListener('mousedown', onDocumentMouseDown)
  } else {
    document.removeEventListener('mousedown', onDocumentMouseDown)
  }
})

onBeforeUnmount(() => {
  if (typeof document !== 'undefined') {
    document.removeEventListener('mousedown', onDocumentMouseDown)
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
      :aria-activedescendant="open && highlightedIndex >= 0 ? `${uid}-opt-${highlightedIndex}` : undefined"
      :disabled="disabled"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span :class="['ui-select-label', { placeholder: isPlaceholder }]">
        {{ displayLabel }}
      </span>
      <UiIcon name="ChevD" :size="11" class="ui-select-caret" :class="{ open }" />
    </button>

    <Transition name="ui-select-fade">
      <ul
        v-if="open"
        :id="listboxId"
        ref="listboxRef"
        class="ui-select-listbox"
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

/* listbox はトリガー直下、`--bg-glass-hi` のトークンを採用 (元 select の白背景を解消) */
.ui-select-listbox {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  margin: 0;
  padding: 4px;
  list-style: none;
  background: var(--bg-2);
  border: 1px solid var(--line-hi);
  border-radius: 8px;
  box-shadow:
    0 12px 32px -12px rgba(0, 0, 0, 0.55),
    0 0 0 1px rgba(0, 0, 0, 0.25);
  max-height: 280px;
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
  background: rgba(124, 242, 212, 0.10);
  color: var(--accent);
}
html.light .ui-select-item.highlighted {
  background: rgba(15, 168, 133, 0.10);
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

/* 出現/消失アニメーション */
.ui-select-fade-enter-active,
.ui-select-fade-leave-active {
  transition:
    opacity 0.12s ease,
    transform 0.12s ease;
}
.ui-select-fade-enter-from,
.ui-select-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
