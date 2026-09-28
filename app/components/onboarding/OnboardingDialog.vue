<script setup lang="ts">
/** 初回起動ガイド (3 ステップ)。表示状態は useOnboarding singleton。UiModal で a11y (dialog/aria-modal/focus trap/Esc) を継承。 */
import { ONBOARDING_STEPS, useOnboarding } from '~/composables/useOnboarding'

const { t } = useI18n()
const { open, step, saving, next, back, complete } = useOnboarding()
const stepIndex = computed(() => ONBOARDING_STEPS.indexOf(step.value))
const isFirst = computed(() => stepIndex.value === 0)
const isLast = computed(() => stepIndex.value === ONBOARDING_STEPS.length - 1)
const titleId = 'onboarding-title'
</script>

<template>
  <UiModal
    :open="open"
    :title="t(`onboarding.${step}.title`)"
    icon="Logo"
    size="lg"
    :close-on-backdrop="false"
    :busy="saving"
    :aria-labelledby="titleId"
    @close="complete"
  >
    <!-- 現在地ドット: aria は "ステップ n / 3" をテキストで -->
    <div class="ob-dots" role="status" aria-live="polite">
      <span class="sr-only">{{
        t('onboarding.progress', { n: stepIndex + 1, total: ONBOARDING_STEPS.length })
      }}</span>
      <i
        v-for="(s, i) in ONBOARDING_STEPS"
        :key="s"
        :class="['ob-dot', { active: i === stepIndex, done: i < stepIndex }]"
        aria-hidden="true"
      />
    </div>
    <OnboardingStepWelcome v-if="step === 'welcome'" />
    <OnboardingStepSafety v-else-if="step === 'safety'" />
    <OnboardingStepStart v-else @action-taken="complete" />
    <template #leftNote>
      <UiButton variant="ghost" :disabled="saving" @click="complete">{{
        t('onboarding.skip')
      }}</UiButton>
    </template>
    <template #actions>
      <UiButton v-if="!isFirst" variant="ghost" :disabled="saving" @click="back">{{
        t('common.back')
      }}</UiButton>
      <UiButton v-if="!isLast" variant="primary" @click="next">{{ t('onboarding.next') }}</UiButton>
      <UiButton v-else variant="primary" icon-left="Check" :loading="saving" @click="complete">{{
        t('onboarding.finish')
      }}</UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ob-dots {
  @apply mb-4 flex items-center justify-center gap-1.5;
}
.ob-dot {
  @apply size-1.5 rounded-full bg-line;
}
.ob-dot.active {
  @apply bg-accent;
}
.ob-dot.done {
  @apply bg-accent-dim;
}
</style>
