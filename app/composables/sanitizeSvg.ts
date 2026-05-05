/**
 * 軽量 SVG サニタイザー (依存ゼロ)。
 *
 * クリエイターモードで読み込む SVG カーソル原稿のために、
 * `<script>` / `on*` 属性 / 外部参照 (`href`) / `<foreignObject>` を除去する。
 *
 * NOTE: 将来 DOMPurify を導入する場合はこのファイルを差し替える。
 *       現状は Tauri バイナリサイズを抑えるため最小実装。
 */

const ALLOWED_TAGS = new Set([
  'svg', 'g', 'path', 'circle', 'rect', 'ellipse', 'polygon', 'polyline',
  'line', 'defs', 'linearGradient', 'radialGradient', 'stop', 'clipPath',
  'mask', 'use', 'title', 'desc', 'metadata', 'style',
])

/** 外部 URL を運ぶ可能性がある属性は完全に除去 */
const FORBIDDEN_ATTRS = new Set([
  'href', 'xlink:href', 'src', 'srcset',
])

const FORBIDDEN_ATTR_PREFIXES = ['on'] // onload, onclick, etc.

const FORBIDDEN_TAGS = new Set([
  'script', 'foreignObject', 'image', 'iframe', 'object', 'embed',
  'animate', 'animateMotion', 'animateTransform', 'set', // SMIL アニメ経由のスクリプト実行
])

export interface SvgSanitizeResult {
  sanitized: string
  removed: string[]
}

/**
 * `svgString` をパースし、安全な要素のみで再構築した文字列を返す。
 * 失敗時 (パース不可) は空文字列。
 */
export function sanitizeSvg(svgString: string): SvgSanitizeResult {
  if (typeof DOMParser === 'undefined') {
    return { sanitized: '', removed: ['DOMParser unavailable'] }
  }

  const removed: string[] = []
  const parser = new DOMParser()
  const doc = parser.parseFromString(svgString, 'image/svg+xml')

  // パースエラー判定
  const parserErr = doc.querySelector('parsererror')
  if (parserErr) {
    return { sanitized: '', removed: [`SVG parse error: ${parserErr.textContent ?? ''}`] }
  }

  const svg = doc.documentElement
  if (!svg || svg.tagName.toLowerCase() !== 'svg') {
    return { sanitized: '', removed: ['root element is not <svg>'] }
  }

  walk(svg, removed)

  // XMLSerializer は標準で使用可。doctype 等は破棄される。
  const serializer = new XMLSerializer()
  return { sanitized: serializer.serializeToString(svg), removed }
}

function walk(el: Element, removed: string[]) {
  // 子から先に処理 (削除しても親のループが安定するように逆順)
  const children = Array.from(el.children)
  for (const child of children) {
    const tag = child.tagName.toLowerCase()
    if (FORBIDDEN_TAGS.has(tag) || !ALLOWED_TAGS.has(tag)) {
      removed.push(`<${tag}>`)
      child.remove()
      continue
    }
    walk(child, removed)
  }

  // 属性
  const attrs = Array.from(el.attributes)
  for (const attr of attrs) {
    const name = attr.name.toLowerCase()
    if (FORBIDDEN_ATTRS.has(name)) {
      removed.push(`@${name}`)
      el.removeAttribute(attr.name)
      continue
    }
    if (FORBIDDEN_ATTR_PREFIXES.some((p) => name.startsWith(p))) {
      removed.push(`@${name}`)
      el.removeAttribute(attr.name)
      continue
    }
    // url(...) 内の javascript: / data: を弾く
    if (typeof attr.value === 'string' && /^\s*(javascript|vbscript|data):/i.test(attr.value)) {
      removed.push(`@${name}=${attr.value.slice(0, 32)}...`)
      el.removeAttribute(attr.name)
    }
  }
}
