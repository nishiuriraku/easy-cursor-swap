<script setup lang="ts">
const { t } = useI18n()
const { tp } = usePlatform()
const panicHotkey = computed(
  () => useAppSettings().config.value?.general.panic_hotkey ?? 'Ctrl+Alt+Shift+R',
)
const hotkeyParts = computed(() => panicHotkey.value.split('+'))
</script>

<template>
  <div class="ob-step">
    <p class="ob-lead">{{ t('onboarding.safety.hotkeyLead') }}</p>
    <p class="ob-hotkey">
      <kbd v-for="(part, i) in hotkeyParts" :key="i">{{ part }}</kbd>
    </p>
    <ul class="ob-list">
      <li>{{ t('onboarding.safety.hotkeyStage1') }} ({{ tp('panicStage1Label') }})</li>
      <li>{{ t('onboarding.safety.hotkeyStage2') }}</li>
    </ul>
    <p class="ob-note">{{ t('onboarding.safety.trayLead') }}</p>
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
.ob-hotkey {
  @apply m-0 flex flex-wrap gap-1;
}
.ob-hotkey kbd {
  @apply rounded border border-line bg-bg-2 px-1.5 py-0.5 font-mono text-[12px] text-fg;
}
.ob-list {
  @apply m-0 flex flex-col gap-1.5 pl-5;
  list-style: disc;
}
.ob-note {
  @apply m-0;
}
</style>
