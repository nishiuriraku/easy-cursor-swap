// EasyCursorSwap - ESLint flat config
//
// 公式 @nuxt/eslint-config の flat preset をベースに、プロジェクト固有の
// 不変条件 (no-v-html / no-console fallback) と無視ディレクトリを足している。
// Task 12 で追加された CI gate (`npm run lint:check`) の正準設定。
import { createConfigForNuxt } from '@nuxt/eslint-config/flat'

export default createConfigForNuxt(
  {
    features: {
      // 公式 preset の既定で十分 (JS + TS + Vue + Import がすべて有効)。
      // Stylistic は Prettier に任せるので入れない。
      stylistic: false,
    },
    dirs: {
      // Nuxt 4 のソースディレクトリ構造 (app/ をルート扱い)。
      root: ['./app'],
      src: ['./app'],
    },
  },
  {
    // プロジェクト共通の ignores。preset 標準の ignores に加えて
    // Rust ビルド成果物と型生成物を除外する。
    name: 'easy-cursor-swap/ignores',
    ignores: [
      '**/src-tauri/target/**',
      '**/app/types/generated/**',
      '**/*.vue.d.ts',
      // 旧タスクで除去された IPC/プラグイン残骸 (再出現防止のため無視)
      '**/mocks/**',
    ],
  },
  {
    // No `v-html` ANYWHERE — プロジェクトのハード不変条件。
    // 公式 preset の `flat/recommended` は warn 止まりなので、明示的に error 昇格する。
    name: 'easy-cursor-swap/security/no-v-html',
    files: ['**/*.vue'],
    rules: {
      'vue/no-v-html': 'error',
    },
  },
  {
    // 内部 SVG アイコン実装 (UiIcon / CursorIcon) は innerHTML プロパティで
    // 静的 SVG body を注入する。これは sanitizeSvg 済みの内部定数
    // (UI_ICONS / CURSOR_ICONS) のみで運用するため、no-innerHTML / security
    // ルールはプロジェクト判断で許可する。
    // それ以外の .vue / .ts で innerHTML / outerHTML を新規利用するのは
    // XSS ガードが別途検出する (vue-vhtml-xss-guard)。
    name: 'easy-cursor-swap/security/icon-innerHTML',
    files: [
      'app/components/icons/UiIcon.vue',
      'app/components/icons/CursorIcon.vue',
    ],
    rules: {
      // innerHTML への代入を許可 (静的 SVG body 注入経路)
    },
  },
  {
    // `no-console` はブロックしない。console.warn / console.error は
    // ブラウザ側フォールバック診断パスとして意図的に使われている。
    name: 'easy-cursor-swap/no-console-soft',
    rules: {
      'no-console': 'off',
    },
  },
  {
    // TypeScript ルールの調整 (preset 既定で strict が有効)。
    // `any` の利用は warning まで落とす (プロジェクト内で限定的に利用)。
    name: 'easy-cursor-swap/typescript-loosen',
    rules: {
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
)