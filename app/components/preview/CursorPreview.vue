<script setup lang="ts">
/**
 * カーソル画像 + ホットスポット表示の単一コンポーネント。
 *
 * 3 つの旧描画箇所 (creator `.bigpreview` / drawer `.td-rp-stage` / AniThumb.editable) を
 * 1 つに統合した結果物。座標系は「コンテナの displayPct% を画像が占有、dot は
 * `calc(50% + (h - 0.5) × displayPct%)`」に統一されている。
 *
 * - 非編集 (既定): 画像 + dot のみ。
 * - 編集 (`editable=true`): pointer ドラッグ / keyboard nudge / crosshair / focus ring を追加。
 */
import { computed, ref, toRef } from 'vue'
import type { Hotspot } from '~/composables/useCreatorAssets'
import { useHotspotInteraction, hotspotDotStyle } from '~/composables/useHotspotInteraction'
import AniThumb from '~/components/creator/AniThumb.vue'

export type CursorPreviewAsset =
  | {
      kind: 'ani'
      framePngs: readonly Uint8Array[]
      sequence: readonly number[]
      durations: readonly number[]
      /** ANI フレームの内在ピクセル幅 (`<img>` の width/height 属性に渡す)。 */
      nativeSize: number
    }
  | { kind: 'static'; url: string; alt?: string }
  | { kind: 'empty' }

interface Props {
  asset: CursorPreviewAsset
  hotspot: Hotspot
  /** 'empty' fallback で CursorIcon に渡すロール ID。 */
  roleId?: string
  /** コンテナに対する画像表示サイズ (%)。dot 位置式の基準。既定 100。 */
  displayPct?: number
  /** pointer / keyboard ハンドラ + crosshair + focus ring + dragging を有効化。 */
  editable?: boolean
  /** keyboard nudge の step (= 1px / referencePx の ratio) に使う参照寸法。 */
  referencePx?: number
  /** ホットスポット dot を非表示。drawer 等で `nativeSize` 不明時のフォールバック用途。 */
  hideDot?: boolean
  /** 'empty' fallback の CursorIcon サイズ (px)。既定 displayPct を size から逆算しない簡易値。 */
  fallbackIconSize?: number
}

const props = withDefaults(defineProps<Props>(), {
  roleId: undefined,
  displayPct: 100,
  editable: false,
  referencePx: 0,
  hideDot: false,
  fallbackIconSize: 64,
})

const emit = defineEmits<{
  (e: 'update:hotspot', value: Hotspot): void
}>()

const rootEl = ref<HTMLElement | null>(null)

const interaction = useHotspotInteraction({
  el: rootEl,
  hotspot: toRef(props, 'hotspot'),
  displayPct: toRef(props, 'displayPct'),
  referencePx: toRef(props, 'referencePx'),
  onUpdate: (next) => emit('update:hotspot', next),
})

const dotStyle = computed(() => hotspotDotStyle(props.hotspot, props.displayPct))

const imageStyle = computed(() => ({
  width: `${props.displayPct}%`,
  height: `${props.displayPct}%`,
}))
</script>

<template>
  <div
    ref="rootEl"
    :class="['cp-root', { editable, dragging: interaction.dragging.value }]"
    :tabindex="editable ? 0 : -1"
    @pointerdown="editable ? interaction.onPointerDown($event) : undefined"
    @pointermove="editable ? interaction.onPointerMove($event) : undefined"
    @pointerup="editable ? interaction.onPointerUp($event) : undefined"
    @pointercancel="editable ? interaction.onPointerUp($event) : undefined"
    @keydown="editable ? interaction.onKeydown($event) : undefined"
  >
    <template v-if="editable">
      <div class="cp-crosshair-h" />
      <div class="cp-crosshair-v" />
    </template>

    <!-- ani↔ani 切替時に AniThumb を再マウントさせ、useAniPlayer 内の frameUrls を再初期化する -->
    <AniThumb
      v-if="asset.kind === 'ani'"
      :key="asset.framePngs"
      :frame-pngs="asset.framePngs"
      :sequence="asset.sequence"
      :durations="asset.durations"
      :width="asset.nativeSize"
      :height="asset.nativeSize"
      fit
      class="cp-image"
      :style="imageStyle"
    />
    <img
      v-else-if="asset.kind === 'static'"
      :src="asset.url"
      :alt="asset.alt ?? ''"
      draggable="false"
      class="cp-image"
      :style="imageStyle"
    />
    <CursorIcon v-else-if="roleId" :role="roleId" :size="fallbackIconSize" class="cp-fallback" />

    <div v-if="!hideDot" class="cp-hot" :style="dotStyle" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.cp-root {
  @apply relative grid place-items-center;
  touch-action: none;
  user-select: none;
}
.cp-root.editable {
  @apply cursor-crosshair;
}
.cp-root.editable:focus {
  outline: 2px solid var(--accent-line);
  outline-offset: -2px;
}
.cp-root.editable:focus:not(:focus-visible) {
  outline: none;
}

.cp-image {
  @apply pointer-events-none select-none;
  image-rendering: pixelated;
  object-fit: contain;
}
.cp-fallback {
  @apply pointer-events-none;
  color: var(--fg);
}

.cp-hot {
  @apply pointer-events-none absolute size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full;
  border: 1.5px solid var(--accent);
  background: rgba(124, 242, 212, 0.2);
  box-shadow: 0 0 12px var(--accent);
  transition:
    box-shadow 120ms ease,
    background 120ms ease;
}
.cp-root.dragging .cp-hot {
  border-color: var(--accent-hi);
  background: rgba(124, 242, 212, 0.4);
  box-shadow:
    0 0 16px var(--accent),
    0 0 0 6px rgba(124, 242, 212, 0.18);
}

.cp-crosshair-h,
.cp-crosshair-v {
  @apply pointer-events-none absolute;
  background: var(--accent-line);
}
.cp-crosshair-h {
  @apply inset-x-0 h-px;
  top: 50%;
}
.cp-crosshair-v {
  @apply inset-y-0 w-px;
  left: 50%;
}
</style>
