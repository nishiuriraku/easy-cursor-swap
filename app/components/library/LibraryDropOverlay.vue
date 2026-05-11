<script setup lang="ts">
/**
 * Library 画面のドロップオーバーレイ。
 *
 * `.cursorpack` をウィンドウへドラッグしたときに半透明で覆って視覚フィードバックする。
 * 表示制御は親側 (`showDrop` ref) で `dragenter`/`dragleave` を見て切り替える。
 */
import { useI18n } from '~/composables/useI18n'

const { t } = useI18n()

defineProps<{
  show: boolean
}>()
</script>

<template>
  <Transition name="fade">
    <div v-if="show" class="drop">
      <div class="drop-inner">
        <UiIcon name="Pkg" :size="56" class="ghost-icon" />
        <h3>{{ t('library.drop') }}</h3>
        <p>{{ t('library.dropSub') }}</p>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.drop {
  @apply absolute inset-0 z-10 grid place-items-center;
  background: rgba(10, 11, 15, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
.drop-inner {
  @apply w-[480px] rounded-2xl p-10 text-center;
  border: 1.5px dashed var(--accent-line);
  background: rgba(124, 242, 212, 0.04);
}
.drop-inner h3 {
  @apply m-0 font-display text-[18px] font-semibold tracking-[-0.01em];
  margin: 12px 0 6px;
}
.drop-inner p {
  @apply m-0 text-[13px] text-fg-dim;
}
.drop-inner .ghost-icon {
  @apply text-accent;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
