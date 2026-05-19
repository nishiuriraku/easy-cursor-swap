<script setup lang="ts">
/**
 * Library 画面のヘッダー (パンくず + 検索ボックス + Import/New ボタン)。
 *
 * 検索クエリは v-model で双方向バインディング、Import ボタンは emit で親に通知する。
 * New ボタンは `/creator` への遷移を担うが、WebView2 の URL プレビュー (リンクホバー時に
 * 左下へ出る) を回避するため `<NuxtLink custom v-slot>` で <button> にレンダリングする
 * (DOM 上に <a href> を残さない)。slot props の navigate() が router.push に対応。
 */

const { t } = useI18n()

const searchQuery = defineModel<string>('searchQuery', { required: true })

defineEmits<{
  (e: 'open-import'): void
}>()
</script>

<template>
  <div class="toolbar">
    <div class="bcrumb">
      <span class="crumb">{{ t('library.breadcrumbWorkspace') }}</span>
      <span class="sep">/</span>
      <span class="crumb active">{{ t('library.title') }}</span>
    </div>
    <div class="search">
      <UiIcon name="Search" :size="14" style="color: var(--fg-mute)" />
      <input
        v-model="searchQuery"
        :placeholder="t('library.searchPlaceholder')"
        :aria-label="t('common.search')"
      />
    </div>
    <div class="tb-actions">
      <button class="btn ghost" @click="$emit('open-import')">
        <UiIcon name="Import" :size="14" />{{ t('common.import') }}
      </button>
      <NuxtLink to="/creator" custom>
        <template #default="{ navigate }">
          <button class="btn primary" @click="navigate">
            <UiIcon name="Plus" :size="14" />{{ t('library.new') }}
          </button>
        </template>
      </NuxtLink>
    </div>
  </div>
</template>

<!-- NOTE: 元の scoped style は var(--border)/--bg-elev1/--bg-elev2/--text/--text-mute
  などの未定義トークンに依存しており、それらの declaration は invalid となって
  global.css の .toolbar/.bcrumb/.search/.kbd/.tb-actions/.btn 等が cascade で
  当たっていた。scoped を維持すると Tailwind utility (border / bg / text) が
  global ルールを上書きして見た目が崩れるため、scoped style は丸ごと削除し
  global ルールに一任する。 -->
