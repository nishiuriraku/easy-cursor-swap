/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiProgress from '../UiProgress.vue'

function fillStyle(w: ReturnType<typeof mount>): string {
  return w.find('.ui-progress-fill').attributes('style') ?? ''
}

describe('UiProgress', () => {
  it('computes percentage width from value/max', () => {
    const w = mount(UiProgress, { props: { value: 3, max: 4 } })
    expect(fillStyle(w)).toContain('width: 75%')
    expect(w.find('.ui-progress-track').attributes('aria-valuenow')).toBe('75')
  })

  it('clamps above 100%', () => {
    const w = mount(UiProgress, { props: { value: 200, max: 100 } })
    expect(fillStyle(w)).toContain('width: 100%')
    expect(w.find('.ui-progress-track').attributes('aria-valuenow')).toBe('100')
  })

  it('clamps below 0%', () => {
    const w = mount(UiProgress, { props: { value: -5, max: 100 } })
    expect(fillStyle(w)).toContain('width: 0%')
  })

  it('treats max<=0 as indeterminate (no width, no aria-valuenow)', () => {
    const w = mount(UiProgress, { props: { value: 0, max: 0 } })
    expect(w.find('.ui-progress-fill').classes()).toContain('indeterminate')
    expect(w.find('.ui-progress-track').attributes('aria-valuenow')).toBeUndefined()
    expect(w.find('.ui-progress-track').attributes('aria-valuemax')).toBeUndefined()
  })

  it('honors the indeterminate prop', () => {
    const w = mount(UiProgress, { props: { value: 50, max: 100, indeterminate: true } })
    expect(w.find('.ui-progress-fill').classes()).toContain('indeterminate')
    expect(w.find('.ui-progress-track').attributes('aria-valuenow')).toBeUndefined()
  })

  it('shows percent text only in determinate mode', () => {
    const w = mount(UiProgress, { props: { value: 1, max: 2, showPercent: true } })
    expect(w.find('.ui-progress-pct').text()).toBe('50%')
    const ind = mount(UiProgress, { props: { max: 0, showPercent: true } })
    expect(ind.find('.ui-progress-pct').exists()).toBe(false)
  })

  it('renders a label, used as aria-label fallback', () => {
    const w = mount(UiProgress, { props: { value: 1, max: 4, label: 'Importing' } })
    expect(w.find('.ui-progress-label').text()).toBe('Importing')
    expect(w.find('.ui-progress-track').attributes('aria-label')).toBe('Importing')
  })

  it('renders default slot over label', () => {
    const w = mount(UiProgress, { props: { value: 1, max: 4 }, slots: { default: 'Slotted' } })
    expect(w.find('.ui-progress-label').text()).toBe('Slotted')
  })
})
