/**
 * AboutSection コンポーネントテスト。
 *
 * 静的情報のみのセクション。外部リンク (homepage / issues) は Tauri 2 webview の
 * 制限上 `<a href target=_blank>` では動かないため、Rust 側 `open_url` IPC 経由で
 * 開く button にしている。テストではボタンが GitHub URL を引数に invoke を叩くか
 * を確認する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import AboutSection from '../AboutSection.vue'

// invokeTauri を spy できるよう mock。
const invokeMock = vi.fn().mockResolvedValue(undefined)
vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeMock(...args),
}))

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>' },
  SettingsRow: {
    props: ['label', 'desc', 'mono'],
    template: '<div class="row" :data-label="label"><slot /></div>',
  },
}

describe('AboutSection', () => {
  beforeEach(() => invokeMock.mockClear())

  it('renders header with i18n title and description', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const header = wrapper.find('.section-head')
    expect(header.exists()).toBe(true)
    expect(header.find('h1').text().length).toBeGreaterThan(0)
  })

  it('clicking external buttons invokes open_url with github.com URL', async () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    // OSS ライセンス + 2 外部リンク = 3 button
    const buttons = wrapper.findAll('button')
    expect(buttons.length).toBeGreaterThanOrEqual(3)

    // 最初の 2 button が GitHub リンク (homepage / issues)
    for (let i = 0; i < 2; i++) {
      await buttons[i]!.trigger('click')
    }
    expect(invokeMock).toHaveBeenCalledTimes(2)
    for (const call of invokeMock.mock.calls) {
      expect(call[0]).toBe('open_url')
      const url = (call[1] as { url: string }).url
      expect(url).toMatch(/^https:\/\/github\.com\//)
    }
  })

  it('renders OSS license button', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    // 外部リンク 2 個 + OSS ライセンス 1 個 = 3 button
    const buttons = wrapper.findAll('button')
    expect(buttons.length).toBe(3)
  })

  it('shows version hint with v{version} pattern', () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    const hint = wrapper.find('.head-hint')
    expect(hint.exists()).toBe(true)
    expect(hint.text()).toMatch(/v\d/)
    expect(hint.text()).toContain('MIT')
  })
})
