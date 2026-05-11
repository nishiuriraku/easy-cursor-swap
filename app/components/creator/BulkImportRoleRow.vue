<script setup lang="ts">
/**
 * 一括インポートプレビューモーダル内の 1 ロール行。
 * - 画像サムネ + 信頼度バッジ + 衝突アイコン + 採用/スキップトグル
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'
import AniThumb from './AniThumb.vue'

const { t } = useI18n()

interface Props {
  roleId: string
  roleLabel: string
  required: boolean
  /** 採用候補の画像 (Object URL or null) */
  previewUrl: string | null
  sourceFile: string | null
  nativeSize: number | null
  /** 信頼度 0-1。null なら未マッチ。 */
  confidence: number | null
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  decision: 'apply' | 'skip'
  aniData?: {
    framePngs: number[][]
    sequence: number[]
    perStepDurationsMs: number[]
    isLegacyRawDib: boolean
  } | null
}
const props = withDefaults(defineProps<Props>(), { aniData: null })
const emit = defineEmits<{ (e: 'toggle', value: 'apply' | 'skip'): void }>()

const confidenceLabel = computed(() => {
  if (props.confidence === null) return ''
  return `${Math.round(props.confidence * 100)}%`
})
const conflictTitle = computed(() => {
  if (props.conflict === 'overwrite-existing') return t('bulkImport.conflictOverwrite')
  if (props.conflict === 'collision-with-other-pending') return t('bulkImport.conflictCollision')
  return ''
})
</script>

<template>
  <div class="bi-row" :class="{ skip: decision === 'skip', conflict: conflict !== 'none' }">
    <div class="role-cell">
      <span class="role-id">{{ roleId }}</span>
      <span v-if="required" class="req">★</span>
      <span class="role-label">{{ roleLabel }}</span>
    </div>
    <div class="thumb-cell">
      <AniThumb
        v-if="aniData"
        :frame-pngs="aniData.framePngs.map((b) => new Uint8Array(b))"
        :sequence="aniData.sequence"
        :durations="aniData.perStepDurationsMs"
        :width="32"
        :height="32"
      />
      <img v-else-if="previewUrl" :src="previewUrl" :alt="roleId" />
      <span v-else class="dim">{{ t('bulkImport.notProvided') }}</span>
      <span v-if="aniData?.isLegacyRawDib" class="legacy-chip">{{
        t('bulkImport.legacyAniChip')
      }}</span>
    </div>
    <div class="meta-cell">
      <span v-if="sourceFile" class="src">{{ sourceFile }}</span>
      <span v-if="nativeSize" class="size">{{ nativeSize }}px</span>
    </div>
    <div class="badge-cell">
      <span v-if="confidence !== null" class="confidence">{{ confidenceLabel }}</span>
      <span v-if="conflict !== 'none'" class="warn" :title="conflictTitle">⚠</span>
    </div>
    <div class="action-cell">
      <input
        type="checkbox"
        :checked="decision === 'apply'"
        :disabled="!previewUrl"
        @change="emit('toggle', ($event.target as HTMLInputElement).checked ? 'apply' : 'skip')"
      />
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.bi-row {
  @apply grid grid-cols-[140px_56px_1fr_80px_32px] items-center gap-2 border-b border-line px-2 py-1.5 text-[12px];
}
.bi-row.skip {
  @apply opacity-50;
}
.bi-row.conflict {
  background: rgba(255, 191, 0, 0.04);
}

.role-id {
  @apply font-mono font-medium;
}
.req {
  @apply mx-1 my-0 text-accent;
}
.role-label {
  @apply ml-1.5 text-fg-mute;
}
.thumb-cell img {
  @apply size-12 object-contain;
  image-rendering: pixelated;
}
.dim {
  @apply text-[11px] text-fg-dim;
}
.src {
  @apply font-mono text-[11px] text-fg-dim;
}
.size {
  @apply ml-1.5 font-mono text-[10px] text-fg-mute;
}
.confidence {
  @apply font-mono text-[10px] text-accent;
}
.warn {
  @apply ml-1 text-amber;
}
.legacy-chip {
  @apply ml-1 rounded-sm bg-[rgba(240,180,40,0.2)] px-1 text-[10px] uppercase text-[#f0b428];
}
</style>
