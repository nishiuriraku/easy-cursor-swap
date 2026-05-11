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
        <NuxtLink class="btn primary es-cta-primary" to="/creator">
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

/* NOTE: 元コードは var(--bg-elev1/elev2/base) / var(--border) / var(--text*) など
 * 未定義トークンを多用しており、それらは CSS の invalid value → 既定値で表示されていた。
 * 視覚的な現状を維持するため、移行時もそれらの参照は変更せず保持する。 */

.es-stage {
  @apply relative flex min-h-[60vh] items-center justify-center px-5 py-10;
}

.es-bg {
  @apply pointer-events-none absolute inset-0;
  background:
    radial-gradient(circle at 30% 50%, rgba(106, 213, 184, 0.06), transparent 70%),
    radial-gradient(circle at 70% 50%, rgba(124, 159, 255, 0.04), transparent 70%);
}

.es-empty {
  @apply relative flex max-w-[580px] flex-col items-center gap-4 text-center;
}

.es-empty-icon {
  @apply relative mb-2 flex size-20 items-center justify-center rounded-full;
  background: var(--bg-elev2);
  border: 1px solid var(--border);
}

.es-empty-zero {
  @apply absolute bottom-[-8px] right-[-8px] flex size-7 items-center justify-center rounded-full bg-accent text-[14px] font-bold;
  color: var(--bg-base);
}

.es-eyebrow {
  @apply text-[11px] font-semibold tracking-[0.08em];
}

.es-empty h1 {
  @apply m-0 text-[22px] font-bold;
}

.es-empty p {
  @apply m-0 text-[13px] leading-[1.5];
  color: var(--text-mute);
}

.es-empty p code {
  @apply rounded-[4px] px-1.5 py-0.5 font-mono text-[12px];
  background: var(--bg-elev2);
}

.es-cta-row {
  @apply mt-2 flex flex-wrap justify-center gap-2;
}

.es-drop {
  @apply mt-3 w-full rounded-[12px] border border-dashed p-4;
  border-color: var(--border);
  background: var(--bg-elev1);
}

.es-drop-inner {
  @apply flex items-center justify-center gap-3;
}

.es-drop-t {
  @apply text-[13px] font-semibold;
}

.es-drop-s {
  @apply text-[11px];
  color: var(--text-mute);
}

.es-foot-tip {
  @apply mt-1 inline-flex items-center gap-1 text-[11px];
  color: var(--text-mute);
}

/* このコンポーネントだけは独自の .btn ローカル定義を持っている (グローバル .btn と独立) */
.btn {
  @apply inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-[8px] border px-3.5 text-[13px] no-underline;
  border-color: var(--border);
  background: var(--bg-elev2);
  color: var(--text);
}

.btn.primary {
  @apply bg-accent;
  color: var(--bg-base);
  border-color: var(--accent);
}

.btn.ghost {
  @apply bg-transparent;
}

.es-cta-primary {
  @apply font-semibold;
}
</style>
