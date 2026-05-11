<script setup lang="ts">
/**
 * .ani をアニメーション再生するサムネ / プレビュー要素。
 *
 * - `editable=false` (既定): 画像表示のみ。一括取り込みプレビューモーダル等で使用。
 * - `editable=true`: 十字線オーバーレイ + クリック/ドラッグで `hotspot-pick` / `hotspot-drag` emit。
 *   creator.vue メインパネルで使用する。
 */
import { computed } from 'vue'
import { useAniPlayer } from '~/composables/useAniPlayer'

interface Props {
  framePngs: readonly Uint8Array[]
  sequence: readonly number[]
  durations: readonly number[]
  width: number
  height: number
  editable?: boolean
  hotspot?: { x: number; y: number } | null
}
const props = withDefaults(defineProps<Props>(), {
  editable: false,
  hotspot: null,
})

const emit = defineEmits<{
  (e: 'hotspot-pick', value: { x: number; y: number }): void
  (e: 'hotspot-drag', value: { x: number; y: number }): void
  (e: 'hotspot-drag-end'): void
}>()

const player = useAniPlayer({
  framePngs: props.framePngs,
  sequence: props.sequence,
  perStepDurationsMs: props.durations,
})

const hotspotPercent = computed(() => {
  if (!props.hotspot || props.width === 0 || props.height === 0) return null
  return {
    left: `${(props.hotspot.x / props.width) * 100}%`,
    top: `${(props.hotspot.y / props.height) * 100}%`,
  }
})

let dragging = false

function pointToCoord(e: MouseEvent, el: HTMLElement): { x: number; y: number } {
  const rect = el.getBoundingClientRect()
  const ratioX = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width))
  const ratioY = Math.min(1, Math.max(0, (e.clientY - rect.top) / rect.height))
  return {
    x: Math.round(ratioX * props.width),
    y: Math.round(ratioY * props.height),
  }
}

function onMouseDown(e: MouseEvent) {
  if (!props.editable) return
  dragging = true
  const coord = pointToCoord(e, e.currentTarget as HTMLElement)
  emit('hotspot-pick', coord)
}
function onMouseMove(e: MouseEvent) {
  if (!props.editable || !dragging) return
  const coord = pointToCoord(e, e.currentTarget as HTMLElement)
  emit('hotspot-drag', coord)
}
function onMouseUp() {
  if (!props.editable || !dragging) return
  dragging = false
  emit('hotspot-drag-end')
}
</script>

<template>
  <div
    class="ani-thumb"
    :class="{ editable }"
    :style="{ width: width + 'px', height: height + 'px' }"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @mouseleave="onMouseUp"
  >
    <img :src="player.currentImageUrl.value" :width="width" :height="height" alt="ani" />
    <template v-if="editable && hotspotPercent">
      <div class="crosshair-v" :style="{ left: hotspotPercent.left }" />
      <div class="crosshair-h" :style="{ top: hotspotPercent.top }" />
      <div class="dot" :style="{ left: hotspotPercent.left, top: hotspotPercent.top }" />
    </template>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ani-thumb {
  @apply relative overflow-hidden;
}
.ani-thumb.editable {
  @apply cursor-crosshair;
}
.ani-thumb img {
  @apply pointer-events-none select-none;
}
.crosshair-v {
  @apply pointer-events-none absolute top-0 bottom-0 w-px;
  background: rgba(255, 80, 80, 0.85);
}
.crosshair-h {
  @apply pointer-events-none absolute left-0 right-0 h-px;
  background: rgba(255, 80, 80, 0.85);
}
.dot {
  @apply pointer-events-none absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full;
  background: rgba(255, 80, 80, 0.95);
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.6);
}
</style>
