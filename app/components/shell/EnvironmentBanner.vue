<script setup lang="ts">
/**
 * 動作環境警告バナー (Phase 4-7)
 *
 * 起動時に Rust 側 `get_environment_report` を呼び、
 * RDP / Server SKU 等の動作対象外環境であれば画面上部に警告を表示する。
 *
 * 一度閉じると同セッション中は再表示しない (sessionStorage で記憶)。
 */
import { onMounted, ref } from 'vue'
import { invokeTauri } from '~/composables/useTauri'
import { useI18n } from '~/composables/useI18n'

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
    <div
      v-if="report && report.level !== 'ok' && !dismissed"
      class="env-banner"
      role="alert"
    >
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
.env-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  background: rgba(245, 194, 107, 0.10);
  border-bottom: 1px solid rgba(245, 194, 107, 0.35);
  color: var(--amber);
  font-size: 12.5px;
  flex-shrink: 0;
  z-index: 8;
}
.env-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}
.env-text strong {
  color: var(--fg);
  font-weight: 600;
  font-size: 12.5px;
}
.env-text span {
  font-size: 11.5px;
  color: var(--fg-dim);
  line-height: 1.45;
}
.env-close {
  height: 24px;
  width: 24px;
  padding: 0;
  flex-shrink: 0;
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.18s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}
</style>
