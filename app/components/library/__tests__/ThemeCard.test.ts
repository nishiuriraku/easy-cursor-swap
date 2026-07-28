/**
 * ThemeCard コンポーネントテスト (Wave 2AB / Task 13)。
 *
 * 親へ通知する 2 イベント (toggleFavorite / showDetails) の emit contract を検証する。
 * 詳細モーダルへの遷移とお気に入りトグルは useThemeCardState に集約されているが、
 * ここで固定するのは「子側で加工せず親へそのまま上げる」点 (= payload が theme.id と
 * 同じ string であること)。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeCard from '../ThemeCard.vue'
import type { ThemeCardData } from '~/types/theme'

const stubs = {
  UiIcon: { template: '<span></span>' },
  CursorMatrix: { template: '<div class="matrix-stub" />' },
}

const sampleTheme: ThemeCardData = {
  id: 'theme-uuid-1234',
  name: 'Sample',
  author: 'octocat',
  version: '1.0.0',
  date: '2026-05-10T00:00:00Z',
  applyCount: 3,
  isFavorite: false,
  isActive: false,
  kind: 'local',
  includedRoles: ['Arrow', 'Help'],
  tags: [],
  sizeBytes: 0,
  signed: false,
  description: null,
  schemaVersion: 1,
  license: null,
  homepage: null,
  lastAppliedAt: null,
  clonedFromMarketplaceId: null,
}

function mountWith(theme: Partial<ThemeCardData> = {}) {
  return mount(ThemeCard, {
    props: { theme: { ...sampleTheme, ...theme } },
    global: { stubs },
  })
}

describe('ThemeCard', () => {
  it('カード本体クリックで showDetails が theme.id 付きで発火する', async () => {
    const wrapper = mountWith()
    await wrapper.find('article').trigger('click')
    expect(wrapper.emitted('showDetails')).toEqual([['theme-uuid-1234']])
    expect(wrapper.emitted('toggleFavorite')).toBeUndefined()
  })

  it('カード本体で Enter キーを押すと showDetails が発火する', async () => {
    const wrapper = mountWith()
    await wrapper.find('article').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('showDetails')).toEqual([['theme-uuid-1234']])
  })

  it('お気に入りボタンクリックで toggleFavorite が theme.id 付きで発火する', async () => {
    const wrapper = mountWith({ isFavorite: false })
    const star = wrapper.find('button.star')
    expect(star.exists()).toBe(true)
    await star.trigger('click')
    expect(wrapper.emitted('toggleFavorite')).toEqual([['theme-uuid-1234']])
    // お気に入りボタンのクリックがカード本体に伝播しない (= showDetails 抑制)
    expect(wrapper.emitted('showDetails')).toBeUndefined()
  })

  it('isSystem=true のときはお気に入りボタンが描画されない (SYSTEM スキームはロック)', () => {
    const wrapper = mountWith({ kind: 'system' })
    expect(wrapper.find('button.star').exists()).toBe(false)
  })

  it('isActive=true のときは active-tag が表示される', () => {
    const wrapper = mountWith({ isActive: true })
    expect(wrapper.find('.card-active-tag').exists()).toBe(true)
  })

  it('kind="marketplace" のときは marketplace source-tag が表示される', () => {
    const wrapper = mountWith({ kind: 'marketplace' })
    expect(wrapper.find('.card-source-tag.marketplace').exists()).toBe(true)
  })

  it('signed=true のときは Shield アイコンが描画される', () => {
    const wrapper = mountWith({ signed: true })
    // stub の <span /> は複数あるが、meta-row 内 (.m.signed-stub ではなく) の Shield は
    // v-if="theme.signed" の中に存在することを確認
    const meta = wrapper.find('.meta-row')
    expect(meta.exists()).toBe(true)
    expect(meta.find('.tag.ok.featured-tag').exists()).toBe(true)
  })
})
