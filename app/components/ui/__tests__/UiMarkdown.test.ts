/**
 * @vitest-environment happy-dom
 *
 * UiMarkdown: marked Lexer + 自前 VNode レンダーのテスト。
 * - 見出し / リスト / 強調 / コード等の基本描画
 * - 生 HTML は要素化されずテキスト表示 (v-html 不使用の不変条件)
 * - javascript: リンクはアンカー化しない
 */
import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import UiMarkdown from '../UiMarkdown.vue'

const openExternalUrlMock = vi.fn()

vi.mock('../../../composables/useExternalUrl', () => ({
  openExternalUrl: (...args: unknown[]) => openExternalUrlMock(...args),
  useExternalUrl: () => ({ openExternalUrl: openExternalUrlMock }),
}))

function mountMd(source: string) {
  return mount(UiMarkdown, { props: { source } })
}

describe('UiMarkdown', () => {
  it('見出し・段落・リストを描画する', () => {
    const w = mountMd('### Added\n\n- foo\n- bar\n')
    expect(w.find('h3').exists()).toBe(true)
    expect(w.find('h3').text()).toBe('Added')
    const items = w.findAll('li')
    expect(items.map((li) => li.text())).toEqual(['foo', 'bar'])
  })

  it('強調・インラインコードを描画する', () => {
    const w = mountMd('**bold** と `code` です\n')
    expect(w.find('strong').text()).toBe('bold')
    expect(w.find('code').text()).toBe('code')
  })

  it('コードフェンスを pre > code で描画する', () => {
    const w = mountMd('```\nconst x = 1\n```\n')
    const pre = w.find('pre')
    expect(pre.exists()).toBe(true)
    expect(pre.text()).toContain('const x = 1')
  })

  it('生 HTML は要素化せずテキストとして表示する', () => {
    const w = mountMd('<script>alert(1)</script>\n')
    expect(w.find('script').exists()).toBe(false)
    expect(w.html()).toContain('&lt;script&gt;')
  })

  it('https リンクはアンカー化し、クリックで外部オープンする', async () => {
    openExternalUrlMock.mockReset()
    const w = mountMd('[example](https://example.com)\n')
    const a = w.find('a')
    expect(a.exists()).toBe(true)
    expect(a.attributes('href')).toBe('https://example.com')
    await a.trigger('click')
    expect(openExternalUrlMock).toHaveBeenCalledWith('https://example.com')
  })

  it('javascript: リンクはアンカー化しない', () => {
    const w = mountMd('[x](javascript:alert(1))\n')
    expect(w.find('a').exists()).toBe(false)
    expect(w.text()).toContain('x')
  })

  it('空文字でも壊れない', () => {
    const w = mountMd('')
    expect(w.find('.md-body').exists()).toBe(true)
  })
})
