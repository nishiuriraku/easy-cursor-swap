<script setup lang="ts" generic="T extends string | number | boolean | null">
/**
 * カスタム select ボックス。
 *
 * ネイティブ `<select>` の OS 描画ではダークモード時に option が白浮きする
 * 問題があったため、自前の listbox + Teleport で実装している。
 *
 * - v-model 対応 (任意の primitive 値: string / number / boolean / null)
 * - キーボード操作: Space/Enter で開閉、↑↓ でハイライト、Enter で確定、Esc で閉じる
 * - ARIA: combobox + listbox / aria-expanded / aria-activedescendant
 * - 外側クリックと Esc で閉じる
 * - listbox は `<Teleport to="body">` + `position: fixed` で描画。
 *   `.prop-section` などの祖先が `overflow: hidden` でも listbox はクリップされない。
 *
 * 状態機械 + キーボードナビ + 位置計算は `useListbox` に共通化済み (audit F43-SIZE-001)。
 * 本ファイルは props / emits + 薄い wrapper + template + style のみ。
 */
import { ref, toRef } from 'vue'
import { useListbox, type ListboxOption } from '~/composables/useListbox'

const props = withDefaults(
  defineProps<{
    modelValue: T
    options: ListboxOption<T>[]
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

const triggerRef = ref<HTMLButtonElement | null>(null)
const listboxRef = ref<HTMLUListElement | null>(null)

// 安定 ID。生成タイミングで毎回ユニーク値にする (pages/* 跨ぎでも衝突しない)。
const uid = props.id ?? `ui-select-${Math.random().toString(36).slice(2, 9)}`
const listboxId = `${uid}-listbox`

const {
  open,
  highlightedIndex,
  placement,
  listboxStyle,
  displayLabel,
  isPlaceholder,
  toggle,
  pick,
  onTriggerKeydown,
} = useListbox<T>({
  options: toRef(props, 'options'),
  modelValue: toRef(props, 'modelValue'),
  placeholder: toRef(props, 'placeholder'),
  triggerRef,
  listboxRef,
  onSelect(value) {
    emit('update:modelValue', value)
    emit('change', value)
  },
})

function onClickTrigger() {
  if (props.disabled) return
  toggle()
}

function onKeydown(e: KeyboardEvent) {
  if (props.disabled) return
  onTriggerKeydown(e)
}
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
      @click="onClickTrigger"
      @keydown="onKeydown"
    >
      <span :class="['ui-select-label', { placeholder: isPlaceholder }]">
        {{ displayLabel }}
      </span>
      <UiIcon name="ChevD" :size="11" class="ui-select-caret" :class="{ open }" />
    </button>

    <!-- listbox は祖先の overflow:hidden に影響されないよう body 直下へ Teleport。
         位置は useListbox().listboxStyle が computed する fixed 座標で制御する。 -->
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
  /* 全モーダル (.modal-page) が z-[100] のため、modal 内 UiSelect の listbox は
   * それより上に出る必要がある。SubmitThemeDialog / NewThemeStartModal /
   * BulkImportPreviewModal / SaveDestinationModal 等で modal 内 select の
   * ドロップダウンが modal 下に隠れる回帰を防ぐ。 */
  z-index: 110;
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
