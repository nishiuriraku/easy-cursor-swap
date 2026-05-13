/**
 * CursorMatrix の limit プロップ動作を固定化する。
 *
 * 2026-05-14: ライブラリのテーマカードでは `:limit="6"` を渡し、
 * `includedRoles` の先頭 6 個のみを表示する仕様に変更した。
 * Marketplace カードなど limit 未指定の呼び出しは従来の 17 セル表示を維持する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CursorMatrix from '../CursorMatrix.vue'

describe('CursorMatrix', () => {
  it('limit 未指定なら 17 セル (CURSOR_ROLES 全件) を描画する', () => {
    const w = mount(CursorMatrix, {
      props: { included: ['Arrow', 'IBeam'] },
      global: { stubs: { CursorIcon: true } },
    })
    expect(w.findAll('.cell')).toHaveLength(17)
  })

  it('limit=6 を渡すと CURSOR_ROLES 正規順の先頭 6 個 (含まれるロールのみ) を描画する', () => {
    const w = mount(CursorMatrix, {
      props: {
        // 意図的に乱れた順序で渡す。CURSOR_ROLES 順序 = Arrow, Help, AppStarting, Wait, Crosshair, IBeam, NWPen, No...
        included: ['IBeam', 'Hand', 'Arrow', 'No', 'Help', 'Wait', 'AppStarting', 'Crosshair'],
        limit: 6,
      },
      global: { stubs: { CursorIcon: true } },
    })
    const cells = w.findAll('.cell')
    expect(cells).toHaveLength(6)
    // empty セルは limit モードでは出さない
    expect(w.findAll('.cell.empty')).toHaveLength(0)
    // canonical 順 (Arrow → Help → AppStarting → Wait → Crosshair → IBeam) で並ぶ。
    // Hand / No は 7 番目以降の CURSOR_ROLES 順だが、最初の 6 ロールが揃っているので除外される。
    const titles = cells.map((c) => c.attributes('title'))
    expect(titles).toEqual([
      '通常の選択', // Arrow
      'ヘルプの選択', // Help
      'バックグラウンド作業', // AppStarting
      '待ち状態', // Wait
      '領域の選択', // Crosshair
      'テキストの選択', // IBeam
    ])
  })

  it('limit=6 でも included が 3 個しかなければ 3 セルだけ描画する', () => {
    const w = mount(CursorMatrix, {
      props: { included: ['Wait', 'Arrow', 'IBeam'], limit: 6 },
      global: { stubs: { CursorIcon: true } },
    })
    const cells = w.findAll('.cell')
    expect(cells).toHaveLength(3)
    // 入力順 (Wait, Arrow, IBeam) ではなく canonical 順 (Arrow → Wait → IBeam) で並ぶ
    const titles = cells.map((c) => c.attributes('title'))
    expect(titles).toEqual(['通常の選択', '待ち状態', 'テキストの選択'])
  })

  it('cols=3 を渡すと .cols-3 クラスが付与される (3x2 レイアウト)', () => {
    const w = mount(CursorMatrix, {
      props: { included: ['Arrow', 'IBeam', 'Wait'], limit: 6, cols: 3 },
      global: { stubs: { CursorIcon: true } },
    })
    expect(w.find('.cursors').classes()).toContain('cols-3')
    expect(w.find('.cursors').classes()).not.toContain('cols-6')
  })

  it('cols 未指定なら .cols-6 クラスが付与される (従来の 6x3)', () => {
    const w = mount(CursorMatrix, {
      props: { included: ['Arrow'] },
      global: { stubs: { CursorIcon: true } },
    })
    expect(w.find('.cursors').classes()).toContain('cols-6')
  })
})
