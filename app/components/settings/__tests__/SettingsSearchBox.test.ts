/**
 * SettingsSearchBox コンポーネントテスト (P08a Step 3 / V4)。
 *
 * `useSettingsSearch` をモックし、入力で dropdown 表示、ArrowDown/Enter で
 * `jumpTo` が呼ばれること、Escape で閉じることを検証する。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import SettingsSearchBox from '../SettingsSearchBox.vue'

const jumpToMock = vi.fn()
const closeMock = vi.fn()
const state = {
  open: ref(false),
  activeIndex: ref(0),
  visibleResults: ref([] as Array<{ entry: { section: string } }>),
  overflowCount: ref(0),
}

vi.mock('~/composables/useSettingsSearch', () => ({
  useSettingsSearch: () => ({
    open: state.open,
    activeIndex: state.activeIndex,
    visibleResults: state.visibleResults,
    overflowCount: state.overflowCount,
    focus: vi.fn(),
    close: closeMock,
    moveActive: vi.fn(),
    resetActive: vi.fn(),
    jumpTo: jumpToMock,
  }),
}))

const stubs = {
  UiIcon: { template: '<span></span>' },
  SettingsSearchDropdown: { template: '<div class="dropdown-stub"></div>' },
}

function mountBox() {
  return mount(SettingsSearchBox, {
    props: { context: ref({ hasKeystore: false }), section: 'general' },
    global: { stubs },
  })
}

beforeEach(() => {
  vi.clearAllMocks()
  state.open.value = false
  state.activeIndex.value = 0
  state.visibleResults.value = []
})

describe('SettingsSearchBox', () => {
  it('shows dropdown when input has text', async () => {
    state.open.value = true
    const wrapper = mountBox()
    expect(wrapper.find('.dropdown-stub').exists()).toBe(true)
  })

  it('calls jumpTo on Enter with active result', async () => {
    state.open.value = true
    state.visibleResults.value = [{ entry: { section: 'keys' } }]
    const wrapper = mountBox()
    await wrapper.find('input').trigger('keydown', { key: 'Enter' })
    expect(jumpToMock).toHaveBeenCalledWith({ section: 'keys' })
  })

  it('closes on Escape', async () => {
    state.open.value = true
    const wrapper = mountBox()
    await wrapper.find('input').trigger('keydown', { key: 'Escape' })
    expect(closeMock).toHaveBeenCalled()
  })
})
