<script setup lang="ts">
/**
 * 画面下部フローティングバナー (Library / Creator 共通、P08a Step 1)。
 *
 * `tone: 'accent'` = 成功・情報系 (旧 creator `.import-banner`)、
 * `tone: 'danger'` = エラー系 (旧 index `.apply-error`)。
 * `Transition name="fade"` は呼び出し側が包む (既存どおり)。
 */

const { t } = useI18n()

withDefaults(
  defineProps<{
    tone: 'accent' | 'danger'
    icon?: string
    role?: 'status' | 'alert'
  }>(),
  { icon: undefined, role: 'status' },
)

defineEmits<{
  dismiss: []
}>()
</script>

<template>
  <div class="floating-banner" :class="`tone-${tone}`" :role="role">
    <UiIcon v-if="icon" :name="icon" :size="13" />
    <span class="banner-body"><slot /></span>
    <span class="banner-actions"><slot name="actions" /></span>
    <button
      class="btn ghost banner-close"
      :aria-label="t('common.close')"
      @click="$emit('dismiss')"
    >
      <UiIcon name="X" :size="11" />
    </button>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.floating-banner {
  @apply fixed bottom-12 left-1/2 z-[90] flex min-w-[320px] max-w-[80%] -translate-x-1/2 items-center gap-2.5 rounded-[8px] border px-3.5 py-2.5 text-[12.5px] text-fg-dim backdrop-blur-[12px];
  box-shadow: var(--shadow-2);
}
.tone-accent {
  @apply border-accent-line;
  background: rgba(124, 242, 212, 0.1);
}
.tone-danger {
  border-color: rgba(255, 107, 138, 0.4);
  background: rgba(255, 107, 138, 0.12);
  color: #ffb8c5;
}
.banner-body {
  @apply flex min-w-0 flex-1 items-center gap-2;
}
.banner-actions {
  @apply flex items-center gap-2;
}
.banner-close {
  height: 24px;
  flex: none;
}
</style>
