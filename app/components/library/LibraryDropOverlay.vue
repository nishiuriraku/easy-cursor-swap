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
.drop {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(10, 11, 15, 0.85);
  backdrop-filter: blur(8px);
  z-index: 100;
  pointer-events: none;
}

.drop-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 32px 48px;
  border-radius: 16px;
  border: 2px dashed var(--accent);
  background: var(--bg-elev1);
}

.drop-inner h3 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 18px;
  color: var(--text);
}

.drop-inner p {
  margin: 0;
  font-size: 13px;
  color: var(--text-mute);
}

.ghost-icon {
  color: var(--accent);
  opacity: 0.6;
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
