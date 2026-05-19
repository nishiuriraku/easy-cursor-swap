<script setup lang="ts">
/**
 * Library 画面の「空状態」(テーマが 1 件も無いとき) のヒーロー UI。
 *
 * design/empty-states.jsx の LibraryEmpty を Vue 化したもの。
 * 新規作成 / インポート / Marketplace 遷移の 3 つの導線を提供する。
 */

const { t } = useI18n()

defineEmits<{
  (e: 'open-import'): void
}>()
</script>

<template>
  <div class="es-stage">
    <div class="es-bg" />
    <div class="es-empty">
      <div class="es-empty-icon">
        <UiIcon name="Library" :size="32" style="color: var(--accent)" />
        <span class="es-empty-zero">0</span>
      </div>

      <div class="es-eyebrow" style="color: var(--fg-mute)">EMPTY · NO THEMES</div>
      <h1>{{ t('library.emptyTitle') }}</h1>
      <p>
        {{ t('library.emptyBodyPrefix') }}<code>.cursorpack</code>{{ t('library.emptyBodySuffix') }}
      </p>

      <div class="es-cta-row">
        <NuxtLink class="btn primary" to="/creator">
          <UiIcon name="Plus" :size="14" />{{ t('library.emptyNew') }}
        </NuxtLink>
        <button class="btn" @click="$emit('open-import')">
          <UiIcon name="Import" :size="13" />{{ t('library.emptyImport') }}
        </button>
        <NuxtLink class="btn ghost" to="/marketplace">
          <UiIcon name="Globe" :size="13" />{{ t('library.emptyOpenIndex') }}
        </NuxtLink>
      </div>

      <div class="es-drop">
        <div class="es-drop-inner">
          <UiIcon name="Import" :size="20" style="color: var(--accent)" />
          <div>
            <div class="es-drop-t">{{ t('library.emptyDropText') }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference '~/assets/css/tailwind.css';

.es-stage {
  @apply relative grid flex-1 overflow-auto px-8 py-10;
  place-items: center;
}
.es-bg {
  @apply pointer-events-none absolute inset-0 z-0;
  /* gradient のサイズは stage 寸法に追従させる (% ベース)。固定 px だと
   * stage が flex-1 で伸びた際に上下端まで halo が届かず dead zone ができる。
   * stop を 80% に伸ばし、上下の halo を中央近くまで滑らかに繋ぐ。 */
  background:
    radial-gradient(85% 65% at 50% 0%, rgba(124, 242, 212, 0.08), transparent 80%),
    radial-gradient(100% 70% at 50% 100%, rgba(139, 125, 255, 0.05), transparent 80%);
}
:where(html.light) .es-bg {
  background:
    radial-gradient(85% 65% at 50% 0%, rgba(15, 168, 133, 0.08), transparent 80%),
    radial-gradient(100% 70% at 50% 100%, rgba(106, 92, 255, 0.05), transparent 80%);
}
.es-empty {
  @apply relative z-10 flex w-[640px] max-w-full flex-col items-center text-center;
}
.es-empty-icon {
  @apply relative mb-[18px] grid size-20 place-items-center rounded-[20px] border border-accent-line;
  background: rgba(124, 242, 212, 0.06);
  box-shadow: 0 16px 40px -16px rgba(124, 242, 212, 0.4);
}
.es-empty-zero {
  @apply absolute -right-2.5 -top-2 rounded-full border border-accent-line bg-bg-1 px-[9px] py-px font-mono text-[13px] font-semibold tracking-[0.04em] text-accent;
}
.es-eyebrow {
  @apply mb-2.5 font-mono text-[10px] uppercase tracking-[0.18em] text-accent;
}
.es-empty h1 {
  @apply m-0 font-display text-[28px] font-semibold tracking-[-0.02em];
}
.es-empty p {
  @apply mx-0 mb-0 mt-2.5 max-w-[480px] text-[13.5px] leading-[1.7] text-fg-dim;
  text-wrap: pretty;
}
.es-empty p code {
  @apply rounded border border-accent-line bg-accent-dim px-[5px] py-px font-mono text-[12px] text-accent;
}
.es-cta-row {
  @apply mt-6 flex flex-wrap justify-center gap-2.5;
}
.es-drop {
  @apply mt-[26px] w-full max-w-[560px] rounded-xl p-3.5;
  border: 1.5px dashed var(--accent-line);
  background: rgba(124, 242, 212, 0.03);
}
:where(html.light) .es-drop {
  background: rgba(15, 168, 133, 0.04);
}
.es-drop-inner {
  @apply flex items-center gap-3.5 text-left;
}
.es-drop-t {
  @apply text-[13px] font-medium text-fg;
}
.es-drop-s {
  @apply mt-0.5 text-[11.5px] text-fg-dim;
}
</style>
