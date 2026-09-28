<script setup lang="ts">
const { t } = useI18n()
const { info } = useAppInfo()
const snapshotPath = computed(
  () => `${info.value?.cursors_dir ?? '~/.custom_cursors'}/_initial_snapshot.json`,
)
</script>

<template>
  <div class="ob-step">
    <p class="ob-lead">{{ t('onboarding.welcome.lead') }}</p>
    <ul class="ob-list">
      <li>{{ t('onboarding.welcome.featureLibrary') }}</li>
      <li>{{ t('onboarding.welcome.featureIndex') }}</li>
      <li>{{ t('onboarding.welcome.featureCreator') }}</li>
    </ul>
    <p class="ob-note">{{ t('onboarding.welcome.snapshot', { path: snapshotPath }) }}</p>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ob-step {
  @apply flex flex-col gap-3 text-[13px] text-fg-dim;
}
.ob-lead {
  @apply m-0 text-fg;
}
.ob-list {
  @apply m-0 flex flex-col gap-1.5 pl-5;
  list-style: disc;
}
.ob-note {
  @apply m-0 rounded-md border border-line p-2.5 font-mono text-[11.5px];
  background: rgba(124, 242, 212, 0.04);
}
</style>
