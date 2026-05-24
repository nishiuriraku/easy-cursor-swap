/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiButton from '../UiButton.vue'

const stubs = {
  UiIcon: {
    template: '<span data-testid="icon" :data-name="name"></span>',
    props: ['name', 'size'],
  },
}

describe('UiButton', () => {
  it('renders default variant', () => {
    const w = mount(UiButton, { slots: { default: 'Click me' }, global: { stubs } })
    expect(w.find('button').classes()).toContain('btn')
    expect(w.find('button').classes()).not.toContain('primary')
    expect(w.text()).toBe('Click me')
  })

  it('applies variant class', () => {
    const w = mount(UiButton, {
      props: { variant: 'primary' },
      slots: { default: 'Go' },
      global: { stubs },
    })
    expect(w.find('button').classes()).toContain('primary')
  })

  it('applies icon size class', () => {
    const w = mount(UiButton, { props: { size: 'icon', ariaLabel: 'close' }, global: { stubs } })
    expect(w.find('button').classes()).toContain('icon')
    expect(w.find('button').attributes('aria-label')).toBe('close')
  })

  it('renders icon left when iconLeft is set', () => {
    const w = mount(UiButton, {
      props: { iconLeft: 'Check' },
      slots: { default: 'OK' },
      global: { stubs },
    })
    const icons = w.findAll('[data-testid="icon"]')
    expect(icons[0].attributes('data-name')).toBe('Check')
  })

  it('replaces iconLeft with spinner when loading', () => {
    const w = mount(UiButton, {
      props: { iconLeft: 'Check', loading: true },
      slots: { default: 'OK' },
      global: { stubs },
    })
    expect(w.find('.spinner').exists()).toBe(true)
    expect(
      w.findAll('[data-testid="icon"]').filter((i) => i.attributes('data-name') === 'Check'),
    ).toHaveLength(0)
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })

  it('is disabled when disabled prop is true', () => {
    const w = mount(UiButton, {
      props: { disabled: true },
      slots: { default: 'X' },
      global: { stubs },
    })
    expect(w.find('button').attributes('disabled')).toBeDefined()
  })

  it('emits click when clicked', async () => {
    const w = mount(UiButton, { slots: { default: 'X' }, global: { stubs } })
    await w.find('button').trigger('click')
    expect(w.emitted('click')).toHaveLength(1)
  })

  it('does not emit click when disabled', async () => {
    const w = mount(UiButton, {
      props: { disabled: true },
      slots: { default: 'X' },
      global: { stubs },
    })
    await w.find('button').trigger('click')
    expect(w.emitted('click')).toBeUndefined()
  })
})
