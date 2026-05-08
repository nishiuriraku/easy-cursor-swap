/**
 * CreatorRoleList テスト。
 *
 * 17 役割リストボックスの emit 配線とキーボードイベント転送を確認する。
 * RoleListItem は子の関心ごとなので stub に置き換える。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorRoleList from '../CreatorRoleList.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  RoleListItem: {
    props: ['role', 'index', 'status', 'active'],
    emits: ['select'],
    template:
      '<button class="role-stub" :data-role="role.id" :data-status="status" :data-active="active" @click="$emit(\'select\', role.id)">{{role.id}}</button>',
  },
}

const baseProps = {
  filledCount: 11,
  activeRoleId: 'Arrow',
  statusOf: (id: string) => (id === 'Arrow' ? 'filled' : 'empty') as 'filled' | 'empty',
}

describe('CreatorRoleList', () => {
  it('renders all 17 roles', () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    const items = wrapper.findAll('.role-stub')
    expect(items).toHaveLength(17)
  })

  it('passes activeRoleId as active=true to matching role', () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    const arrow = wrapper.find('[data-role="Arrow"]')
    expect(arrow.attributes('data-active')).toBe('true')
    const help = wrapper.find('[data-role="Help"]')
    expect(help.attributes('data-active')).toBe('false')
  })

  it('shows filledCount/17 in pane head', () => {
    const wrapper = mount(CreatorRoleList, {
      props: { ...baseProps, filledCount: 7 },
      global: { stubs },
    })
    expect(wrapper.find('.tag').text()).toContain('7 / 17')
  })

  it('forwards statusOf result via prop', () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    expect(wrapper.find('[data-role="Arrow"]').attributes('data-status')).toBe('filled')
    expect(wrapper.find('[data-role="Help"]').attributes('data-status')).toBe('empty')
  })

  it('emits select when a role is clicked', async () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    await wrapper.find('[data-role="Hand"]').trigger('click')
    expect(wrapper.emitted('select')).toEqual([['Hand']])
  })

  it('emits keydown event from listbox container', async () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    const list = wrapper.find('.role-list')
    await list.trigger('keydown', { key: 'ArrowDown' })
    expect(wrapper.emitted('keydown')).toBeTruthy()
    expect(wrapper.emitted('keydown')!.length).toBe(1)
    const ev = wrapper.emitted('keydown')![0]![0] as KeyboardEvent
    expect(ev.key).toBe('ArrowDown')
  })

  it('marks listbox container with role=listbox', () => {
    const wrapper = mount(CreatorRoleList, { props: baseProps, global: { stubs } })
    const list = wrapper.find('.role-list')
    expect(list.attributes('role')).toBe('listbox')
  })
})
