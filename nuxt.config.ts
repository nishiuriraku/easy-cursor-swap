// CursorForge - Nuxt 設定
// Tauri v2 連携用の SPA モード設定
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',

  // Tauri は SSR 非対応 → SPA モードでレンダリング
  // NOTE: Nuxt 4.4.4 で ssr:false が IPC エラーを引き起こすため
  // routeRules で全ルートを SPA にフォールバックさせる
  routeRules: {
    '/**': { ssr: false },
  },

  devtools: { enabled: true },

  // ディレクトリ名のプレフィックスを付けず、`<UiIcon>` や `<ThemeCard>` のように
  // ファイル名そのままで参照できるようにする (デザイン仕様の命名規則と整合)
  components: [
    { path: '~/components', pathPrefix: false },
  ],

  devServer: {
    // Tauri が使用するポートを固定
    port: 3000,
  },

  vite: {
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
      title: 'CursorForge',
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
