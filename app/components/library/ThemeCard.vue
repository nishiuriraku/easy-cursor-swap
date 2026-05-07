<script setup lang="ts">
/**
 * テーマカード。プレビュー (17 ロールマトリクス) + 名前/作者 + メタ + カバレッジ + アクション。
 * - `apply` クリックで親へ emit (Rust IPC は親で実行)
 * - `toggleFavorite` でスター切替
 * - マウント時に Rust から実カーソル PNG を取得し、マトリクスに反映する
 */
import { computed, onMounted, ref, watch } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'
import { useThemePreviews } from '~/composables/useThemePreviews'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  apply: [id: string]
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

/** カード本体のクリック/Enter で「適用」と同じ動作。子の <button> は stopPropagation で防御。 */
function onCardActivate(e: Event) {
  if (props.theme.isActive) return
  // 内側のボタン上ならスキップ (個別ハンドラに委譲)
  const target = e.target as HTMLElement | null
  if (target?.closest('button, a, input')) return
  emit('apply', props.theme.id)
}

function onCardKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    onCardActivate(e)
  }
}

const coveragePct = computed(() =>
  Math.round((props.theme.includedRoles.length / 17) * 100),
)

const displayDate = computed(() => {
  // ISO8601 が来たら先頭 10 文字 (YYYY-MM-DD) にトリム
  const d = props.theme.date
  return d.length > 10 ? d.slice(0, 10) : d
})

// 実カーソル画像のプレビュー (キャッシュ越しに取得)
const previewMap = ref<Record<string, string> | null>(null)
const { getMap } = useThemePreviews()

async function fetchPreview() {
  if (!props.theme.id) return
  const map = await getMap(props.theme.id)
  previewMap.value = map
}

onMounted(fetchPreview)
watch(() => props.theme.id, fetchPreview)
</script>

<template>
  <article
    :class="['card', { active: theme.isActive, interactive: !theme.isActive }]"
    :aria-label="theme.name"
    :tabindex="theme.isActive ? -1 : 0"
    role="button"
    @dblclick="onCardActivate"
    @keydown="onCardKeydown"
  >
    <div class="card-preview">
      <div v-if="theme.isActive" class="card-active-tag">
        <span class="pulse" aria-hidden="true" />{{ t('library.activeTag') }}
      </div>
      <CursorMatrix
        :included="theme.includedRoles"
        :preview-map="previewMap"
        :aria-label="t('library.coverage', { filled: theme.includedRoles.length })"
      />
    </div>
    <div class="card-body">
      <div class="card-row">
        <div>
          <div class="card-name">{{ theme.name }}</div>
          <div class="card-author">@{{ theme.author ?? 'unknown' }}</div>
        </div>
        <button
          :class="['star', { on: theme.isFavorite }]"
          :aria-label="theme.isFavorite
            ? t('library.filterFavorites') + 'から削除'
            : t('library.filterFavorites') + 'に追加'"
          :aria-pressed="theme.isFavorite"
          @click="emit('toggleFavorite', theme.id)"
        >
          <UiIcon :name="theme.isFavorite ? 'Star' : 'StarO'" :size="13" aria-hidden="true" />
        </button>
      </div>
      <div class="meta-row" aria-label="`${theme.name} v${theme.version}, ${displayDate}`">
        <span class="m" aria-hidden="true"><b>v</b>{{ theme.version }}</span>
        <span class="m" aria-hidden="true">{{ displayDate }}</span>
        <span class="m" aria-hidden="true">×{{ theme.applyCount }}</span>
      </div>
      <div class="coverage" aria-label="カバレッジ {{ theme.includedRoles.length }}/17">
        <div class="bar" aria-hidden="true"><i :style="{ width: coveragePct + '%' }" /></div>
        <span class="num" aria-hidden="true">{{ theme.includedRoles.length }}/17</span>
      </div>
      <div class="card-actions">
        <button
          v-if="theme.isActive"
          class="btn"
          disabled
          :aria-label="`${theme.name} — ${t('common.apply')}済み`"
          style="opacity: 0.6; cursor: default;"
        >
          <UiIcon name="Check" :size="13" aria-hidden="true" />{{ t('common.apply') }}済み
        </button>
        <button
          v-else
          class="btn primary"
          :aria-label="`${theme.name} を${t('common.apply')}`"
          @click="emit('apply', theme.id)"
        >
          {{ t('common.apply') }}
        </button>
        <button
          class="btn icon"
          :aria-label="`${theme.name} の詳細`"
          @click="emit('showDetails', theme.id)"
        >
          <UiIcon name="ChevD" :size="13" aria-hidden="true" />
        </button>
      </div>
    </div>
  </article>
</template>
