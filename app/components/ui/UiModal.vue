<script setup lang="ts">
/**
 * 共通モーダル shell。
 *
 * 機能:
 * - `<Teleport to="body">` で z-index / overflow 階層を独立化
 * - useModalLifecycle で body scroll lock + Escape を統一
 * - useFocusTrap で Tab cycling + フォーカス復帰
 * - busy=true の間は backdrop/Esc を無効化 (誤キャンセル防止)
 *
 * slots:
 * - default      : body コンテンツ (modal-body 内)
 * - actions      : フッター右の主操作群 (省略時は modal-foot 非表示)
 * - leftNote     : フッター左の補助テキスト
 * - headExtra    : タイトル右の追加要素 (例: signed タグ)
 *
 * 既存 `.modal-page` / `.modal` / `.modal-head` / `.modal-icon` / `.modal-body`
 * / `.modal-foot` 共通 CSS をそのまま使用。
 */
const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    icon?: string
    iconTone?: 'accent' | 'danger' | 'warn' | 'success'
    size?: 'sm' | 'md' | 'lg'
    closeOnBackdrop?: boolean
    closeOnEsc?: boolean
    busy?: boolean
    bodyPadded?: boolean
    ariaLabelledby?: string
  }>(),
  {
    iconTone: 'accent',
    size: 'md',
    closeOnBackdrop: true,
    closeOnEsc: true,
    busy: false,
    bodyPadded: true,
  },
)

const emit = defineEmits<{
  'update:open': [value: boolean]
  close: []
}>()

const slots = useSlots()
const titleId = computed(() => props.ariaLabelledby ?? `ui-modal-${useId()}`)
const hasFooter = computed(() => Boolean(slots.actions || slots.leftNote))

const modalRef = ref<HTMLElement | null>(null)
const active = computed(() => props.open)

useModalLifecycle({
  open: active,
  onClose() {
    if (props.busy) return
    if (!props.closeOnEsc) return
    emit('update:open', false)
    emit('close')
  },
})

useFocusTrap(modalRef, active)

const iconToneClass = computed(() => `tone-${props.iconTone}`)
const sizeClass = computed(() => `size-${props.size}`)

function onBackdrop(e: MouseEvent) {
  if (e.target !== e.currentTarget) return
  if (props.busy || !props.closeOnBackdrop) return
  emit('update:open', false)
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ui-modal-fade">
      <div
        v-if="open"
        class="modal-page"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        @click="onBackdrop"
      >
        <div ref="modalRef" :class="['modal', sizeClass]" @click.stop>
          <div class="modal-head">
            <div v-if="icon" :class="['modal-icon', iconToneClass]" aria-hidden="true">
              <UiIcon :name="icon" :size="20" />
            </div>
            <div class="modal-head-text">
              <h2 :id="titleId">{{ title }}</h2>
              <p v-if="description">{{ description }}</p>
            </div>
            <slot name="headExtra" />
          </div>

          <div :class="['modal-body', { 'p-0': !bodyPadded }]">
            <slot />
          </div>

          <div v-if="hasFooter" class="modal-foot">
            <div v-if="$slots.leftNote" class="left-note">
              <slot name="leftNote" />
            </div>
            <div v-else />
            <div v-if="$slots.actions" class="actions">
              <slot name="actions" />
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.modal-head-text {
  @apply min-w-0 flex-1;
}

.modal.size-sm {
  @apply w-[420px];
}
.modal.size-md {
  @apply w-[540px];
}
.modal.size-lg {
  @apply w-[720px];
}

/* Icon tone overrides (default = accent, already styled by .modal-icon). */
.modal-icon.tone-danger {
  border-color: rgba(255, 107, 138, 0.35);
  color: var(--rose);
  background: rgba(255, 107, 138, 0.12);
}
.modal-icon.tone-warn {
  border-color: rgba(245, 158, 11, 0.35);
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.12);
}
.modal-icon.tone-success {
  border-color: var(--accent-line);
  color: var(--accent);
  background: rgba(124, 242, 212, 0.12);
}

.ui-modal-fade-enter-active,
.ui-modal-fade-leave-active {
  transition: opacity 0.18s ease-out;
}
.ui-modal-fade-enter-from,
.ui-modal-fade-leave-to {
  opacity: 0;
}
</style>
