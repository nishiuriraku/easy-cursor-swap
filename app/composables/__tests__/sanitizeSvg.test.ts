/**
 * sanitizeSvg のセキュリティテスト。
 *
 * Creator が外部から取り込む SVG にスクリプトやトラッカーが混入していても、
 * 確実に剥がれて安全な構造のみ残ることを保証する。
 *
 * happy-dom が DOMParser / XMLSerializer / Element API を提供するので、
 * vitest 環境で実 DOM と同じパスで動作テストできる。
 */
import { describe, it, expect } from 'vitest'
import { sanitizeSvg } from '../sanitizeSvg'

describe('sanitizeSvg', () => {
  describe('benign SVG', () => {
    it('passes a clean svg through with no removals', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H24V24H0z" fill="#ff0"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.removed).toHaveLength(0)
      expect(r.sanitized).toContain('<svg')
      expect(r.sanitized).toContain('<path')
      expect(r.sanitized).toContain('fill="#ff0"')
    })

    it('keeps standard structural tags', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="g"><stop offset="0%" /></linearGradient></defs><g><circle r="5"/><rect width="10" height="10"/></g></svg>'
      const r = sanitizeSvg(input)
      expect(r.removed).toHaveLength(0)
      expect(r.sanitized).toContain('<linearGradient')
      expect(r.sanitized).toContain('<stop')
      expect(r.sanitized).toContain('<circle')
      expect(r.sanitized).toContain('<rect')
    })
  })

  describe('script injection', () => {
    it('strips <script> tag', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><script>alert("xss")</script><path d="M0 0"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('<script')
      expect(r.sanitized).not.toContain('alert')
      expect(r.removed).toContain('<script>')
    })

    it('strips on* event attributes', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><circle onclick="evil()" r="5"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('onload')
      expect(r.sanitized).not.toContain('onclick')
      expect(r.sanitized).not.toContain('alert')
      expect(r.sanitized).not.toContain('evil')
      expect(r.removed.some((x) => x.includes('onload'))).toBe(true)
      expect(r.removed.some((x) => x.includes('onclick'))).toBe(true)
    })

    it('strips foreignObject (HTML escape hatch)', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><div xmlns="http://www.w3.org/1999/xhtml">html</div></foreignObject></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized.toLowerCase()).not.toContain('foreignobject')
      expect(r.sanitized).not.toContain('<div')
      expect(r.removed).toContain('<foreignobject>')
    })

    it('strips javascript: protocol in attributes', () => {
      // path の d 属性は無害だが、href / xlink:href は除去対象
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><a href="javascript:alert(1)"><circle r="5"/></a></svg>'
      const r = sanitizeSvg(input)
      // <a> は ALLOWED_TAGS にない → 除去 (子の circle も巻き添え)
      expect(r.sanitized).not.toContain('javascript:')
      expect(r.removed).toContain('<a>')
    })

    it('strips href attribute on allowed tags', () => {
      // <use> は許可されているが href は外部参照経路なので除去
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><use href="https://evil.example/x.svg"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('https://evil.example')
      expect(r.removed).toContain('@href')
    })

    it('strips xlink:href attribute', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><use xlink:href="#x"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('xlink:href')
      expect(r.removed.some((x) => x.includes('href'))).toBe(true)
    })

    it('strips data: URI in attribute values', () => {
      // url(data:...) を持つ fill は除去
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><circle r="5" fill="data:image/svg+xml;base64,PHN2ZyB4bWxucz0i..."/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('data:image')
      expect(r.removed.some((x) => x.startsWith('@fill'))).toBe(true)
    })

    it('strips vbscript: protocol', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><circle r="5" fill="vbscript:msgbox(1)"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('vbscript')
    })

    it('strips animate-based scripting (SMIL)', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><animate attributeName="x" values="0;1"/><set attributeName="y" to="10"/><circle r="5"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('<animate')
      expect(r.sanitized).not.toContain('<set')
      expect(r.removed).toContain('<animate>')
      expect(r.removed).toContain('<set>')
    })

    it('strips raster image (potential pixel-tracker)', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><image href="https://tracker.example/pixel.gif" width="1" height="1"/></svg>'
      const r = sanitizeSvg(input)
      expect(r.sanitized).not.toContain('<image')
      expect(r.removed).toContain('<image>')
    })
  })

  describe('error handling', () => {
    it('returns empty string for invalid svg root', () => {
      const r = sanitizeSvg('<not-svg/>')
      expect(r.sanitized).toBe('')
      expect(r.removed.length).toBeGreaterThan(0)
    })

    it('returns empty string for malformed xml', () => {
      const r = sanitizeSvg('<svg><<<>><invalid')
      // happy-dom はエラーを documentElement に格納するので
      // sanitized が空文字列か、root が svg でない判定で弾かれる。
      expect(r.sanitized).toBe('')
    })

    it('handles empty input gracefully', () => {
      const r = sanitizeSvg('')
      expect(r.sanitized).toBe('')
      expect(r.removed.length).toBeGreaterThan(0)
    })
  })

  describe('edge cases', () => {
    it('preserves nested groups and gradients', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><defs><radialGradient id="g"><stop offset="0%" stop-color="#fff"/><stop offset="100%" stop-color="#000"/></radialGradient></defs><g transform="translate(5,5)"><g><circle r="3"/></g></g></svg>'
      const r = sanitizeSvg(input)
      expect(r.removed).toHaveLength(0)
      expect(r.sanitized).toContain('<radialGradient')
      expect(r.sanitized).toContain('transform="translate(5,5)"')
    })

    it('preserves clipPath and mask', () => {
      const input =
        '<svg xmlns="http://www.w3.org/2000/svg"><clipPath id="c"><rect width="10" height="10"/></clipPath><mask id="m"><circle r="5"/></mask></svg>'
      const r = sanitizeSvg(input)
      expect(r.removed).toHaveLength(0)
      expect(r.sanitized).toContain('<clipPath')
      expect(r.sanitized).toContain('<mask')
    })

    it('case-insensitively matches forbidden attributes', () => {
      const input = '<svg xmlns="http://www.w3.org/2000/svg" OnLoad="alert(1)"/>'
      const r = sanitizeSvg(input)
      expect(r.sanitized.toLowerCase()).not.toContain('onload')
    })
  })
})
