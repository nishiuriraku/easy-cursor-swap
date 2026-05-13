<script setup lang="ts">
/**
 * 一括インポートプレビューモーダル内の 1 ロール行。
 *
 * - 割当済: サムネ + 衝突アイコン + 解除ボタン
 * - 空き:   点線枠 + 「未割当」ラベル (未マッチプールからドロップダウンで割当)
 *
 * 確信度パーセント表示と採用/スキップトグルは廃止。「適用しないファイル」は
 * 解除ボタンで未マッチプールへ戻す UX に統一した (情報を失わず、操作が 1 種類で済む)。
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
  conflict: 'none' | 'overwrite-existing' | 'collision-with-other-pending'
  aniData?: {
    framePngs: number[][]
    sequence: number[]
    perStepDurationsMs: number[]
    isLegacyRawDib: boolean
  } | null
  /**
   * 親側で 1 度だけ Uint8Array 化した frames 配列。テンプレートに
   * `new Uint8Array(...)` を書くと Vue 3 のグローバル白リストに
   * Uint8Array が含まれないため warn が出るので script setup 内で済ませて渡す。
   */
  aniFramesU8?: readonly Uint8Array[] | null
}
const props = withDefaults(defineProps<Props>(), { aniData: null, aniFramesU8: null })
const emit = defineEmits<{
  /** 解除ボタンクリック: 割当済アセットを未マッチプールに戻す */
  (e: 'unassign'): void
}>()

const isEmpty = computed(() => props.previewUrl === null && props.aniData === null)

const conflictTitle = computed(() => {
  if (props.conflict === 'overwrite-existing') return t('bulkImport.conflictOverwrite')
  if (props.conflict === 'collision-with-other-pending') return t('bulkImport.conflictCollision')
  return ''
})
</script>

<template>
  <div class="bi-row" :class="{ conflict: conflict !== 'none', empty: isEmpty }">
    <div class="role-cell">
      <div class="role-line">
        <CursorIcon :role="roleId" :size="14" />
        <span class="role-label">{{ roleLabel }}</span>
        <span v-if="required" class="req">★</span>
      </div>
      <span class="role-id">{{ roleId }}</span>
    </div>
    <div class="thumb-cell">
      <AniThumb
        v-if="aniData && aniFramesU8"
        :frame-pngs="aniFramesU8"
        :sequence="aniData.sequence"
        :durations="aniData.perStepDurationsMs"
        :width="32"
        :height="32"
      />
      <img v-else-if="previewUrl" :src="previewUrl" :alt="roleId" />
      <span v-else class="dim">{{ t('bulkImport.emptyRole') }}</span>
      <span v-if="aniData?.isLegacyRawDib" class="legacy-chip">{{
        t('bulkImport.legacyAniChip')
      }}</span>
    </div>
    <div class="meta-cell">
      <span v-if="sourceFile" class="src">{{ sourceFile }}</span>
      <span v-if="nativeSize" class="size">{{ nativeSize }}px</span>
    </div>
    <div class="badge-cell">
      <span v-if="conflict !== 'none'" class="warn" :title="conflictTitle">⚠</span>
    </div>
    <div class="action-cell">
      <button
        v-if="!isEmpty"
        type="button"
        class="btn ghost unassign-btn"
        :title="t('bulkImport.unassign')"
        @click="emit('unassign')"
      >
        ✕
      </button>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.bi-row {
  @apply grid grid-cols-[160px_56px_1fr_24px_32px] items-center gap-2 border-b border-line px-2 py-1.5 text-[12px];
}
.bi-row.conflict {
  background: rgba(255, 191, 0, 0.04);
}
.bi-row.empty .thumb-cell {
  @apply rounded border border-dashed border-line/70;
}

.role-cell {
  @apply flex min-w-0 flex-col gap-0.5;
}
.role-line {
  @apply flex items-center gap-1;
}
.role-label {
  @apply truncate text-[12px] font-medium text-fg;
}
.req {
  @apply text-accent;
}
.role-id {
  @apply font-mono text-[10px] text-fg-mute;
}
.thumb-cell {
  @apply flex h-12 items-center justify-center;
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
.warn {
  @apply ml-1 text-amber;
}
.legacy-chip {
  @apply ml-1 rounded-sm bg-[rgba(240,180,40,0.2)] px-1 text-[10px] uppercase text-[#f0b428];
}
.action-cell {
  @apply flex items-center justify-end;
}
.unassign-btn {
  @apply px-1.5 py-0 text-[12px] leading-none;
}
</style>
