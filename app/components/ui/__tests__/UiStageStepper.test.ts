/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiStageStepper from '../UiStageStepper.vue'

const stubs = {
  UiIcon: {
    template: '<span data-testid="icon" :data-name="name"></span>',
    props: ['name', 'size'],
  },
  UiSpinner: {
    template: '<span data-testid="spinner"></span>',
    props: ['size', 'label'],
  },
}

const STAGES = [
  { id: 'scan', label: 'Scan' },
  { id: 'parse', label: 'Parse' },
  { id: 'extract', label: 'Extract' },
]

describe('UiStageStepper', () => {
  it('renders all stage labels in order', () => {
    const w = mount(UiStageStepper, {
      props: { stages: STAGES, currentId: 'parse' },
      global: { stubs },
    })
    const items = w.findAll('.ui-stepper-item')
    expect(items.map((i) => i.find('.ui-stepper-label').text())).toEqual([
      'Scan',
      'Parse',
      'Extract',
    ])
  })

  it('marks prior stages done, current active, later pending', () => {
    const w = mount(UiStageStepper, {
      props: { stages: STAGES, currentId: 'parse' },
      global: { stubs },
    })
    const items = w.findAll('.ui-stepper-item')
    expect(items[0].classes()).toContain('done')
    expect(items[1].classes()).toContain('active')
    expect(items[2].classes()).toContain('pending')
  })

  it('shows a spinner on the active stage and a check on done stages', () => {
    const w = mount(UiStageStepper, {
      props: { stages: STAGES, currentId: 'parse' },
      global: { stubs },
    })
    const items = w.findAll('.ui-stepper-item')
    expect(items[0].find('[data-testid="icon"]').attributes('data-name')).toBe('Check')
    expect(items[1].find('[data-testid="spinner"]').exists()).toBe(true)
  })

  it('sets aria-current=step on the active stage only', () => {
    const w = mount(UiStageStepper, {
      props: { stages: STAGES, currentId: 'parse' },
      global: { stubs },
    })
    const items = w.findAll('.ui-stepper-item')
    expect(items[1].attributes('aria-current')).toBe('step')
    expect(items[0].attributes('aria-current')).toBeUndefined()
  })

  it('marks a failed stage with an X regardless of position', () => {
    const w = mount(UiStageStepper, {
      props: { stages: STAGES, currentId: 'parse', failedId: 'parse' },
      global: { stubs },
    })
    const items = w.findAll('.ui-stepper-item')
    expect(items[1].classes()).toContain('failed')
    expect(items[1].find('[data-testid="icon"]').attributes('data-name')).toBe('X')
  })
})
