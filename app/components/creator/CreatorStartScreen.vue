<script setup lang="ts">
/**
 * Creator モードの初期画面 (design/empty-states.jsx::CreatorStart の Vue 版)
 *
 * ヒーローブロック + 3 CTA (新規 / .cursorpack インポート / 既存複製) +
 * キーボードショートカット表示 + 最近のドラフト一覧。
 *
 * ドラフト一覧は将来の `get_drafts` IPC を想定し、現状は空配列で渡されたら
 * セクションごと非表示にするフォールバック設計。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

defineProps<{
  /** 最近編集中のドラフト一覧。空配列なら "RECENT DRAFTS" セクションを隠す。 */
  recentDrafts?: Array<{
    id: string
    name: string
    modified: string
    roleCount: number
    isDraft: boolean
  }>
}>()

const emit = defineEmits<{
  /** ヒーローの「新規作成」CTA。空のテーマで Creator モードに入る。 */
  startNew: []
  /** 既存テーマ複製。親で Library 選択モーダルを開いてから取込フローに流す。 */
  duplicateExisting: []
  /** 最近のドラフトを開く。 */
  openDraft: [id: string]
}>()
</script>

<template>
  <div class="es-stage">
    <div class="es-bg" />
    <div class="es-creator-hero">
      <div class="es-mark">
        <CursorIcon role="Arrow" :size="48" style="color: var(--accent)" />
      </div>
      <div class="es-eyebrow">CREATOR · v1.0</div>
      <h1>{{ t('creatorStart.title') }}</h1>
      <p>{{ t('creatorStart.description') }}</p>

      <div class="es-cta-row">
        <button class="btn primary es-cta-primary" @click="emit('startNew')">
          <UiIcon name="Plus" :size="14" />
          {{ t('creatorStart.btnNew') }}
        </button>
        <button class="btn ghost" @click="emit('duplicateExisting')">
          <UiIcon name="Brush" :size="13" />{{ t('creatorStart.btnDuplicate') }}
        </button>
      </div>

      <div class="es-shortcuts">
        <span class="es-kb">
          <span class="kbd">Ctrl</span><span class="kbd">N</span
          ><span>{{ t('creatorStart.kbdNew') }}</span>
        </span>
        <span class="es-kb">
          <span class="kbd">Ctrl</span><span class="kbd">O</span
          ><span>{{ t('creatorStart.kbdOpen') }}</span>
        </span>
      </div>
    </div>

    <div v-if="recentDrafts && recentDrafts.length > 0" class="es-recent">
      <div class="es-recent-h">
        <span class="td-pane-k">RECENT DRAFTS</span>
        <span class="td-pane-link">all →</span>
      </div>
      <div class="es-recent-list">
        <button
          v-for="d in recentDrafts"
          :key="d.id"
          class="es-recent-item"
          @click="emit('openDraft', d.id)"
        >
          <div class="es-recent-thumb">
            <UiIcon :name="d.isDraft ? 'Brush' : 'Library'" :size="14" />
          </div>
          <div class="es-recent-meta">
            <div class="es-recent-name">
              {{ d.name }}
              <span v-if="d.isDraft" class="es-draft">DRAFT</span>
            </div>
            <div class="es-recent-sub">{{ d.modified }} · {{ d.roleCount }}/17 役割</div>
          </div>
          <UiIcon
            name="ChevD"
            :size="12"
            style="color: var(--fg-mute); transform: rotate(-90deg)"
          />
        </button>
      </div>
    </div>
  </div>
</template>
