/**
 * AboutSection コンポーネントテスト。
 *
 * 静的情報のみのセクション。外部リンク (homepage / issues) は Tauri 2 webview の
 * 制限上 `<a href target=_blank>` では動かないため、Rust 側 `open_url` IPC 経由で
 * 開く button にしている。テストではボタンが GitHub URL を引数に invoke を叩くか
 * を確認する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import AboutSection from '../AboutSection.vue'

// invokeTauri を spy できるよう mock。
// `get_app_info` だけは AboutSection の onMounted がバージョン取得に使うので
// 固定値を返し、それ以外 (open_url 等) は undefined を返す。
const invokeMock = vi.fn().mockImplementation((cmd: string) => {
  if (cmd === 'get_app_info') {
    return Promise.resolve({
      version: '0.1.0',
      cursors_dir: '',
      config_dir: '',
      os_version: 'Windows 10.0',
    })
  }
  return Promise.resolve(undefined)
})
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
    // get_app_info も onMounted で呼ばれるので open_url 呼び出しのみ抽出して検証する。
    const openUrlCalls = invokeMock.mock.calls.filter((c) => c[0] === 'open_url')
    expect(openUrlCalls).toHaveLength(2)
    for (const call of openUrlCalls) {
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

  it('shows version hint with v{version} pattern from get_app_info', async () => {
    const wrapper = mount(AboutSection, { global: { stubs } })
    // useAppInfo の load() が解決するのを待ってから DOM を見る。
    await flushPromises()
    const hint = wrapper.find('.head-hint')
    expect(hint.exists()).toBe(true)
    expect(hint.text()).toMatch(/v\d/)
    expect(hint.text()).toContain('MIT')
  })
})
