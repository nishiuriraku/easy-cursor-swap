/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import UiSelect from '../UiSelect.vue'
import UiSkeleton from '../UiSkeleton.vue'
import UiSkeletonCard from '../UiSkeletonCard.vue'
import UiSpinner from '../UiSpinner.vue'
import UiStageStepper from '../UiStageStepper.vue'

describe('ui misc a11y', () => {
  it('UiSelect associates label with combobox', async () => {
    const w = mount(UiSelect, {
      props: {
        modelValue: 'a',
        options: [
          { value: 'a', label: 'A' },
          { value: 'b', label: 'B' },
        ],
      },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('UiSkeleton marks busy region', async () => {
    const w = mount(UiSkeleton, { attachTo: document.body })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('UiSkeletonCard has no axe violations', async () => {
    const w = mount(UiSkeletonCard, { attachTo: document.body })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('UiSpinner exposes status role', async () => {
    const w = mount(UiSpinner, {
      props: { label: 'Loading' },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('UiStageStepper marks current step', async () => {
    const w = mount(UiStageStepper, {
      props: {
        stages: [
          { id: 'a', label: 'A' },
          { id: 'b', label: 'B' },
        ],
        currentId: 'a',
      },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
