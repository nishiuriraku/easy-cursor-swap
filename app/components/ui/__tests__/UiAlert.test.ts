/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiAlert from '../UiAlert.vue'

const stubs = { UiIcon: { template: '<span data-testid="icon" :data-name="name"></span>', props: ['name', 'size'] } }

describe('UiAlert', () => {
  it('renders with role="alert"', () => {
    const w = mount(UiAlert, { props: { tone: 'info' }, slots: { default: 'hi' }, global: { stubs } })
    expect(w.attributes('role')).toBe('alert')
  })

  it('applies tone class', () => {
    const w = mount(UiAlert, { props: { tone: 'warn' }, slots: { default: 'hi' }, global: { stubs } })
    expect(w.classes()).toContain('ui-alert')
    expect(w.classes()).toContain('warn')
  })

  it('renders title when provided', () => {
    const w = mount(UiAlert, {
      props: { tone: 'danger', title: 'Danger!' },
      slots: { default: 'body' },
      global: { stubs },
    })
    expect(w.find('strong').text()).toBe('Danger!')
    expect(w.text()).toContain('body')
  })

  it('shows default icon for tone=warn (AlertTriangle)', () => {
    const w = mount(UiAlert, { props: { tone: 'warn' }, slots: { default: '.' }, global: { stubs } })
    expect(w.find('[data-testid="icon"]').attributes('data-name')).toBe('AlertTriangle')
  })

  it('shows default icon for tone=danger (Alert)', () => {
    const w = mount(UiAlert, { props: { tone: 'danger' }, slots: { default: '.' }, global: { stubs } })
    expect(w.find('[data-testid="icon"]').attributes('data-name')).toBe('Alert')
  })

  it('uses override icon when icon prop is set', () => {
    const w = mount(UiAlert, { props: { tone: 'info', icon: 'Pkg' }, slots: { default: '.' }, global: { stubs } })
    expect(w.find('[data-testid="icon"]').attributes('data-name')).toBe('Pkg')
  })

  it('hides icon when icon=false', () => {
    const w = mount(UiAlert, { props: { tone: 'info', icon: false }, slots: { default: '.' }, global: { stubs } })
    expect(w.find('[data-testid="icon"]').exists()).toBe(false)
  })

  it('applies dense class when dense=true', () => {
    const w = mount(UiAlert, { props: { tone: 'info', dense: true }, slots: { default: '.' }, global: { stubs } })
    expect(w.classes()).toContain('dense')
  })
})
