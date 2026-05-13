<script setup lang="ts">
/**
 * テーマ選択モーダル。
 * 設定画面などで「テーマを変更」する場面で使用。
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
  /**
   * フッターに「未指定にする」ボタンを表示するか。既定 true。
   * Creator 複製モーダルでは「未指定」が意味を成さないので false で渡す。
   */
  showClear?: boolean
  /**
   * フッターに「キャンセル」ボタンを表示するか。既定 true。
   * ヘッダ右上の ✕ ボタンと重複するため、不要な画面 (Creator 複製) では false。
   */
  showFooterCancel?: boolean
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

      <div v-if="(props.showClear ?? true) || (props.showFooterCancel ?? true)" class="modal-foot">
        <button v-if="props.showClear ?? true" class="btn ghost" @click="clear">
          {{ t('themePicker.clear') }}
        </button>
        <div class="actions">
          <button v-if="props.showFooterCancel ?? true" class="btn ghost" @click="emit('cancel')">
            {{ t('common.cancel') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.picker-modal {
  @apply w-[480px];
}
.picker-body {
  @apply px-5 pb-2 pt-4;
}
.picker-search {
  @apply mb-3 max-w-none;
}
.picker-empty {
  @apply flex flex-col items-center gap-3 py-8 text-fg-mute;
}
.picker-empty p {
  @apply m-0 text-[13px];
}

.picker-list {
  @apply m-0 flex max-h-[360px] flex-col gap-1 overflow-y-auto p-0;
  list-style: none;
}

.picker-item {
  @apply flex cursor-pointer items-center gap-3 rounded-[8px] border border-transparent px-3 py-2.5;
  transition:
    background 0.12s,
    border-color 0.12s;
}
.picker-item:hover {
  @apply bg-white/[0.04];
  border-color: var(--line-hi);
}
.picker-item.selected {
  @apply bg-accent-dim;
  border-color: var(--accent-line);
}

.pi-thumb {
  @apply grid size-9 shrink-0 place-items-center rounded-[8px] border border-line text-fg;
  background: linear-gradient(135deg, rgba(124, 242, 212, 0.15), rgba(139, 125, 255, 0.15));
}

.pi-meta {
  @apply min-w-0 flex-1;
}
.pi-name {
  @apply font-display text-[13px] font-semibold tracking-[-0.01em];
}
.pi-sub {
  @apply mt-0.5 font-mono text-[10.5px] text-fg-mute;
}
</style>
