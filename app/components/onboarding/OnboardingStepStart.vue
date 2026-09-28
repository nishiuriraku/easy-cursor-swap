<script setup lang="ts">
const { t } = useI18n()

const emit = defineEmits<{
  actionTaken: []
}>()

async function go(target: 'import' | 'index' | 'creator') {
  emit('actionTaken') // 親が complete() を await せずに閉じる
  if (target === 'import') await navigateTo({ path: '/', query: { openImport: '1' } })
  else if (target === 'index') await navigateTo('/marketplace')
  else await navigateTo('/creator')
}
</script>

<template>
  <div class="ob-step">
    <p class="ob-lead">{{ t('onboarding.start.title') }}</p>
    <div class="ob-cards">
      <button type="button" class="ob-card" @click="go('import')">
        <UiIcon name="Import" :size="20" aria-hidden="true" />
        <span class="ob-card-title">{{ t('onboarding.start.importTitle') }}</span>
        <span class="ob-card-desc">{{ t('onboarding.start.importDesc') }}</span>
      </button>
      <button type="button" class="ob-card" @click="go('index')">
        <UiIcon name="Globe" :size="20" aria-hidden="true" />
        <span class="ob-card-title">{{ t('onboarding.start.indexTitle') }}</span>
        <span class="ob-card-desc">{{ t('onboarding.start.indexDesc') }}</span>
      </button>
      <button type="button" class="ob-card" @click="go('creator')">
        <UiIcon name="Brush" :size="20" aria-hidden="true" />
        <span class="ob-card-title">{{ t('onboarding.start.creatorTitle') }}</span>
        <span class="ob-card-desc">{{ t('onboarding.start.creatorDesc') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.ob-step {
  @apply flex flex-col gap-3 text-[13px] text-fg-dim;
}
.ob-lead {
  @apply m-0 font-semibold text-fg;
}
.ob-cards {
  @apply grid grid-cols-3 gap-2.5;
}
.ob-card {
  @apply flex flex-col items-start gap-1 rounded-[8px] border border-line p-3 text-left;
  background: rgba(124, 242, 212, 0.04);
}
.ob-card:hover {
  @apply border-accent-line;
}
.ob-card-title {
  @apply text-[12.5px] font-semibold text-fg;
}
.ob-card-desc {
  @apply text-[11.5px] text-fg-dim;
}
</style>
