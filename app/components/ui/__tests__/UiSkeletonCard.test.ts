/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiSkeletonCard from '../UiSkeletonCard.vue'

describe('UiSkeletonCard', () => {
  it('renders a card with shimmer, decorative', () => {
    const w = mount(UiSkeletonCard)
    expect(w.find('.card').classes()).toContain('skeleton')
    expect(w.find('.card').attributes('aria-hidden')).toBe('true')
  })

  it('defaults to 280px height', () => {
    const w = mount(UiSkeletonCard)
    expect(w.find('.card').attributes('style') ?? '').toContain('height: 280px')
  })

  it('accepts a custom height', () => {
    const w = mount(UiSkeletonCard, { props: { height: '120px' } })
    expect(w.find('.card').attributes('style') ?? '').toContain('height: 120px')
  })
})
