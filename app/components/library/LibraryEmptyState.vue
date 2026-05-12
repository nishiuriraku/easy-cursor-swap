<script setup lang="ts">
/**
 * Library 画面の「空状態」(テーマが 1 件も無いとき) のヒーロー UI。
 *
 * design/empty-states.jsx の LibraryEmpty を Vue 化したもの。
 * 新規作成 / インポート / Marketplace 遷移の 3 つの導線を提供する。
 */
import { useI18n } from '~/composables/useI18n'

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
        <code>.cursorpack</code> をこのウィンドウへドラッグするか、
        公式インデックスから取り込み、または Creator で新しく作りましょう。
      </p>

      <div class="es-cta-row">
        <NuxtLink class="btn primary" to="/creator">
          <UiIcon name="Plus" :size="14" />新規作成
        </NuxtLink>
        <button class="btn" @click="$emit('open-import')">
          <UiIcon name="Import" :size="13" />.cursorpack をインポート
        </button>
        <NuxtLink class="btn ghost" to="/marketplace">
          <UiIcon name="Globe" :size="13" />インデックスを開く
        </NuxtLink>
      </div>

      <div class="es-drop">
        <div class="es-drop-inner">
          <UiIcon name="Import" :size="20" style="color: var(--accent)" />
          <div>
            <div class="es-drop-t">.cursorpack をここにドロップ</div>
            <div class="es-drop-s">{{ t('drop.autoVerify') }}</div>
          </div>
        </div>
      </div>

      <div class="es-foot-tip">
        <UiIcon name="Shield" :size="11" style="color: var(--accent)" />
        <span>{{ t('drop.footTip') }}</span>
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
  background:
    radial-gradient(700px 400px at 50% 0%, rgba(124, 242, 212, 0.08), transparent 60%),
    radial-gradient(900px 500px at 50% 100%, rgba(139, 125, 255, 0.05), transparent 60%);
}
:where(html.light) .es-bg {
  background:
    radial-gradient(700px 400px at 50% 0%, rgba(15, 168, 133, 0.08), transparent 60%),
    radial-gradient(900px 500px at 50% 100%, rgba(106, 92, 255, 0.05), transparent 60%);
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
.es-foot-tip {
  @apply mt-[18px] flex items-center gap-2 font-mono text-[10.5px] tracking-[0.02em] text-fg-mute;
}
.es-foot-tip code {
  @apply rounded-[3px] px-[5px] py-px text-fg-dim;
  background: rgba(255, 255, 255, 0.04);
}
:where(html.light) .es-foot-tip code {
  background: rgba(15, 20, 35, 0.04);
}
</style>
