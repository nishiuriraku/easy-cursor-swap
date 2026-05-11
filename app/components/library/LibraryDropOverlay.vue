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
/* 元の scoped style は var(--bg-elev1)/var(--text*) 等の未定義トークンに依存し、
 * 実際の見た目は global.css の .drop / .drop-inner / .drop-inner h3 / .ghost-icon
 * ルールが提供していた (scoped はほぼ dead-code 状態)。
 * scoped を維持すると Tailwind utility が global を上書きして visual regression が
 * 起きるため、Vue Transition の fade-* だけ残して他は削除。 */

.fade-enter-active,
.fade-leave-active {
  transition: opacity 200ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
