import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import Unimport from 'unimport/unplugin'
import { fileURLToPath } from 'node:url'

// Vitest では Nuxt のビルド時 auto-import 注入が走らないため、
// 同じ役割を担う unimport (composable / Vue API) と
// unplugin-vue-components (SFC タグ解決) を明示的に組み込む。
// Nuxt 本体の挙動 (`pathPrefix: false`) と揃えるため、コンポーネント名は
// ディレクトリ階層のプレフィックス無しで解決させる。
export default defineConfig({
  plugins: [
    vue(),
    Unimport.vite({
      dts: false,
      presets: ['vue'],
      // `./app/utils/**` はディレクトリ未作成のため省略。
      // utils を新設する際にここに追加する。
      dirs: ['./app/composables/**'],
    }),
    Components({
      dts: false,
      dirs: ['app/components'],
      extensions: ['vue'],
      deep: true,
      directoryAsNamespace: false,
    }),
  ],
  test: {
    environment: 'happy-dom',
    include: ['app/**/__tests__/**/*.test.ts'],
    globals: true,
  },
  resolve: {
    alias: {
      '~': fileURLToPath(new URL('./app', import.meta.url)),
    },
  },
})
