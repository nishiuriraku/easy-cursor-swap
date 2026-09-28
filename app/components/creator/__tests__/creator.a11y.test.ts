/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import CreatorToolbar from '../CreatorToolbar.vue'

const stubs = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
  UiButton: { template: '<button><slot /></button>' },
}

describe('creator section a11y', () => {
  it('CreatorToolbar has no axe violations', async () => {
    const w = mount(CreatorToolbar, {
      props: {
        metaName: 'T',
        metaVersion: '1.0.0',
        hasKeystoreSigning: false,
        exportBusy: false,
        arrowAssigned: true,
      },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
