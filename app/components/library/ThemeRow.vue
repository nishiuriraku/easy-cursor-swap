<script setup lang="ts">
/**
 * テーマライブラリの一覧表示行 (List view の 1 行)。
 *
 * design/library-list.jsx + design/styles-list.css を Vue/Nuxt 4 へ移植。
 * グリッド表示の `ThemeCard.vue` と同じ emit を共有し、Library ページで
 * 表示モード切替 (`viewMode === 'list'`) に応じて差し替えられる。
 *
 * 共通プレゼンテーション状態 (preview fetch / isSystem / isMarketplace /
 * displayDate / onActivate / onKeydown / onFavorite) は `useThemeCardState`
 * に集約。
 *
 * 6 列構成 (Phase 13-A で coverage バーを撤去、preview を Arrow 1 個に。
 *  2026-05-13 で署名列 sig を撤去。
 *  2026-05-14 で Arrow セルを実 PNG プレビュー表示へ昇格):
 *  fav | preview (Arrow 1 個) | name+tags | ver | date | size
 */
import { computed, toRef } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'
import { useThemeCardState } from '~/composables/useThemeCardState'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

const { previewMap, isSystem, isMarketplace, displayDate, onActivate, onKeydown, onFavorite } =
  useThemeCardState(toRef(props, 'theme'), {
    showDetails: (id) => emit('showDetails', id),
    toggleFavorite: (id) => emit('toggleFavorite', id),
  })

/** 行ビューでは Arrow 1 個だけアイコン表示する。included かどうかで描画切替。 */
const hasArrow = computed(() => props.theme.includedRoles.includes('Arrow'))
const arrowPreviewUrl = computed(() => previewMap.value?.['Arrow'] ?? null)

/** バイト数 → 「2.1 MB」「412 KB」表示。
 *  - undefined または 0 は「—」(Windows システムスキーム想定)
 *  - 1 KB 未満は B、1 MB 未満は KB、それ以上は MB。 */
const displaySize = computed(() => {
  const b = props.theme.sizeBytes
  if (b == null || b === 0) return '—'
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${(b / (1024 * 1024)).toFixed(1)} MB`
})
</script>

<template>
  <div
    :class="['lib-row', { active: theme.isActive }]"
    role="row"
    tabindex="0"
    :aria-label="t('library.detailAria', { name: theme.name })"
    @click="onActivate"
    @keydown="onKeydown"
  >
    <!-- お気に入り -->
    <div class="lt-col lt-fav" role="cell">
      <button
        v-if="!isSystem"
        :class="['star', { on: theme.isFavorite }]"
        :aria-label="theme.isFavorite ? t('library.favRemove') : t('library.favAdd')"
        :aria-pressed="theme.isFavorite"
        @click="onFavorite"
      >
        <UiIcon :name="theme.isFavorite ? 'Star' : 'StarO'" :size="12" aria-hidden="true" />
      </button>
    </div>

    <!-- プレビュー (Arrow 1 アイコン) -->
    <div class="lt-col lt-preview" role="cell">
      <div
        class="lt-mini"
        :aria-label="t('library.coverage', { filled: theme.includedRoles.length })"
      >
        <div :class="['lt-cell', { empty: !hasArrow }]">
          <img
            v-if="hasArrow && arrowPreviewUrl"
            :src="arrowPreviewUrl"
            alt=""
            class="lt-cell-img"
          />
          <CursorIcon v-else-if="hasArrow" role="Arrow" :size="14" />
        </div>
      </div>
    </div>

    <!-- 名前 / 作者 / Active / Tags -->
    <div class="lt-col lt-name" role="cell">
      <div class="lt-name-wrap">
        <div class="lt-name-row">
          <span class="lt-name-text">{{ theme.name }}</span>
          <span v-if="theme.isActive" class="lt-active-pill">
            <span class="pulse" aria-hidden="true" />{{ t('library.activeTag') }}
          </span>
          <span
            v-if="isMarketplace"
            class="lt-tag lt-tag-marketplace"
            :aria-label="t('library.sourceTagMarketplaceAria')"
            >MARKETPLACE</span
          >
          <span v-for="tag in theme.tags ?? []" :key="tag" class="lt-tag">{{ tag }}</span>
        </div>
        <div class="lt-author">@{{ theme.author ?? 'unknown' }}</div>
      </div>
    </div>

    <!-- バージョン / 日付 / サイズ -->
    <div class="lt-col lt-ver" role="cell">
      <span class="lt-mono">v{{ theme.version }}</span>
    </div>
    <div class="lt-col lt-date" role="cell">
      <span class="lt-mono">{{ displayDate }}</span>
    </div>
    <div class="lt-col lt-size" role="cell">
      <span class="lt-mono">{{ displaySize }}</span>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.lt-tag-marketplace {
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.16), rgba(124, 242, 212, 0.04));
  color: var(--accent, #7cf2d4);
  border-color: rgba(124, 242, 212, 0.35);
}
</style>
