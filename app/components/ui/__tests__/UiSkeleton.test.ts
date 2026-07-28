/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import UiSkeleton from '../UiSkeleton.vue'

describe('UiSkeleton', () => {
  it('renders the shared .skeleton shimmer, decorative', () => {
    const w = mount(UiSkeleton)
    expect(w.find('.skeleton').exists()).toBe(true)
    expect(w.find('.skeleton').attributes('aria-hidden')).toBe('true')
  })

  it('applies width / height / radius', () => {
    const w = mount(UiSkeleton, { props: { width: '120px', height: '20px', radius: '4px' } })
    const style = w.find('.skeleton').attributes('style') ?? ''
    expect(style).toContain('width: 120px')
    expect(style).toContain('height: 20px')
    expect(style).toContain('border-radius: 4px')
  })

  it('renders a pill when circle=true', () => {
    const w = mount(UiSkeleton, { props: { circle: true } })
    const style = w.find('.skeleton').attributes('style') ?? ''
    expect(style).toContain('border-radius: 999px')
  })
})
