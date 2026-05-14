/**
 * ThemeDetailDrawer コンポーネントテスト。
 *
 * 静的要素整理リファクタ後の実データ駆動描画を固定化する:
 * - description / tags / signed / license / homepage / lastAppliedAt
 *   の各フィールドが「無いときは描画しない」「あるときだけ描画する」
 * - VERSION/changelog セクションと SIGNATURE strip セルが完全削除されている
 * - PACKAGE は schema_version + sizeBytes、USAGE は lastAppliedAt 追記
 */
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeDetailDrawer from '../ThemeDetailDrawer.vue'
import type { ThemeCardData } from '~/types/theme'

// `vi.mock` のファクトリは hoist されるため、ja リソースの import も
// `vi.hoisted` 経由で巻き上げる。`t(key, params)` で {var} 展開まで再現する。
vi.mock('~/composables/useI18n', async () => {
  const ja = (await import('~/locales/ja')).default
  function resolveJa(key: string, params?: Record<string, string | number>): string {
    const parts = key.split('.')
    let cursor: unknown = ja
    for (const p of parts) {
      if (typeof cursor !== 'object' || cursor === null) return key
      cursor = (cursor as Record<string, unknown>)[p]
    }
    if (typeof cursor !== 'string') return key
    if (!params) return cursor
    return cursor.replace(/\{(\w+)\}/g, (_, k: string) =>
      params[k] === undefined ? `{${k}}` : String(params[k]),
    )
  }
  return {
    useI18n: () => ({ t: resolveJa }),
  }
})

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: vi.fn().mockResolvedValue(undefined),
}))

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  CursorIcon: { template: '<span data-testid="cursor-icon"></span>' },
  CursorPreview: { template: '<div data-testid="cursor-preview"></div>' },
}

function makeTheme(overrides: Partial<ThemeCardData> = {}): ThemeCardData {
  return {
    id: '00000000-0000-0000-0000-000000000001',
    name: 'Sample',
    author: 'Tester',
    version: '1.0.0',
    date: '2026-05-12T00:00:00Z',
    applyCount: 3,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow', 'Hand'],
    kind: 'local',
    tags: [],
    sizeBytes: 2048,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: 1,
    license: null,
    homepage: null,
    ...overrides,
  }
}

function mountDrawer(theme: ThemeCardData) {
  return mount(ThemeDetailDrawer, {
    props: { theme, previewMap: null, previewDetails: null },
    global: { stubs },
  })
}

describe('ThemeDetailDrawer — 静的要素の整理', () => {
  it('description が null のとき本文段落を描画しない', () => {
    const w = mountDrawer(makeTheme({ description: null }))
    expect(w.find('.td-desc').exists()).toBe(false)
  })

  it('description が文字列のとき本文を描画する', () => {
    const w = mountDrawer(makeTheme({ description: 'テーマ説明' }))
    expect(w.find('.td-desc').text()).toBe('テーマ説明')
  })

  it('tags が空 + signed=false のとき tag 行ごと描画しない', () => {
    const w = mountDrawer(makeTheme({ tags: [], signed: false }))
    expect(w.find('.td-tags').exists()).toBe(false)
  })

  it('signed=true のとき tag 行に signed pill を描画する', () => {
    const w = mountDrawer(makeTheme({ signed: true }))
    expect(w.find('.td-tag-on').exists()).toBe(true)
  })

  it('theme.tags 配列をそのまま chips として描画する', () => {
    const w = mountDrawer(makeTheme({ tags: ['dark', 'minimal'] }))
    const chips = w.findAll('.td-tag').filter((c) => !c.classes('td-tag-on'))
    expect(chips.map((c) => c.text())).toEqual(['dark', 'minimal'])
  })

  it('VERSION/changelog セクションを描画しない', () => {
    const w = mountDrawer(makeTheme())
    expect(w.find('.td-changelog').exists()).toBe(false)
  })

  it('SIGNATURE strip セルを描画しない (PACKAGE / USAGE / SOURCE の 3 セル構成)', () => {
    const w = mountDrawer(makeTheme({ signed: true }))
    const cellKeys = w
      .findAll('.td-cell-k')
      .map((el) => el.text().trim())
      .filter((t) => t.length > 0)
    expect(cellKeys).not.toContain('SIGNATURE')
    expect(cellKeys).toEqual(['PACKAGE', 'USAGE', 'SOURCE'])
  })

  it('PACKAGE セルに roles・サイズ (v) と schema v{n} (sub) を表示', () => {
    const w = mountDrawer(makeTheme({ schemaVersion: 1, sizeBytes: 2 * 1024 * 1024 }))
    const pkg = w.findAll('.td-cell')[0]!
    expect(pkg.find('.td-cell-v').text()).toContain('roles')
    expect(pkg.find('.td-cell-v').text()).toContain('2.0 MB')
    expect(pkg.find('.td-cell-sub').text()).toContain('schema v1')
  })

  it('lastAppliedAt があれば USAGE サブに「lastAppliedPrefix YYYY-MM-DD」を出す', () => {
    const w = mountDrawer(makeTheme({ applyCount: 5, lastAppliedAt: '2026-05-10T12:00:00Z' }))
    const usage = w.findAll('.td-cell')[1]!
    // 実 ja リソースを解決して描画される接頭辞 (「前回」) と日付の双方を確認。
    expect(usage.find('.td-cell-sub').text()).toContain('前回')
    expect(usage.find('.td-cell-sub').text()).toContain('2026-05-10')
  })

  it('license / homepage が null のとき SOURCE サブには version しか出さない', () => {
    const w = mountDrawer(makeTheme({ license: null, homepage: null }))
    const src = w.findAll('.td-cell')[2]!
    expect(src.find('.td-cell-sub').text()).toContain('v1.0.0')
    expect(src.find('.td-cell-sub').text()).not.toContain('·')
  })

  it('homepage があると openHomepage ボタンを描画', () => {
    const w = mountDrawer(makeTheme({ homepage: 'https://example.test' }))
    const src = w.findAll('.td-cell')[2]!
    expect(src.find('button.td-pane-link').exists()).toBe(true)
  })

  it('system scheme は description フォールバックを出す', () => {
    const w = mountDrawer(makeTheme({ kind: 'system', description: null }))
    expect(w.find('.td-desc').text()).toContain('Windows のマウスのプロパティ')
  })
})
