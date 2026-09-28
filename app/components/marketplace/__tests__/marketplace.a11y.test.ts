/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import SubmitThemeDialog from '../SubmitThemeDialog.vue'

const stubs = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
  UiButton: { template: '<button><slot /></button>' },
  UiModal: { template: '<div role="dialog"><slot /></div>' },
}

describe('marketplace section a11y', () => {
  it('SubmitThemeDialog has no axe violations', async () => {
    const w = mount(SubmitThemeDialog, {
      props: { open: true },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
