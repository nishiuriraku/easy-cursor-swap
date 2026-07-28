<script setup lang="ts">
/**
 * 共通スピナー。Tailwind v4 shared `.spinner` リング (tailwind.css) の薄い Vue ラッパ。
 *
 * - size:  px (既定 20 = `.spinner` の size-5)。`.spinner` の `@apply size-5` を
 *          inline style が常に上書きする (UiButton と同じ手法)。
 * - label: 与えると `role="status"` + `aria-label` でスクリーンリーダーに通知。
 *          省略時は装飾とみなし `aria-hidden`。ボタン内など既にテキストがある場所では
 *          label を渡さず装飾にする。
 *
 * LD1: `<span class="spinner">` の inline 重複 (CreatorToolbar / LibrarySection /
 * LoggingSection / ConfigRecoveryPanel など) をこのコンポーネントに集約する。
 */
const props = withDefaults(
  defineProps<{
    size?: number
    label?: string
  }>(),
  {
    size: 20,
  },
)
</script>

<template>
  <span
    class="spinner"
    :style="{ width: `${props.size}px`, height: `${props.size}px` }"
    :role="props.label ? 'status' : undefined"
    :aria-label="props.label || undefined"
    :aria-hidden="props.label ? undefined : 'true'"
  />
</template>
