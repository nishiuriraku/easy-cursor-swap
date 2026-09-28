<script lang="ts">
/**
 * Markdown 安全描画コンポーネント (更新内容モーダル等)。
 *
 * - `marked` は Lexer (字句解析) のみに使い、自前の再帰レンダーで VNode 化する。
 * - HTML 文字列を一切生成しない (`v-html` 不使用の不変条件を維持)。
 *   `html` トークン (生 HTML / `<script>` 等) はテキストとして表示する。
 * - リンクは `http(s)://` / `mailto:` のみ外部ブラウザで開き、
 *   それ以外 (`javascript:` 等) はテキスト化する。
 */
import { Lexer } from 'marked'
import type { Token, Tokens } from 'marked'

const SAFE_HREF_RE = /^(https?:\/\/|mailto:)/i

function inlineText(token: Tokens.Text | Tokens.Escape): string {
  return token.text
}

export default defineComponent({
  name: 'UiMarkdown',
  props: {
    source: { type: String, default: '' },
  },
  setup(props) {
    function openLink(href: string): void {
      if (SAFE_HREF_RE.test(href.trim())) void openExternalUrl(href)
    }

    function renderInlineChildren(token: { tokens?: Token[] }, key: string): unknown[] {
      if (!token.tokens) return []
      return renderInline(token.tokens, key)
    }

    function renderInline(tokens: Token[], keyPrefix: string): unknown[] {
      const out: unknown[] = []
      tokens.forEach((token, i) => {
        const key = `${keyPrefix}-i${i}`
        switch (token.type) {
          case 'text':
          case 'escape': {
            const t = token as Tokens.Text | Tokens.Escape
            out.push(...(t.tokens ? renderInline(t.tokens, key) : [inlineText(t)]))
            break
          }
          case 'strong':
            out.push(h('strong', { key }, renderInlineChildren(token, key)))
            break
          case 'em':
            out.push(h('em', { key }, renderInlineChildren(token, key)))
            break
          case 'del':
            out.push(h('del', { key }, renderInlineChildren(token, key)))
            break
          case 'codespan':
            out.push(h('code', { key }, (token as Tokens.Codespan).text))
            break
          case 'br':
            out.push(h('br', { key }))
            break
          case 'link': {
            const t = token as Tokens.Link
            const children = renderInlineChildren(t, key)
            out.push(
              SAFE_HREF_RE.test(t.href.trim())
                ? h(
                    'a',
                    {
                      key,
                      href: t.href,
                      rel: 'noopener noreferrer',
                      onClick: (e: Event) => {
                        e.preventDefault()
                        openLink(t.href)
                      },
                    },
                    children.length > 0 ? children : [t.text],
                  )
                : h('span', { key }, children.length > 0 ? children : [t.text]),
            )
            break
          }
          case 'image': {
            // 画像は取得しない。alt テキストのみ表示する。
            const t = token as Tokens.Image
            const children = renderInlineChildren(t, key)
            out.push(h('span', { key }, children.length > 0 ? children : [t.text]))
            break
          }
          case 'html':
            // 生 HTML はテキストとして表示 (サニタイズ不要化)。
            out.push(h('span', { key }, (token as Tokens.HTML).text))
            break
          default:
            break
        }
      })
      return out
    }

    function renderBlocks(tokens: Token[], keyPrefix: string): unknown[] {
      const out: unknown[] = []
      tokens.forEach((token, i) => {
        const key = `${keyPrefix}-b${i}`
        switch (token.type) {
          case 'space':
          case 'def':
            break
          case 'heading': {
            const t = token as Tokens.Heading
            const depth = Math.min(Math.max(t.depth, 1), 6)
            out.push(h(`h${depth}`, { key }, renderInlineChildren(t, key)))
            break
          }
          case 'paragraph':
            out.push(h('p', { key }, renderInlineChildren(token, key)))
            break
          case 'text': {
            const t = token as Tokens.Text
            out.push(h('span', { key }, t.tokens ? renderInline(t.tokens, key) : [t.text]))
            break
          }
          case 'blockquote':
            out.push(
              h('blockquote', { key }, renderBlocks((token as Tokens.Blockquote).tokens, key)),
            )
            break
          case 'list': {
            const t = token as Tokens.List
            const tag = t.ordered ? 'ol' : 'ul'
            if (t.start !== '' && t.start !== 1 && tag === 'ol') {
              out.push(
                h(
                  tag,
                  { key, start: t.start },
                  t.items.map((item, j) => renderListItem(item, `${key}-li${j}`)),
                ),
              )
            } else {
              out.push(
                h(
                  tag,
                  { key },
                  t.items.map((item, j) => renderListItem(item, `${key}-li${j}`)),
                ),
              )
            }
            break
          }
          case 'code': {
            const t = token as Tokens.Code
            out.push(h('pre', { key }, [h('code', { key: `${key}-code` }, t.text)]))
            break
          }
          case 'hr':
            out.push(h('hr', { key }))
            break
          case 'table': {
            const t = token as Tokens.Table
            out.push(
              h('table', { key }, [
                h('thead', { key: `${key}-head` }, [
                  h(
                    'tr',
                    { key: `${key}-headrow` },
                    t.header.map((cell, c) =>
                      h(
                        'th',
                        { key: `${key}-h${c}`, align: t.align[c] ?? undefined },
                        renderInline(cell.tokens, `${key}-h${c}`),
                      ),
                    ),
                  ),
                ]),
                h(
                  'tbody',
                  { key: `${key}-body` },
                  t.rows.map((row, r) =>
                    h(
                      'tr',
                      { key: `${key}-r${r}` },
                      row.map((cell, c) =>
                        h(
                          'td',
                          { key: `${key}-r${r}c${c}`, align: t.align[c] ?? undefined },
                          renderInline(cell.tokens, `${key}-r${r}c${c}`),
                        ),
                      ),
                    ),
                  ),
                ),
              ]),
            )
            break
          }
          case 'html':
            out.push(h('p', { key }, [(token as Tokens.HTML).text]))
            break
          default:
            break
        }
      })
      return out
    }

    function renderListItem(item: Tokens.ListItem, key: string): unknown {
      const children = renderBlocks(item.tokens, key)
      if (item.task) {
        return h('li', { key, class: 'md-task' }, [
          h('input', {
            key: `${key}-box`,
            type: 'checkbox',
            checked: item.checked,
            disabled: true,
            'aria-hidden': 'true',
            tabindex: '-1',
          }),
          h('span', { key: `${key}-text` }, children),
        ])
      }
      return h('li', { key }, children)
    }

    return () => h('div', { class: 'md-body' }, renderBlocks(Lexer.lex(props.source ?? ''), 'md'))
  },
})
</script>

