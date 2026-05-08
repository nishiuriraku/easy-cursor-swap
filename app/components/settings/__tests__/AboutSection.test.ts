/**
 * AboutSection コンポーネントテスト。
 *
 * 静的情報のみのセクション。リンク先 (homepage / issues) が安全な GitHub URL を
 * 指していることと、target="_blank" + rel="noopener" でフィッシング保護されている
 * ことを確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import AboutSection from '../AboutSection.vue'

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  SettingsRow: {
    props: ['label', 'desc', 'mono'],
    template: '<div class="row" :data-label="label"><slot /></div>',
  },
}

describe('AboutSection', () => {
  it('renders header with i18n title and description', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const header = wrapper.find('.section-head')
    expect(header.exists()).toBe(true)
    // h1 に title が入る (ja: 'About', en: 'About')
    expect(header.find('h1').text().length).toBeGreaterThan(0)
  })

  it('renders all external links pointing to github.com', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const links = wrapper.findAll('a')
    expect(links.length).toBeGreaterThanOrEqual(2)
    for (const a of links) {
      const href = a.attributes('href') ?? ''
      expect(href).toMatch(/^https:\/\/github\.com\//)
    }
  })

  it('opens external links with target=_blank and rel=noopener', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const links = wrapper.findAll('a')
    for (const a of links) {
      // フィッシング/タブハイジャック対策
      expect(a.attributes('target')).toBe('_blank')
      expect(a.attributes('rel')).toBe('noopener')
    }
  })

  it('renders OSS license button', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    // 唯一の button (リンクではない) は OSS ライセンス表示
    const buttons = wrapper.findAll('button')
    expect(buttons).toHaveLength(1)
  })

  it('shows version hint with v{version} pattern', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const hint = wrapper.find('.head-hint')
    expect(hint.exists()).toBe(true)
    // 'v1.0.0' のようなパターンが含まれる (MIT License も)
    expect(hint.text()).toMatch(/v\d/)
    expect(hint.text()).toContain('MIT')
  })
})
