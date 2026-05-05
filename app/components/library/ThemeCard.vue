<script setup lang="ts">
/**
 * テーマカード。プレビュー (17 ロールマトリクス) + 名前/作者 + メタ + カバレッジ + アクション。
 * - `apply` クリックで親へ emit (Rust IPC は親で実行)
 * - `toggleFavorite` でスター切替
 */
import { computed } from 'vue'
import type { ThemeCardData } from '~/types/theme'
// UiIcon / CursorMatrix は Nuxt の自動インポートで解決される

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  apply: [id: string]
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

const coveragePct = computed(() =>
  Math.round((props.theme.includedRoles.length / 17) * 100),
)

const displayDate = computed(() => {
  // ISO8601 が来たら先頭 10 文字 (YYYY-MM-DD) にトリム
  const d = props.theme.date
  return d.length > 10 ? d.slice(0, 10) : d
})
</script>

<template>
  <div :class="['card', { active: theme.isActive }]">
    <div class="card-preview">
      <div v-if="theme.isActive" class="card-active-tag">
        <span class="pulse" />ACTIVE
      </div>
      <CursorMatrix :included="theme.includedRoles" />
    </div>
    <div class="card-body">
      <div class="card-row">
        <div>
          <div class="card-name">{{ theme.name }}</div>
          <div class="card-author">@{{ theme.author ?? 'unknown' }}</div>
        </div>
        <button
          :class="['star', { on: theme.isFavorite }]"
          :aria-label="theme.isFavorite ? 'お気に入り解除' : 'お気に入りに追加'"
          @click="emit('toggleFavorite', theme.id)"
        >
          <UiIcon :name="theme.isFavorite ? 'Star' : 'StarO'" :size="13" />
        </button>
      </div>
      <div class="meta-row">
        <span class="m"><b>v</b>{{ theme.version }}</span>
        <span class="m">{{ displayDate }}</span>
        <span class="m">×{{ theme.applyCount }}</span>
      </div>
      <div class="coverage">
        <div class="bar"><i :style="{ width: coveragePct + '%' }" /></div>
        <span class="num">{{ theme.includedRoles.length }}/17</span>
      </div>
      <div class="card-actions">
        <button
          v-if="theme.isActive"
          class="btn"
          disabled
          style="opacity: 0.6; cursor: default;"
        >
          <UiIcon name="Check" :size="13" />適用中
        </button>
        <button v-else class="btn primary" @click="emit('apply', theme.id)">
          適用
        </button>
        <button class="btn icon" aria-label="詳細" @click="emit('showDetails', theme.id)">
          <UiIcon name="ChevD" :size="13" />
        </button>
      </div>
    </div>
  </div>
</template>
