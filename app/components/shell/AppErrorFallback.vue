<script setup lang="ts">
/**
 * エラーバウンダリのフォールバック表示 (P11c)。
 *
 * `UiAlert tone="danger"` + `<pre>` の詳細 (message + stack、`v-html` 不使用) +
 * [コピー] [再起動] [閉じる] ボタン。詳細テキストは `.selectable` で選択可。
 * 再起動の実処理は caller (`AppErrorBoundary` / `error.vue`) の `reload` に委譲し、
 * ここでは `reload` / `dismiss` を emit するだけ。
 */
import type { CapturedError } from '~/composables/useAppErrorState'

const props = withDefaults(
  defineProps<{
    error: CapturedError
    showDismiss?: boolean
  }>(),
  { showDismiss: false },
)

const emit = defineEmits<{
  dismiss: []
  reload: []
}>()

const { t } = useI18n()
const { info } = useAppInfo()

const copied = ref(false)
let copiedTimer: ReturnType<typeof setTimeout> | null = null

/** バグ報告用にコピーする 1 ブロック。バージョン / 時刻 / source を先頭に付ける。 */
const detailText = computed(() => {
  const version = info.value?.version ?? '?'
  const lines = [`[EasyCursorSwap v${version}] ${props.error.at} ${props.error.source}`]
  lines.push(props.error.message)
  if (props.error.info) lines.push(`info: ${props.error.info}`)
  if (props.error.stack) lines.push('', props.error.stack)
  return lines.join('\n')
})

async function copyDetails() {
  try {
    await navigator.clipboard.writeText(detailText.value)
    copied.value = true
    if (copiedTimer) clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => {
      copied.value = false
      copiedTimer = null
    }, 2000)
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error('[error-boundary] clipboard copy failed:', e)
  }
}

onUnmounted(() => {
  if (copiedTimer) clearTimeout(copiedTimer)
})
</script>

<template>
  <UiAlert tone="danger" :title="t('errorBoundary.title')">
    <p class="eb-lead">{{ t('errorBoundary.lead') }}</p>
    <pre class="eb-detail selectable">{{ detailText }}</pre>
    <div class="eb-actions">
      <UiButton icon-left="Pkg" @click="copyDetails">
        <span v-if="copied">{{ t('common.copied') }}</span>
        <span v-else>{{ t('errorBoundary.copy') }}</span>
      </UiButton>
      <UiButton variant="primary" icon-left="Refresh" @click="emit('reload')">
        {{ t('errorBoundary.reload') }}
      </UiButton>
      <UiButton v-if="showDismiss" variant="ghost" @click="emit('dismiss')">
        {{ t('errorBoundary.dismiss') }}
      </UiButton>
    </div>
  </UiAlert>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.eb-lead {
  @apply m-0 mb-2 text-[12.5px];
}
.eb-detail {
  @apply m-0 mb-3 max-h-48 overflow-auto rounded-[6px] border border-line p-2 font-mono text-[11px] whitespace-pre-wrap;
  background: rgba(0, 0, 0, 0.25);
}
:where(html.light) .eb-detail {
  background: rgba(15, 20, 35, 0.04);
}
.eb-actions {
  @apply flex flex-wrap items-center gap-2;
}
</style>
