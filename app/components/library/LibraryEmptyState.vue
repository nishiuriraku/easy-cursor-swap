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
.es-stage {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60vh;
  padding: 40px 20px;
}

.es-bg {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 30% 50%, rgba(106, 213, 184, 0.06), transparent 70%),
    radial-gradient(circle at 70% 50%, rgba(124, 159, 255, 0.04), transparent 70%);
  pointer-events: none;
}

.es-empty {
  position: relative;
  max-width: 580px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.es-empty-icon {
  position: relative;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--bg-elev2);
  border: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}

.es-empty-zero {
  position: absolute;
  bottom: -8px;
  right: -8px;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--bg-base);
  font-weight: 700;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.es-eyebrow {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
}

.es-empty h1 {
  font-size: 22px;
  font-weight: 700;
  margin: 0;
}

.es-empty p {
  font-size: 13px;
  color: var(--text-mute);
  line-height: 1.5;
  margin: 0;
}

.es-empty p code {
  font-family: var(--font-mono);
  background: var(--bg-elev2);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.es-cta-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: center;
  margin-top: 8px;
}

.es-drop {
  width: 100%;
  border: 1px dashed var(--border);
  border-radius: 12px;
  padding: 16px;
  background: var(--bg-elev1);
  margin-top: 12px;
}

.es-drop-inner {
  display: flex;
  align-items: center;
  gap: 12px;
  justify-content: center;
}

.es-drop-t {
  font-size: 13px;
  font-weight: 600;
}

.es-drop-s {
  font-size: 11px;
  color: var(--text-mute);
}

.es-foot-tip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--text-mute);
  margin-top: 4px;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev2);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  text-decoration: none;
}

.btn.primary {
  background: var(--accent);
  color: var(--bg-base);
  border-color: var(--accent);
}

.btn.ghost {
  background: transparent;
}

.es-cta-primary {
  font-weight: 600;
}
</style>
