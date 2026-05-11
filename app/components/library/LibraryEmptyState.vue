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

<!-- NOTE: 元の scoped style は var(--bg-elev*)/var(--border)/var(--text*) など
  未定義トークンに依存しており、実際の見た目は global.css の .es-* / .btn ルールが
  提供していた (scoped はほぼ dead-code 状態)。
  scoped を維持すると Tailwind utility が global を上書きして visual regression が
  起きるため、scoped style 全体を削除して global ルールに一任。 -->
