<script setup lang="ts">
/**
 * 一括インポートプレビューモーダル内の 1 ロール行。
 * - 画像サムネ + 信頼度バッジ + 衝突アイコン + 採用/スキップトグル
 */
import { computed } from 'vue'
import { useI18n } from '~/composables/useI18n'

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
}
const props = defineProps<Props>()
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
      <img v-if="previewUrl" :src="previewUrl" :alt="roleId" />
      <span v-else class="dim">{{ t('bulkImport.notProvided') }}</span>
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
.bi-row {
  display: grid;
  grid-template-columns: 140px 56px 1fr 80px 32px;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}
.bi-row.skip {
  opacity: 0.5;
}
.bi-row.conflict {
  background: rgba(255, 191, 0, 0.04);
}

.role-id {
  font-family: var(--font-mono);
  font-weight: 500;
}
.req {
  color: var(--accent);
  margin: 0 4px;
}
.role-label {
  color: var(--fg-mute);
  margin-left: 6px;
}
.thumb-cell img {
  width: 48px;
  height: 48px;
  object-fit: contain;
  image-rendering: pixelated;
}
.dim {
  color: var(--fg-dim);
  font-size: 11px;
}
.src {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--fg-dim);
}
.size {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--fg-mute);
  margin-left: 6px;
}
.confidence {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--accent);
}
.warn {
  color: var(--amber);
  margin-left: 4px;
}
</style>
