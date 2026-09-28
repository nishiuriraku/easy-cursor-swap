import { expect } from 'vitest'
import * as matchers from 'vitest-axe/matchers'
import { configureAxe } from 'vitest-axe'

expect.extend(matchers)

/**
 * happy-dom はレイアウト / 計算済スタイルを持たないため、視覚依存ルールは無効化する。
 * ページ全体前提のランドマーク系も、コンポーネント単体 mount では偽陽性になるので外す。
 */
export const axe = configureAxe({
  rules: {
    'color-contrast': { enabled: false },
    'scrollable-region-focusable': { enabled: false },
    region: { enabled: false },
    'landmark-one-main': { enabled: false },
    'page-has-heading-one': { enabled: false },
  },
})
