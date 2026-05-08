<script setup lang="ts">
/**
 * テーマライブラリの一覧表示行 (List view の 1 行)。
 *
 * design/library-list.jsx + design/styles-list.css を Vue/Nuxt 4 へ移植。
 * グリッド表示の `ThemeCard.vue` と同じ emit を共有し、Library ページで
 * 表示モード切替 (`viewMode === 'list'`) に応じて差し替えられる。
 *
 * 9 列構成:
 *  fav | preview | name+tags | coverage bar | ver | date | size | sig | actions
 *
 * 狭幅では `.lt-mini` が media query で縦 1 セルに圧縮され、Arrow のみ残る
 * (Q3 で「非表示ではなく Arrow のみ」と決定)。
 */
import { computed } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { CURSOR_ROLES } from '~/components/icons/CursorIcons'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  theme: ThemeCardData
}>()

const emit = defineEmits<{
  apply: [id: string]
  toggleFavorite: [id: string]
  showDetails: [id: string]
}>()

/** プレビュー列で表示する役割。Source of Truth は CURSOR_ROLES の先頭 8 件
 *  (= Arrow / Help / AppStarting / Wait / Crosshair / IBeam / NWPen / No)。 */
const previewRoles = CURSOR_ROLES.slice(0, 8)

const coveragePct = computed(() =>
  Math.round((props.theme.includedRoles.length / 17) * 100),
)

const isSystem = computed(() => props.theme.kind === 'system')

/** ISO8601 / YYYY-MM-DD どちらでも先頭 10 文字に切り詰める。
 *  Windows システムスキームは `date` が空文字で来るので「—」で代替。 */
const displayDate = computed(() => {
  const d = props.theme.date
  if (!d) return '—'
  return d.length > 10 ? d.slice(0, 10) : d
})

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

const isSigned = computed(() => props.theme.signed !== false)

/** Active / 非 Active で別ボタン。Active ボタンは disabled だが clickable に
 *  しないため `aria-disabled` も付与している。 */
const applyLabel = computed(() =>
  props.theme.isActive ? t('library.appliedActive') : t('common.apply'),
)

function onApply() {
  if (props.theme.isActive) return
  emit('apply', props.theme.id)
}

function onFav() {
  if (isSystem.value) return
  emit('toggleFavorite', props.theme.id)
}

function onDetail() {
  emit('showDetails', props.theme.id)
}
</script>

<template>
  <div :class="['lib-row', { active: theme.isActive }]" role="row">
    <!-- お気に入り -->
    <div class="lt-col lt-fav" role="cell">
      <button
        v-if="!isSystem"
        :class="['star', { on: theme.isFavorite }]"
        :aria-label="theme.isFavorite
          ? t('library.filterFavorites') + 'から削除'
          : t('library.filterFavorites') + 'に追加'"
        :aria-pressed="theme.isFavorite"
        @click="onFav"
      >
        <UiIcon :name="theme.isFavorite ? 'Star' : 'StarO'" :size="12" aria-hidden="true" />
      </button>
    </div>

    <!-- プレビュー (8 セル → 狭幅では Arrow のみ) -->
    <div class="lt-col lt-preview" role="cell">
      <div class="lt-mini" :aria-label="t('library.coverage', { filled: theme.includedRoles.length })">
        <div
          v-for="role in previewRoles"
          :key="role.id"
          :class="['lt-cell', { empty: !theme.includedRoles.includes(role.id) }]"
          :title="role.jp"
        >
          <CursorIcon
            v-if="theme.includedRoles.includes(role.id)"
            :role="role.id"
            :size="11"
          />
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
            v-for="tag in theme.tags ?? []"
            :key="tag"
            class="lt-tag"
          >{{ tag }}</span>
        </div>
        <div class="lt-author">@{{ theme.author ?? 'unknown' }}</div>
      </div>
    </div>

    <!-- カバレッジ -->
    <div class="lt-col lt-cov" role="cell">
      <div class="lt-cov-wrap">
        <div class="lt-cov-bar" aria-hidden="true">
          <i :style="{ width: coveragePct + '%' }" />
        </div>
        <span class="lt-cov-num">
          {{ theme.includedRoles.length }}<span class="lt-cov-tot">/17</span>
        </span>
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

    <!-- 署名 -->
    <div class="lt-col lt-sig" role="cell">
      <span v-if="isSigned" class="lt-sig-ok">
        <UiIcon name="Shield" :size="11" aria-hidden="true" />{{ t('library.sigSigned') }}
      </span>
      <span v-else class="lt-sig-warn">
        <UiIcon name="Alert" :size="11" aria-hidden="true" />{{ t('library.sigUnsigned') }}
      </span>
    </div>

    <!-- アクション -->
    <div class="lt-col lt-act" role="cell">
      <button
        v-if="theme.isActive"
        class="btn"
        disabled
        aria-disabled="true"
        :aria-label="`${theme.name} — ${applyLabel}`"
        style="opacity: 0.6; cursor: default; height: 28px;"
      >
        <UiIcon name="Check" :size="12" aria-hidden="true" />{{ applyLabel }}
      </button>
      <button
        v-else
        class="btn primary"
        :aria-label="`${theme.name} を${t('common.apply')}`"
        style="height: 28px;"
        @click="onApply"
      >
        {{ applyLabel }}
      </button>
      <button
        class="btn icon"
        :aria-label="`${theme.name} の詳細を開く`"
        style="height: 28px; width: 28px;"
        @click="onDetail"
      >
        <UiIcon name="ChevD" :size="12" aria-hidden="true" />
      </button>
    </div>
  </div>
</template>
