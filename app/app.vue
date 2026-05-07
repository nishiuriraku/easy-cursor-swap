<script setup lang="ts">
/**
 * EasyCursorSwap - ルートコンポーネント
 *
 * UI テーマ (Light/Dark/Auto) は Rust 設定ではなく localStorage で
 * 永続化するため、ここで一度だけ初期化する。
 */
import { useUiTheme } from '~/composables/useUiTheme'

useUiTheme()
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
