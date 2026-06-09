/**
 * marketplaceTagLabel: 公式インデックス提出タグの ID (英語 enum, ALLOWED_MARKETPLACE_TAGS) を
 * locale 化した表示ラベルに変換する純関数。提出値は英語のまま、表示だけ i18n キー
 * `marketplace.tag<Capitalized>` で解決する (Y14: 提出モーダルのタグが日本語化されない不具合)。
 */
import { describe, it, expect } from 'vitest'
import { marketplaceTagLabel } from '../marketplaceTagLabel'

// 既知キーだけ訳を返し未知キーはキー自体を返す疑似 t (本物の useI18n.t と同じフォールバック挙動)
const t = (key: string): string =>
  (
    ({
      'marketplace.tagLight': 'ライト',
      'marketplace.tagNeon': 'ネオン',
    }) as Record<string, string>
  )[key] ?? key

describe('marketplaceTagLabel', () => {
  it('既知タグを i18n キー marketplace.tag<Capitalized> で訳す', () => {
    expect(marketplaceTagLabel('light', t)).toBe('ライト')
    expect(marketplaceTagLabel('neon', t)).toBe('ネオン')
  })

  it('未知タグ (キー解決失敗) は生のタグ文字列にフォールバックする', () => {
    expect(marketplaceTagLabel('zzz', t)).toBe('zzz')
  })

  it('空文字はそのまま返す', () => {
    expect(marketplaceTagLabel('', t)).toBe('')
  })
})
