/**
 * LibraryListHeader コンポーネントテスト (P08a Step 4 / V5)。
 *
 * `aria-sort` が active 列だけ ascending|descending であること、
 * click/Enter/Space で `sort` emit されることを確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibraryListHeader from '../LibraryListHeader.vue'

describe('LibraryListHeader', () => {
  it('sets aria-sort only on the active column', () => {
    const wrapper = mount(LibraryListHeader, { props: { sortKey: 'name', sortDir: 'asc' } })
    const headers = wrapper.findAll('[role="columnheader"]')
    const sortable = headers.filter((h) => h.classes().includes('lt-sortable'))
    expect(sortable).toHaveLength(3)
    expect(sortable[0]!.attributes('aria-sort')).toBe('ascending')
    expect(sortable[1]!.attributes('aria-sort')).toBe('none')
    expect(sortable[2]!.attributes('aria-sort')).toBe('none')
  })

  it('emits sort on click/Enter/Space', async () => {
    const wrapper = mount(LibraryListHeader, { props: { sortKey: 'updated', sortDir: 'desc' } })
    const size = wrapper.findAll('.lt-sortable')[2]!
    await size.trigger('click')
    expect(wrapper.emitted('sort')).toEqual([['size']])
    await size.trigger('keydown.enter')
    await size.trigger('keydown.space')
    expect(wrapper.emitted('sort')).toEqual([['size'], ['size'], ['size']])
  })
})
