<script setup lang="ts">
/**
 * クリエイターの中央エディタ下部にある 6 サイズタイル列。
 * 各サイズに画像が割り当て済みなら、その実画像 (Blob URL) を表示する。
 * 未割り当ては + アイコンのまま。
 */
const props = defineProps<{
  /** 32, 48, 64, 96, 128, 256 のいずれか */
  sizes: number[]
  filledSizes: number[]
  activeSize: number
  role: string
  /** サイズ → 実画像 (Blob URL) のマップ。filledSizes に含まれるサイズだけキーがあれば良い。 */
  previewMap?: Record<number, string>
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
      <template v-if="filledSizes.includes(s)">
        <img
          v-if="previewMap && previewMap[s]"
          :src="previewMap[s]"
          alt=""
          :style="{
            width: tileIconSize(s) + 'px',
            height: tileIconSize(s) + 'px',
          }"
          class="size-tile-img"
          draggable="false"
        />
        <CursorIcon v-else :role="role" :size="tileIconSize(s)" style="color: var(--fg)" />
      </template>
      <UiIcon v-else name="Plus" :size="14" />
      <span class="lbl">{{ s }}px</span>
    </button>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.size-strip {
  @apply flex gap-2.5 rounded-[10px] border border-line p-1.5;
  background: rgba(255, 255, 255, 0.02);
}
.size-tile {
  @apply relative grid size-16 cursor-pointer place-items-center rounded-[7px] border border-line;
  background:
    repeating-conic-gradient(rgba(255, 255, 255, 0.03) 0% 25%, transparent 0% 50%) 0 / 8px 8px,
    var(--bg-2);
  transition: border 0.12s;
}
.size-tile:hover {
  @apply border-line-hi;
}
.size-tile.active {
  border-color: var(--accent);
  box-shadow:
    0 0 0 1px var(--accent),
    inset 0 0 0 1px rgba(124, 242, 212, 0.2);
}
.size-tile.empty {
  @apply text-fg-faint;
}
.size-tile-img {
  image-rendering: pixelated;
  object-fit: contain;
}
.size-tile .lbl {
  @apply absolute -right-1.5 -top-1.5 rounded border border-line bg-bg-2 px-1 py-px font-mono text-[9.5px] text-fg-mute;
}
.size-tile.active .lbl {
  @apply border-accent-line text-accent;
  background: rgba(124, 242, 212, 0.08);
}
</style>
