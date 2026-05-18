<script setup lang="ts">
/**
 * EasyCursorSwap - ルートコンポーネント
 *
 * UI テーマ (Light/Dark/Auto) は Rust 設定ではなく localStorage で
 * 永続化するため、ここで一度だけ初期化する。
 *
 * 起動時の自動アップデートチェック (`general.auto_update === true` の時) も
 * ここでキック。24h クールダウン + Toast 通知のみで実装は composable 側。
 */

useUiTheme()
bootstrapUpdaterCheck()
</script>

<template>
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>

<style>
/* グローバルスタイルの読み込み */
@import '~/assets/css/global.css';
</style>
