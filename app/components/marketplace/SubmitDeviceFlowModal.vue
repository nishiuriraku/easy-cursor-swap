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
  <UiModal
    :open="open"
    :title="t('marketplace.deviceFlowTitle')"
    icon="Globe"
    size="md"
    @close="onCancel"
  >
    <p class="hint">{{ t('marketplace.deviceFlowHint') }}</p>
    <div class="code-box" aria-live="polite">{{ userCode ?? '...' }}</div>
    <UiAlert v-if="status === 'expired'" tone="danger">
      {{ t('marketplace.deviceFlowExpired') }}
    </UiAlert>
    <UiAlert v-if="status === 'denied'" tone="danger">
      {{ t('marketplace.deviceFlowDenied') }}
    </UiAlert>

    <template #actions>
      <UiButton variant="ghost" @click="onCancel">
        {{ t('marketplace.deviceFlowCancel') }}
      </UiButton>
      <UiButton variant="primary" :disabled="!verificationUri" @click="openGithub">
        {{ t('marketplace.deviceFlowOpenBtn') }}
      </UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.hint {
  @apply m-0 text-[13px] text-fg-dim;
}

.code-box {
  @apply mx-auto my-3 select-all rounded-[8px] border border-line bg-black/30 px-4 py-3 text-center font-mono text-[28px] tracking-[0.2em];
}

:where(html.light) .code-box {
  background: rgba(15, 20, 35, 0.04);
}
</style>
