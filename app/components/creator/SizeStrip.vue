<script setup lang="ts">
/**
 * クリエイターの中央エディタ下部にある 6 サイズタイル列。
 * 各サイズに画像が割り当て済みなら CursorIcon、未割り当てなら + アイコン。
 */
const props = defineProps<{
  /** 32, 48, 64, 96, 128, 256 のいずれか */
  sizes: number[]
  filledSizes: number[]
  activeSize: number
  role: string
}>()

defineEmits<{
  select: [size: number]
}>()

function tileIconSize(s: number): number {
  return Math.min(40, Math.floor(s * 0.5))
}
</script>

<template>
  <div class="size-strip">
    <button
      v-for="s in props.sizes"
      :key="s"
      :class="['size-tile', { active: s === activeSize, empty: !filledSizes.includes(s) }]"
      @click="$emit('select', s)"
    >
      <CursorIcon
        v-if="filledSizes.includes(s)"
        :role="role"
        :size="tileIconSize(s)"
        style="color: var(--fg)"
      />
      <UiIcon v-else name="Plus" :size="14" />
      <span class="lbl">{{ s }}px</span>
    </button>
  </div>
</template>

<style scoped>
/* `.size-tile` 本体は global.css 側で完全に定義済み。ここではスタイル追加なし。 */
</style>
