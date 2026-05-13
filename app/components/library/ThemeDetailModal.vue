<script setup lang="ts">
/**
 * テーマ詳細モーダル
 *
 * カードのインライン展開だと開閉トグルが分かりにくく、複数同時展開で
 * グリッドが乱れる UX 問題があったため、シェブロン押下で中央オーバーレイ
 * のモーダルに切り替えた。
 *
 * - 背景クリック / Esc キーで閉じる
 * - スクロールロック (body に overflow: hidden を一時的に付与)
 * - Teleport で `<body>` 直下にレンダリングして z-index 戦争を回避
 */
import { computed, onBeforeUnmount, watch } from 'vue'
import type { ThemeCardData } from '~/types/theme'
import { useI18n } from '~/composables/useI18n'
import type { RolePreviewDetail } from '~/composables/useThemePreviews'

const { t } = useI18n()

const props = defineProps<{
  /** 開く対象のテーマ。null のときはモーダル非表示。 */
  theme: ThemeCardData | null
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon フォールバック。 */
  previewMap: Record<string, string> | null
  /** 役割名 → ホットスポット詳細。ホットスポットドット表示に使う。 */
  previewDetails?: Record<string, RolePreviewDetail> | null
}>()

const emit = defineEmits<{
  close: []
  apply: [id: string]
  edit: [id: string]
  duplicate: [id: string]
  exportPack: [id: string]
  delete: [id: string]
}>()

const isOpen = computed(() => props.theme !== null)

function close() {
  emit('close')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
  }
}

// モーダル開閉に追従して body スクロールをロック / 解除する。
// `<Teleport>` 配下なので Vue の lifecycle と body の状態が乖離しないよう
// watch で同期する。
let prevOverflow: string | null = null
watch(isOpen, (open) => {
  if (typeof document === 'undefined') return
  if (open) {
    prevOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    document.addEventListener('keydown', onKeydown)
  } else {
    if (prevOverflow !== null) {
      document.body.style.overflow = prevOverflow
      prevOverflow = null
    }
    document.removeEventListener('keydown', onKeydown)
  }
})

onBeforeUnmount(() => {
  if (typeof document === 'undefined') return
  if (prevOverflow !== null) {
    document.body.style.overflow = prevOverflow
  }
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="isOpen && theme"
        class="td-modal-backdrop"
        role="dialog"
        aria-modal="true"
        :aria-label="t('themeDetail.modalAria', { name: theme.name })"
        @click.self="close"
      >
        <div class="td-standalone td-modal-shell" @click.stop>
          <div class="td-standalone-h">
            <div>
              <div class="td-standalone-eyebrow">{{ t('themePicker.detailsEyebrow') }}</div>
              <h2>{{ theme.name }}</h2>
              <div class="td-standalone-sub">
                @{{ theme.author ?? 'unknown' }} · v{{ theme.version }}
                <template v-if="theme.date"> · {{ theme.date.slice(0, 10) }}</template>
              </div>
            </div>
            <button
              class="btn icon"
              :aria-label="t('common.close')"
              :title="`${t('common.close')} (Esc)`"
              @click="close"
            >
              <UiIcon name="X" :size="13" />
            </button>
          </div>
          <div class="td-modal-body">
            <ThemeDetailDrawer
              :theme="theme"
              :preview-map="previewMap"
              :preview-details="previewDetails"
              @apply="(id) => emit('apply', id)"
              @edit="(id) => emit('edit', id)"
              @duplicate="(id) => emit('duplicate', id)"
              @export-pack="(id) => emit('exportPack', id)"
              @delete="(id) => emit('delete', id)"
              @close="close"
            />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.td-modal-backdrop {
  @apply fixed inset-0 z-[100] grid place-items-center bg-[rgba(8,9,14,0.6)] p-8 backdrop-blur-[8px];
}
/*
 * シェルは `grid place-items-center` の中で中央寄せされる。
 * 高さはコンテンツに追従し、`max-h` だけで上限を抑える。
 * `td-standalone` 由来の `height: 100%` が乗ると、グリッドトラック全体
 * (viewport - 64px) まで張ってしまい常に最大サイズになるので、レイアウト
 * 系プロパティはこちらに寄せて `td-standalone` は枠線・角丸・影だけに絞る。
 */
.td-modal-shell {
  @apply flex h-auto max-h-[calc(100vh-64px)] w-[min(960px,100%)] flex-col overflow-hidden;
}
.td-modal-body {
  @apply min-h-0 flex-1 overflow-y-auto;
}
/* モーダル内の drawer は通常のカード文脈ではないので、外側の境界線を消して二重枠を避ける */
.td-modal-body :deep(.td-drawer) {
  background: transparent;
}

.td-standalone {
  @apply rounded-[14px] border border-line-hi bg-bg-1;
  box-shadow: var(--shadow-2);
}
.td-standalone-h {
  @apply flex items-start justify-between gap-3 border-b border-line px-[22px] pb-4 pt-[18px];
}
.td-standalone-eyebrow {
  @apply mb-1 font-mono text-[9.5px] uppercase tracking-[0.16em] text-accent;
}
.td-standalone h2 {
  @apply m-0 font-display text-[20px] font-semibold tracking-[-0.02em];
}
.td-standalone-sub {
  @apply mt-1 font-mono text-[12px] tracking-[0.02em] text-fg-dim;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
