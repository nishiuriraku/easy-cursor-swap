import { afterEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import SettingsSearchDropdown from '../SettingsSearchDropdown.vue'
import type { SearchResult } from '~/composables/useSettingsSearch'

function mkResult(anchor: string, label: string, section = 'General'): SearchResult {
  return {
    entry: { section: 'general', anchor, labelKey: `settings.${anchor}Label` },
    displayLabel: label,
    displaySectionLabel: section,
  }
}

/**
 * 本コンポーネントは `<Teleport to="body">` で body 直下に描画する。
 * vue-test-utils の wrapper は teleport 先の DOM を取り込まないため、
 * テストでは `document.body` 側を直接クエリする。`attachTo: document.body` で
 * unmount 時に DOM がクリーンアップされる。
 */
function mountDD(props: Record<string, unknown>) {
  return mount(SettingsSearchDropdown, {
    props: { anchorEl: document.body, ...props },
    attachTo: document.body,
  })
}

afterEach(() => {
  // Teleport 先に残ったノードを手動で掃除する (attachTo の unmount 時クリーンアップが
  // teleport された子要素まで届かない happy-dom の挙動への対策)。
  document.body.querySelectorAll('.search-dd').forEach((n) => n.remove())
})

describe('SettingsSearchDropdown', () => {
  it('shows "no results" message when results empty and query non-empty', () => {
    mountDD({ results: [], overflowCount: 0, activeIndex: 0, query: 'xyz' })
    const txt = document.body.textContent ?? ''
    expect(txt.includes('該当する設定がありません') || txt.includes('No matching settings')).toBe(
      true,
    )
  })

  it('renders one row per result', () => {
    const results = [mkResult('a', 'AAA'), mkResult('b', 'BBB'), mkResult('c', 'CCC')]
    mountDD({ results, overflowCount: 0, activeIndex: 0, query: 'a' })
    expect(document.body.querySelectorAll('[role="option"]').length).toBe(3)
  })

  it('marks activeIndex row with aria-selected', () => {
    const results = [mkResult('a', 'AAA'), mkResult('b', 'BBB')]
    mountDD({ results, overflowCount: 0, activeIndex: 1, query: 'a' })
    const opts = document.body.querySelectorAll('[role="option"]')
    expect(opts[0]!.getAttribute('aria-selected')).toBe('false')
    expect(opts[1]!.getAttribute('aria-selected')).toBe('true')
  })

  it('emits select with entry when row is clicked', () => {
    const r = mkResult('a', 'AAA')
    const w = mountDD({ results: [r], overflowCount: 0, activeIndex: 0, query: 'a' })
    const row = document.body.querySelector('[role="option"]') as HTMLElement | null
    expect(row).not.toBeNull()
    row!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }))
    const emitted = w.emitted('select')
    expect(emitted).toBeTruthy()
    expect(emitted![0]![0]).toStrictEqual(r.entry)
  })

  it('shows overflow footer when overflowCount > 0', () => {
    const results = [mkResult('a', 'AAA')]
    mountDD({ results, overflowCount: 5, activeIndex: 0, query: 'a' })
    expect(document.body.textContent ?? '').toContain('5')
  })

  it('does not render anything when query is empty', () => {
    mountDD({ results: [], overflowCount: 0, activeIndex: 0, query: '' })
    expect(document.body.querySelector('[role="listbox"]')).toBeNull()
  })
})
