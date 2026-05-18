/**
 * `pickLocalizedName` のフォールバックチェーン検証。
 *
 * Rust 側 `LocalizedString::get` と挙動を 1:1 一致させる契約があるため、
 * Rust 側の挙動 (`src-tauri/src/theme/types.rs:214-234` の `get`) と本テストを
 * 並行して読むこと。どちらか一方を変える場合は両方を変える。
 */
import { describe, expect, it } from 'vitest'
import { pickLocalizedName } from '../pickLocalizedName'

describe('pickLocalizedName', () => {
  it('plain string はそのまま返す (LocalizedString::Simple ケース)', () => {
    // 既存の curated index は全てこの形 ("EasyCursorSwap Mint" 等)。
    // 旧スキーマ完全互換性のテスト。
    expect(pickLocalizedName('EasyCursorSwap Mint', 'ja')).toBe('EasyCursorSwap Mint')
    expect(pickLocalizedName('EasyCursorSwap Mint', 'en')).toBe('EasyCursorSwap Mint')
    expect(pickLocalizedName('EasyCursorSwap Mint', 'zh')).toBe('EasyCursorSwap Mint')
  })

  it('指定ロケールがマップに存在すれば最優先で返す', () => {
    const name = { ja: 'ミント', en: 'Mint', default: 'EasyCursorSwap Mint' }
    expect(pickLocalizedName(name, 'ja')).toBe('ミント')
    expect(pickLocalizedName(name, 'en')).toBe('Mint')
  })

  it('指定ロケールが無い場合は default → en → 最初の値 の順にフォールバック', () => {
    // default あり: default が勝つ
    expect(pickLocalizedName({ en: 'Mint', default: 'X' }, 'zh')).toBe('X')
    // default 無し、en あり: en が勝つ
    expect(pickLocalizedName({ ja: 'ミント', en: 'Mint' }, 'zh')).toBe('Mint')
    // default 無し、en 無し: 残った最初の値
    expect(pickLocalizedName({ fr: 'Menthe' }, 'zh')).toBe('Menthe')
  })

  it('空マップは空文字列を返す', () => {
    // 異常系: ja.ts 側の {{ name }} 補間が壊れないように空文字列で受ける。
    // UI はトーストでこのケースを `'Untitled'` 等に差し替えるかどうか判断する。
    expect(pickLocalizedName({}, 'ja')).toBe('')
  })

  it('Rust の LocalizedString::get と完全一致 — 指定 locale > default > en の優先順位', () => {
    // 「default が無い & 指定 locale も無い & en あり」のとき en にフォールバック。
    // Rust 側のテスト名: theme/types tests::localized_falls_back_to_en
    expect(pickLocalizedName({ ja: 'ミント', en: 'Mint' }, 'fr')).toBe('Mint')
  })
})