<style scoped>
@reference '~/assets/css/tailwind.css';

.md-body {
  @apply m-0 min-w-0 text-[12.5px] leading-[1.7] text-fg;
  word-break: break-word;
}
.md-body :is(h1, h2, h3, h4, h5, h6) {
  @apply font-bold text-fg;
  margin: 14px 0 6px;
  line-height: 1.4;
}
.md-body :is(h1, h2, h3, h4, h5, h6):first-child {
  margin-top: 0;
}
.md-body h1 {
  font-size: 16px;
}
.md-body h2 {
  font-size: 15px;
}
.md-body h3 {
  font-size: 13.5px;
}
.md-body :is(h4, h5, h6) {
  font-size: 12.5px;
}
.md-body p {
  margin: 0 0 8px;
}
.md-body p:last-child {
  margin-bottom: 0;
}
.md-body :is(ul, ol) {
  margin: 0 0 8px;
  padding-left: 20px;
}
.md-body ul {
  list-style: disc;
}
.md-body ol {
  list-style: decimal;
}
.md-body li {
  margin: 2px 0;
}
.md-body li > :is(ul, ol) {
  margin-bottom: 0;
}
.md-body li.md-task {
  list-style: none;
  margin-left: -20px;
  display: flex;
  gap: 6px;
  align-items: baseline;
}
.md-body blockquote {
  margin: 0 0 8px;
  padding: 6px 10px;
  border-left: 3px solid var(--line-strong);
  color: var(--fg-dim);
  background: rgba(255, 255, 255, 0.02);
  border-radius: 0 8px 8px 0;
}
.md-body pre {
  @apply font-mono;
  margin: 0 0 8px;
  padding: 8px 10px;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  font-size: 11.5px;
}
.md-body pre code {
  padding: 0;
  border: 0;
  background: transparent;
}
.md-body :not(pre) > code {
  @apply font-mono;
  padding: 1px 5px;
  border: 1px solid var(--line);
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.05);
  font-size: 11.5px;
}
.md-body a {
  color: rgb(var(--accent));
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
}
.md-body hr {
  margin: 12px 0;
  border: 0;
  border-top: 1px solid var(--line);
}
.md-body table {
  margin: 0 0 8px;
  border-collapse: collapse;
  font-size: 12px;
}
.md-body :is(th, td) {
  padding: 5px 9px;
  border: 1px solid var(--line);
  text-align: left;
}
.md-body th {
  font-weight: 600;
  background: rgba(255, 255, 255, 0.03);
}
</style>
