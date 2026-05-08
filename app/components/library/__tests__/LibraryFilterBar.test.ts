/**
 * LibraryFilterBar の対話テスト。
 *
 * 4 つのフィルタチップとソートボタンが期待通り emit するか、
 * カウントバッジが props から正しく描画されるかを確認する。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LibraryFilterBar from '../LibraryFilterBar.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
}

const baseProps = {
  filter: 'all' as const,
  counts: { all: 17, favorites: 3, recent: 5, unsigned: 2 },
  sortLabel: 'By name',
}

describe('LibraryFilterBar', () => {
  it('renders 4 filter chips with counts', () => {
    const wrapper = mount(LibraryFilterBar, {
      props: baseProps,
      global: { stubs },
    })
    const chips = wrapper.findAll('.chip')
    expect(chips).toHaveLength(4)
    // counts.all → 最初のチップ末尾の <span class="num">
    const html = wrapper.html()
    expect(html).toContain('>17<') // counts.all
    expect(html).toContain('>3<') // favorites
    expect(html).toContain('>5<') // recent
    expect(html).toContain('>2<') // unsigned
  })

  it('marks the active filter chip with .active class', () => {
    const wrapper = mount(LibraryFilterBar, {
      props: { ...baseProps, filter: 'favorites' },
      global: { stubs },
    })
    const chips = wrapper.findAll('.chip')
    const activeChips = chips.filter((c) => c.classes().includes('active'))
    expect(activeChips).toHaveLength(1)
    // favorites は 2 番目 (index 1)
    expect(chips[1]!.classes()).toContain('active')
    expect(chips[1]!.attributes('aria-pressed')).toBe('true')
  })

  it('updates filter v-model when chip clicked', async () => {
    const wrapper = mount(LibraryFilterBar, {
      props: baseProps,
      global: { stubs },
    })
    // favorites チップ (index 1) をクリック
    await wrapper.findAll('.chip')[1]!.trigger('click')
    expect(wrapper.emitted('update:filter')).toEqual([['favorites']])
  })

  it('emits cycle-sort when sort button clicked', async () => {
    const wrapper = mount(LibraryFilterBar, {
      props: baseProps,
      global: { stubs },
    })
    const sortBtn = wrapper.find('.sort .btn')
    await sortBtn.trigger('click')
    expect(wrapper.emitted('cycle-sort')).toHaveLength(1)
  })

  it('renders the sortLabel text', () => {
    const wrapper = mount(LibraryFilterBar, {
      props: { ...baseProps, sortLabel: 'By usage' },
      global: { stubs },
    })
    expect(wrapper.find('.sort').text()).toContain('By usage')
  })

  it('does not emit anything on initial render', () => {
    const wrapper = mount(LibraryFilterBar, {
      props: baseProps,
      global: { stubs },
    })
    expect(wrapper.emitted('update:filter')).toBeUndefined()
    expect(wrapper.emitted('cycle-sort')).toBeUndefined()
  })

  it('clicking each of the 4 chips emits the matching filter id', async () => {
    const ids = ['all', 'favorites', 'recent', 'unsigned'] as const
    for (let i = 0; i < ids.length; i++) {
      // 同値再代入は defineModel が emit を抑制する仕様があるため、
      // クリック対象と異なる初期値でマウントする (= 必ず変化するクリックにする)。
      const initial = ids[(i + 1) % ids.length]
      const wrapper = mount(LibraryFilterBar, {
        props: { ...baseProps, filter: initial },
        global: { stubs },
      })
      await wrapper.findAll('.chip')[i]!.trigger('click')
      expect(wrapper.emitted('update:filter')).toEqual([[ids[i]]])
    }
  })
})
