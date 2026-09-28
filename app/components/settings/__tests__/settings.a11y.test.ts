/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import StartupSection from '../StartupSection.vue'
import SettingsToggle from '../SettingsToggle.vue'

const stubs = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
  SettingsRow: {
    props: ['label', 'desc'],
    template: '<div :data-label="label"><slot /></div>',
  },
}

describe('settings section a11y', () => {
  it('StartupSection has no axe violations', async () => {
    const w = mount(StartupSection, {
      props: { autoStart: true, startMinimized: false, isMsixPackaged: false },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('SettingsToggle exposes pressed state', async () => {
    const w = mount(SettingsToggle, {
      props: { modelValue: true, label: 'Auto start' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
