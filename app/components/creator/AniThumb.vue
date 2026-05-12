<script setup lang="ts">
/**
 * .ani をアニメーション再生する素の `<img>` ラッパー。
 *
 * - `fit=true` (`<CursorPreview>` から呼ばれる主用途): 親要素を 100% に占めて表示する。
 *   `image-rendering: pixelated` で粗くスケール。`width/height` props はフレームの内在ピクセル幅。
 * - `fit=false` (既定): props.width/height をそのまま CSS サイズに使う (ヘッダーアイコン等の用途)。
 *
 * ホットスポット表示・編集は `<CursorPreview>` 側で行うため、本コンポーネントは持たない。
 */
import { useAniPlayer } from '~/composables/useAniPlayer'

interface Props {
  framePngs: readonly Uint8Array[]
  sequence: readonly number[]
  durations: readonly number[]
  width: number
  height: number
  fit?: boolean
}
const props = withDefaults(defineProps<Props>(), {
  fit: false,
})

const player = useAniPlayer({
  framePngs: props.framePngs,
  sequence: props.sequence,
  perStepDurationsMs: props.durations,
})
</script>

<template>
  <div
    class="ani-thumb"
    :class="{ fit }"
    :style="fit ? undefined : { width: width + 'px', height: height + 'px' }"
  >
    <img :src="player.currentImageUrl.value" :width="width" :height="height" alt="ani" />
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ani-thumb {
  @apply relative overflow-hidden;
}
.ani-thumb.fit {
  width: 100%;
  height: 100%;
  aspect-ratio: 1 / 1;
}
.ani-thumb img {
  @apply pointer-events-none select-none;
}
.ani-thumb.fit img {
  width: 100%;
  height: 100%;
  image-rendering: pixelated;
}
</style>
