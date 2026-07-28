<script setup lang="ts">
/**
 * 名前付きステージ表示。ストリーミングフローの現在地を示す。
 *
 * 例: creator export = role → package → sign / bulk import = scan → parse → extract。
 * stages を canonical 順に渡し、currentId で active ステージを指定する。
 * failedId を渡すとそのステージを失敗表示にする。
 *
 * 各ステージの状態:
 *  - done   : current より前 (Check アイコン)
 *  - active : current と一致 (スピナー)
 *  - failed : failedId と一致 (X アイコン)
 *  - pending: current より後 (ドット)
 *
 * 表示専用 (i18n はしない)。親が翻訳済み label を渡す (LD4)。
 */
interface Stage {
  id: string
  label: string
}

type StageStatus = 'done' | 'active' | 'pending' | 'failed'

const props = withDefaults(
  defineProps<{
    stages: Stage[]
    currentId: string
    failedId?: string | null
  }>(),
  {
    failedId: null,
  },
)

const currentIndex = computed(() => props.stages.findIndex((s) => s.id === props.currentId))

function statusOf(index: number): StageStatus {
  const stage = props.stages[index]
  if (stage && props.failedId && stage.id === props.failedId) return 'failed'
  const ci = currentIndex.value
  if (ci < 0) return 'pending'
  if (index < ci) return 'done'
  if (index === ci) return 'active'
  return 'pending'
}
</script>

<template>
  <ol class="ui-stepper">
    <li
      v-for="(stage, i) in props.stages"
      :key="stage.id"
      class="ui-stepper-item"
      :class="statusOf(i)"
      :aria-current="statusOf(i) === 'active' ? 'step' : undefined"
    >
      <span class="ui-stepper-marker" aria-hidden="true">
        <UiIcon v-if="statusOf(i) === 'done'" name="Check" :size="10" />
        <UiIcon v-else-if="statusOf(i) === 'failed'" name="X" :size="10" />
        <UiSpinner v-else-if="statusOf(i) === 'active'" :size="12" />
        <span v-else class="ui-stepper-dot" />
      </span>
      <span class="ui-stepper-label">{{ stage.label }}</span>
    </li>
  </ol>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ui-stepper {
  @apply m-0 flex list-none flex-wrap items-center gap-x-3 gap-y-1.5 p-0;
}
.ui-stepper-item {
  @apply flex items-center gap-1.5 text-[12px] text-fg-faint;
}
.ui-stepper-item.done {
  @apply text-fg-mute;
}
.ui-stepper-item.active {
  @apply font-medium text-fg;
}
.ui-stepper-item.failed {
  color: var(--rose);
}
.ui-stepper-marker {
  @apply grid size-4 shrink-0 place-items-center;
}
.ui-stepper-item.done .ui-stepper-marker {
  color: var(--accent);
}
.ui-stepper-dot {
  @apply size-1.5 rounded-full;
  background: var(--line-strong);
}
</style>
