<script setup lang="ts">
/**
 * 動作環境警告バナー (Phase 4-7)
 *
 * 起動時に Rust 側 `get_environment_report` を呼び、
 * RDP / Server SKU 等の動作対象外環境であれば画面上部に警告を表示する。
 *
 * 一度閉じると同セッション中は再表示しない (sessionStorage で記憶)。
 */

const { t } = useI18n()

interface EnvironmentReport {
  is_remote_session: boolean
  is_server_sku: boolean
  product_name: string | null
  level: 'ok' | 'warn' | 'error'
  message: string | null
}

const report = ref<EnvironmentReport | null>(null)
const dismissed = ref(false)

const STORAGE_KEY = 'easy-cursor-swap.env-banner-dismissed'

onMounted(async () => {
  // セッション内で既に閉じていれば抑制
  if (typeof sessionStorage !== 'undefined' && sessionStorage.getItem(STORAGE_KEY) === '1') {
    dismissed.value = true
    return
  }
  try {
    report.value = await invokeTauri<EnvironmentReport>('get_environment_report')
  } catch (err) {
    console.warn('[EnvironmentBanner] failed:', err)
  }
})

function close() {
  dismissed.value = true
  if (typeof sessionStorage !== 'undefined') {
    sessionStorage.setItem(STORAGE_KEY, '1')
  }
}
</script>

<template>
  <Transition name="fade">
    <div v-if="report && report.level !== 'ok' && !dismissed" class="env-banner" role="alert">
      <UiIcon name="Alert" :size="14" />
      <div class="env-text">
        <strong>{{ t('environment.unsupportedDetected') }}</strong>
        <span>{{ report.message }}</span>
      </div>
      <button class="btn ghost env-close" :aria-label="t('common.close')" @click="close">
        <UiIcon name="X" :size="11" />
      </button>
    </div>
  </Transition>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.env-banner {
  @apply z-[8] flex shrink-0 items-center gap-3 border-b px-4 py-2.5 text-[12.5px] text-amber;
  background: rgba(245, 194, 107, 0.1);
  border-bottom-color: rgba(245, 194, 107, 0.35);
}
.env-text {
  @apply flex min-w-0 flex-1 flex-col gap-0.5;
}
.env-text strong {
  @apply text-[12.5px] font-semibold text-fg;
}
.env-text span {
  @apply text-[11.5px] leading-[1.45] text-fg-dim;
}
.env-close {
  @apply h-6 w-6 shrink-0 p-0;
}

/* Vue Transition の fade-* は class 名を Vue が直接適用するため utility 化不可。 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
