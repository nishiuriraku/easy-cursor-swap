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
  @apply pointer-events-none absolute inset-0 z-[100] flex items-center justify-center bg-[rgba(10,11,15,0.85)] backdrop-blur-[8px];
}

.drop-inner {
  @apply flex flex-col items-center gap-3 rounded-2xl border-2 border-dashed border-accent px-12 py-8;
  /* NOTE: var(--bg-elev1) は元コードから未定義 (resolved to invalid → fallback)。
   * 視覚的な現状を保つためそのまま残す。 */
  background: var(--bg-elev1);
}

.drop-inner h3 {
  @apply m-0 font-display text-[18px];
  color: var(--text);
}

.drop-inner p {
  @apply m-0 text-[13px];
  color: var(--text-mute);
}

.ghost-icon {
  @apply text-accent opacity-60;
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
