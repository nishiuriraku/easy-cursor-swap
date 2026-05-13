import { describe, expect, it } from 'vitest'
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

describe('SettingsSearchDropdown', () => {
  it('shows "no results" message when results empty and query non-empty', () => {
    const w = mount(SettingsSearchDropdown, {
      props: { results: [], overflowCount: 0, activeIndex: 0, query: 'xyz' },
    })
    // ロケールに依存せず、ja/en のいずれかの no-result 文言が出ていること
    const txt = w.text()
    expect(txt.includes('該当する設定がありません') || txt.includes('No matching settings')).toBe(
      true,
    )
  })

  it('renders one row per result', () => {
    const results = [mkResult('a', 'AAA'), mkResult('b', 'BBB'), mkResult('c', 'CCC')]
    const w = mount(SettingsSearchDropdown, {
      props: { results, overflowCount: 0, activeIndex: 0, query: 'a' },
    })
    expect(w.findAll('[role="option"]')).toHaveLength(3)
  })

  it('marks activeIndex row with aria-selected', () => {
    const results = [mkResult('a', 'AAA'), mkResult('b', 'BBB')]
    const w = mount(SettingsSearchDropdown, {
      props: { results, overflowCount: 0, activeIndex: 1, query: 'a' },
    })
    const opts = w.findAll('[role="option"]')
    expect(opts[0]!.attributes('aria-selected')).toBe('false')
    expect(opts[1]!.attributes('aria-selected')).toBe('true')
  })

  it('emits select with entry when row is clicked', async () => {
    const r = mkResult('a', 'AAA')
    const w = mount(SettingsSearchDropdown, {
      props: { results: [r], overflowCount: 0, activeIndex: 0, query: 'a' },
    })
    await w.find('[role="option"]').trigger('mousedown')
    const emitted = w.emitted('select')
    expect(emitted).toBeTruthy()
    expect(emitted![0]![0]).toStrictEqual(r.entry)
  })

  it('shows overflow footer when overflowCount > 0', () => {
    const results = [mkResult('a', 'AAA')]
    const w = mount(SettingsSearchDropdown, {
      props: { results, overflowCount: 5, activeIndex: 0, query: 'a' },
    })
    expect(w.text()).toContain('5')
  })

  it('does not render anything when query is empty', () => {
    const w = mount(SettingsSearchDropdown, {
      props: { results: [], overflowCount: 0, activeIndex: 0, query: '' },
    })
    expect(w.find('[role="listbox"]').exists()).toBe(false)
  })
})
