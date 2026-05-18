// EasyCursorSwap - Nuxt 設定
// Tauri v2 連携用の SPA モード設定
import tailwindcss from '@tailwindcss/vite'

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',

  // Tauri は SSR 非対応 → SPA モードでレンダリング
  // NOTE: Nuxt 4.4.4 で ssr:false が IPC エラーを引き起こすため
  // routeRules で全ルートを SPA にフォールバックさせる
  routeRules: {
    '/**': { ssr: false },
  },

  // production ビルドでは Nuxt DevTools の UI フックを完全に閉じる。
  // 既定でも production には注入されないが、リリース漏れを防ぐため明示的に gate する。
  devtools: { enabled: process.env.NODE_ENV !== 'production' },

  // ディレクトリ名のプレフィックスを付けず、`<UiIcon>` や `<ThemeCard>` のように
  // ファイル名そのままで参照できるようにする (デザイン仕様の命名規則と整合)
  components: [
    { path: '~/components', pathPrefix: false },
  ],

  devServer: {
    // Tauri が使用するポートを固定
    port: 3000,
  },

  // CSS load 順: Tailwind base + theme tokens を先に、global.css は後ろで上書きする
  // (Phase 8 で global.css を縮小するまで、preflight 競合は global.css 側で吸収)
  css: ['~/assets/css/tailwind.css', '~/assets/css/global.css'],

  vite: {
    plugins: [tailwindcss()],
    // Tauri CLI と Vite の画面クリアが衝突しないようにする
    clearScreen: false,
    // Tauri の環境変数を Vite に渡す
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      // HMR の設定
      strictPort: true,
    },
  },

  app: {
    head: {
      title: 'EasyCursorSwap',
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'description', content: 'Windows用次世代マウスカーソル管理ツール' },
      ],
      link: [
        {
          rel: 'preconnect',
          href: 'https://fonts.googleapis.com',
        },
        {
          rel: 'preconnect',
          href: 'https://fonts.gstatic.com',
          crossorigin: '',
        },
        {
          rel: 'stylesheet',
          href: 'https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=Noto+Sans+JP:wght@300;400;500;600;700&display=swap',
        },
      ],
    },
  },
})
