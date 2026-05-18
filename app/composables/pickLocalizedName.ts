/**
 * Marketplace エントリの `name` フィールドから表示用文字列を取り出すヘルパー。
 *
 * Rust 側 `crate::theme::LocalizedString::get` と挙動を完全一致させる:
 *   1. value が plain string  → そのまま返す
 *   2. value が locale map    → 以下の優先順で探す
 *        a. 指定 locale (例: 'ja')
 *        b. 'default' キー
 *        c. 'en' キー
 *        d. map の最初の値 (deterministic order ではないが、想定外の locale だけ
 *           入っていた場合の最終フォールバック)
 *        e. 全部空なら '' を返す (UI 側で `t('marketplace.untitled')` 等を被せる)
 *
 * `useI18n().locale` は `Ref<Locale>` でリアクティブなので、SFC 側では `computed`
 * の中で本関数を呼ぶことで言語切替に追従できる。
 *
 * 設計判断: フロントエンド独自のフォールバックは導入しない。Rust 側と JS 側で
 * フォールバック規則が違うと、`marketplace.installedToast` トーストと Card 表示で
 * 別の文字列が出る等のラグが発生するため、まずは Rust に揃える。
 */
import type { MarketplaceName } from '~/types/marketplace'

export function pickLocalizedName(name: MarketplaceName, locale: string): string {
  if (typeof name === 'string') return name
  const map = name
  // 直接 hit
  const hit = map[locale]
  if (typeof hit === 'string') return hit
  // 'default' フォールバック
  const def = map.default
  if (typeof def === 'string') return def
  // 'en' フォールバック (Rust 実装と同じ最後の名指し fallback)
  const en = map.en
  if (typeof en === 'string') return en
  // 想定外のキーしか無い場合: 最初の文字列値を返す
  for (const v of Object.values(map)) {
    if (typeof v === 'string') return v
  }
  return ''
}
