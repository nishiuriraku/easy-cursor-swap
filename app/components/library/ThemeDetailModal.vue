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

const { t } = useI18n()

const props = defineProps<{
  /** 開く対象のテーマ。null のときはモーダル非表示。 */
  theme: ThemeCardData | null
  /** 役割名 → PNG Object URL のマップ。null のときは UiIcon フォールバック。 */
  previewMap: Record<string, string> | null
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
        :aria-label="`${theme.name} の詳細`"
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
.td-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  background: rgba(8, 9, 14, 0.6);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: grid;
  place-items: center;
  padding: 32px;
}
.td-modal-shell {
  width: min(960px, 100%);
  max-height: calc(100vh - 64px);
  height: auto;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.td-modal-body {
  flex: 1;
  overflow-y: auto;
}
/* モーダル内の drawer は通常のカード文脈ではないので、外側の境界線を消して二重枠を避ける */
.td-modal-body :deep(.td-drawer) {
  background: transparent;
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
