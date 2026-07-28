/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiSpinner from '../UiSpinner.vue'

describe('UiSpinner', () => {
  it('renders the shared .spinner ring', () => {
    const w = mount(UiSpinner)
    expect(w.find('.spinner').exists()).toBe(true)
  })

  it('applies size as inline width/height', () => {
    const w = mount(UiSpinner, { props: { size: 13 } })
    const style = w.find('.spinner').attributes('style') ?? ''
    expect(style).toContain('width: 13px')
    expect(style).toContain('height: 13px')
  })

  it('is decorative (aria-hidden) when no label', () => {
    const w = mount(UiSpinner)
    expect(w.find('.spinner').attributes('aria-hidden')).toBe('true')
    expect(w.find('.spinner').attributes('role')).toBeUndefined()
  })

  it('announces via role=status + aria-label when label is set', () => {
    const w = mount(UiSpinner, { props: { label: 'Loading' } })
    expect(w.find('.spinner').attributes('role')).toBe('status')
    expect(w.find('.spinner').attributes('aria-label')).toBe('Loading')
    expect(w.find('.spinner').attributes('aria-hidden')).toBeUndefined()
  })
})
