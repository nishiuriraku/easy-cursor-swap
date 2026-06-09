<script setup lang="ts">
/**
 * 汎用シマースケルトン。読込中のブランクを埋めるプレースホルダ片。
 *
 * 共有 `.skeleton` utility (tailwind.css, light 追従) にサイズ/角丸だけ被せる薄いラッパ。
 * width / height は CSS の長さ文字列をそのまま渡す (既定は親いっぱい × 1em)。
 *
 * LD2: index.vue / marketplace.vue のスケルトン + CursorMatrix のセル + marketplace
 * 詳細プレビュー列で再利用する。装飾要素なので常に `aria-hidden`。
 */
const props = withDefaults(
  defineProps<{
    width?: string
    height?: string
    radius?: string
    circle?: boolean
  }>(),
  {
    width: '100%',
    height: '1em',
    radius: 'var(--radius)',
  },
)

const style = computed(() => ({
  width: props.width,
  height: props.height,
  borderRadius: props.circle ? '999px' : props.radius,
}))
</script>

<template>
  <div class="skeleton" :style="style" aria-hidden="true" />
</template>
