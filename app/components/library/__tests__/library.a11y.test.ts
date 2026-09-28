/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from 'vitest'
import { h } from 'vue'
import { mount } from '@vue/test-utils'
import { axe } from '~/test-setup/axe'
import LibraryToolbar from '../LibraryToolbar.vue'
import LibraryFilterBar from '../LibraryFilterBar.vue'

const stubs: Record<string, any> = {
  UiIcon: { template: '<span aria-hidden="true"></span>' },
  UiButton: { template: '<button><slot /></button>' },
  NuxtLink: {
    props: ['to', 'custom'],
    setup(props: { to: string; custom?: boolean }, { slots }: { slots: any }) {
      const navigate = () => {}
      return () =>
        h(
          'span',
          { 'data-testid': 'nuxt-link', 'data-to': props.to },
          slots.default?.({ navigate, isActive: false, isExactActive: false, href: props.to }),
        )
    },
  },
}

describe('library section a11y (index page landmarks)', () => {
  it('LibraryToolbar has no axe violations', async () => {
    const w = mount(LibraryToolbar, {
      props: { searchQuery: '' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })

  it('LibraryFilterBar has no axe violations', async () => {
    const w = mount(LibraryFilterBar, {
      props: { filter: 'all', counts: { all: 1, favorites: 0, recent: 0 }, sortLabel: 'By name' },
      global: { stubs },
      attachTo: document.body,
    })
    expect(await axe(w.element)).toHaveNoViolations()
    w.unmount()
  })
})
