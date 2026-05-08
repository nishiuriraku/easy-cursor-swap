<script setup lang="ts">
/**
 * テーマ選択モーダル。
 * appearance.vue / 設定画面などで「テーマを変更」する場面で使用。
 *
 * - ライブラリのテーマ一覧 + 検索
 * - 選択中ハイライト + クリックで選択
 * - クリアボタン (null = 未指定)
 */
import { computed, ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

const props = defineProps<{
  /** 表示中のテーマ一覧 */
  themes: ThemeCardData[]
  /** 現在選択中の ID */
  modelValue: string | null
  /** モーダルタイトル */
  title?: string
  /** サブタイトル (どのスロット用かの文脈) */
  sub?: string
  /** モーダルアクセント色 (CSS) — Light=amber, Dark=mint 等 */
  accent?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [v: string | null]
  cancel: []
}>()

const query = ref('')

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return props.themes
  return props.themes.filter(
    (t) => t.name.toLowerCase().includes(q) || (t.author?.toLowerCase().includes(q) ?? false),
  )
})

function pick(id: string) {
  emit('update:modelValue', id)
  emit('cancel')
}

function clear() {
  emit('update:modelValue', null)
  emit('cancel')
}

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit('cancel')
}
</script>

<template>
  <div
    class="modal-page"
    role="dialog"
    aria-modal="true"
    aria-labelledby="picker-modal-title"
    @click="onBackdrop"
  >
    <div class="modal picker-modal" @click.stop>
      <div class="modal-head">
        <div
          class="modal-icon"
          aria-hidden="true"
          :style="
            accent ? { background: `${accent}1f`, borderColor: `${accent}59`, color: accent } : {}
          "
        >
          <UiIcon name="Library" :size="20" />
        </div>
        <div style="flex: 1; min-width: 0">
          <h2 id="picker-modal-title">{{ title ?? t('themePicker.titleDefault') }}</h2>
          <p v-if="sub">{{ sub }}</p>
        </div>
        <button class="btn icon" :aria-label="t('common.close')" @click="emit('cancel')">
          <UiIcon name="X" :size="11" />
        </button>
      </div>

      <div class="modal-body picker-body">
        <div class="search picker-search">
          <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
          <input
            v-model="query"
            :placeholder="t('library.searchPlaceholder')"
            :aria-label="t('common.search')"
          />
        </div>

        <div v-if="filtered.length === 0" class="picker-empty">
          <UiIcon name="Search" :size="32" />
          <p>{{ t('themePicker.notFound') }}</p>
        </div>

        <ul v-else class="picker-list">
          <li
            v-for="t in filtered"
            :key="t.id"
            :class="['picker-item', { selected: modelValue === t.id }]"
            tabindex="0"
            role="button"
            @click="pick(t.id)"
            @keydown.enter="pick(t.id)"
            @keydown.space.prevent="pick(t.id)"
          >
            <div class="pi-thumb">
              <CursorIcon role="Arrow" :size="20" />
            </div>
            <div class="pi-meta">
              <div class="pi-name">{{ t.name }}</div>
              <div class="pi-sub">
                @{{ t.author ?? 'unknown' }} · v{{ t.version }} · {{ t.includedRoles.length }}/17
              </div>
            </div>
            <UiIcon
              v-if="modelValue === t.id"
              name="Check"
              :size="14"
              style="color: var(--accent)"
            />
          </li>
        </ul>
      </div>

      <div class="modal-foot">
        <button class="btn ghost" @click="clear">{{ t('themePicker.clear') }}</button>
        <div class="actions">
          <button class="btn ghost" @click="emit('cancel')">{{ t('common.cancel') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.picker-modal {
  width: 480px;
}
.picker-body {
  padding: 16px 20px 8px;
}
.picker-search {
  margin-bottom: 12px;
  max-width: none;
}
.picker-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 32px 0;
  color: var(--fg-mute);
}
.picker-empty p {
  margin: 0;
  font-size: 13px;
}

.picker-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 360px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.picker-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid transparent;
  cursor: pointer;
  transition:
    background 0.12s,
    border-color 0.12s;
}
.picker-item:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: var(--line-hi);
}
.picker-item.selected {
  background: var(--accent-dim);
  border-color: var(--accent-line);
}

.pi-thumb {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.15), rgba(139, 125, 255, 0.15));
  border: 1px solid var(--line);
  color: var(--fg);
  flex-shrink: 0;
}

.pi-meta {
  flex: 1;
  min-width: 0;
}
.pi-name {
  font-family: var(--font-display);
  font-weight: 600;
  font-size: 13px;
  letter-spacing: -0.01em;
}
.pi-sub {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--fg-mute);
  margin-top: 2px;
}
</style>
