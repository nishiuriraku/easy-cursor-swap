<script setup lang="ts">
/**
 * GitHub Device Flow 用モーダル。
 *
 * `open` が true になった瞬間に `useGithubAuth().start()` を呼んで user_code を表示。
 * 同時に clipboard に user_code をコピーし「GitHub を開く」ボタンで認可ページへ。
 * `useGithubAuth().status` が `ready` になったら `ready` イベントで親に login を渡し閉じる。
 */

const { t } = useI18n()
const { status, userCode, verificationUri, login, start, cancel } = useGithubAuth()

interface Props {
  open: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [v: boolean]
  ready: [login: string]
  cancelled: []
}>()

// open=true になった瞬間に Device Flow を開始する。
watch(
  () => props.open,
  async (v) => {
    if (!v) return
    await start()
    if (userCode.value) {
      try {
        await navigator.clipboard.writeText(userCode.value)
      } catch {
        // clipboard 失敗は致命的でない (UI に大きく表示済み)
      }
    }
  },
  { immediate: true },
)

// status が ready になったら親に通知して閉じる。
watch(status, (s) => {
  if (s === 'ready' && login.value) {
    emit('ready', login.value)
    emit('update:open', false)
  }
})

async function openGithub() {
  if (!verificationUri.value) return
  await openExternalUrl(verificationUri.value)
}

async function onCancel() {
  await cancel()
  emit('cancelled')
  emit('update:open', false)
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="modal-page"
      role="dialog"
      aria-modal="true"
      aria-labelledby="df-modal-title"
      @click.self="onCancel"
    >
      <div class="modal df-modal" @click.stop>
        <div class="modal-head">
          <div class="modal-icon" aria-hidden="true">
            <UiIcon name="Globe" :size="18" />
          </div>
          <h2 id="df-modal-title" style="flex: 1; min-width: 0">
            {{ t('marketplace.deviceFlowTitle') }}
          </h2>
        </div>

        <div class="modal-body">
          <p class="hint">{{ t('marketplace.deviceFlowHint') }}</p>
          <div class="code-box" aria-live="polite">{{ userCode ?? '...' }}</div>
          <div v-if="status === 'expired'" class="warn">
            {{ t('marketplace.deviceFlowExpired') }}
          </div>
          <div v-if="status === 'denied'" class="warn">
            {{ t('marketplace.deviceFlowDenied') }}
          </div>
        </div>

        <div class="modal-foot">
          <button class="btn" @click="onCancel">
            {{ t('marketplace.deviceFlowCancel') }}
          </button>
          <button class="btn primary" :disabled="!verificationUri" @click="openGithub">
            {{ t('marketplace.deviceFlowOpenBtn') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.df-modal {
  @apply flex w-[480px] max-w-[92vw] flex-col;
}

.hint {
  @apply m-0 text-[13px] text-fg-dim;
}

.code-box {
  @apply mx-auto my-3 select-all rounded-[8px] border border-line bg-black/30 px-4 py-3 text-center font-mono text-[28px] tracking-[0.2em];
}

:where(html.light) .code-box {
  background: rgba(15, 20, 35, 0.04);
}

.warn {
  @apply rounded-[8px] border px-2.5 py-2 text-[12px];
  background: rgba(245, 100, 100, 0.1);
  border-color: rgba(245, 100, 100, 0.3);
  color: var(--red, #e55);
}
</style>
