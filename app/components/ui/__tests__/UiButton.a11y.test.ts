/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import UiButton from '../UiButton.vue'

const stubs = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
}

describe('UiButton a11y', () => {
  it('text button has no axe violations', async () => {
    const w = mount(UiButton, {
      slots: { default: 'Apply' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('icon-only button requires aria-label', async () => {
    const w = mount(UiButton, {
      props: { size: 'icon', ariaLabel: 'Close' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
