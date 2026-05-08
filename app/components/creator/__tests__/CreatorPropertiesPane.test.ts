/**
 * CreatorPropertiesPane テスト。
 *
 * 右ペインの 4 つの v-model (hotspotX/Y/perSizeHotspot/shadowEnabled) と
 * read-only props (showAdvancedResolutions / importedPreviewUrl /
 * sanitizedRemovals / resample) の表示挙動を確認。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorPropertiesPane from '../CreatorPropertiesPane.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
}

const baseProps = {
  hotspotX: 4,
  hotspotY: 4,
  perSizeHotspot: false,
  shadowEnabled: true,
  showAdvancedResolutions: false,
  importedPreviewUrl: 'blob:fake',
  sanitizedRemovals: [] as string[],
  resample: 'lanczos' as const,
}

describe('CreatorPropertiesPane', () => {
  it('renders 3 prop sections (Hotspot/Asset/Validation)', () => {
    const wrapper = mount(CreatorPropertiesPane, { props: baseProps, global: { stubs } })
    const sections = wrapper.findAll('.prop-section')
    expect(sections).toHaveLength(3)
  })

  it('emits update:hotspotX when X input changes', async () => {
    const wrapper = mount(CreatorPropertiesPane, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input[type="number"]')
    await inputs[0]!.setValue('12')
    const emits = wrapper.emitted('update:hotspotX')
    expect(emits![0]).toEqual([12])
    expect(typeof emits![0]![0]).toBe('number')
  })

  it('emits update:hotspotY when Y input changes', async () => {
    const wrapper = mount(CreatorPropertiesPane, { props: baseProps, global: { stubs } })
    const inputs = wrapper.findAll('input[type="number"]')
    await inputs[1]!.setValue('20')
    expect(wrapper.emitted('update:hotspotY')).toEqual([[20]])
  })

  it('hides per-size toggle when showAdvancedResolutions=false', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, showAdvancedResolutions: false },
      global: { stubs },
    })
    // perSizeHotspot のトグルは Hotspot セクション内、advanced=false なら非表示
    const toggles = wrapper.findAll('.toggle')
    // Asset セクションの shadow トグルだけが見える
    expect(toggles).toHaveLength(1)
  })

  it('shows per-size toggle when showAdvancedResolutions=true', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, showAdvancedResolutions: true },
      global: { stubs },
    })
    const toggles = wrapper.findAll('.toggle')
    // perSizeHotspot トグルと shadow トグルの 2 つ
    expect(toggles).toHaveLength(2)
  })

  it('emits update:shadowEnabled when shadow toggle clicked', async () => {
    const wrapper = mount(CreatorPropertiesPane, { props: baseProps, global: { stubs } })
    // showAdvancedResolutions=false なので toggle は 1 つ (= shadow)
    await wrapper.find('.toggle').trigger('click')
    expect(wrapper.emitted('update:shadowEnabled')).toEqual([[false]])
  })

  it('shows magic-byte OK when importedPreviewUrl present', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, importedPreviewUrl: 'blob:abc' },
      global: { stubs },
    })
    const validationRows = wrapper.findAll('.vrow')
    expect(validationRows[0]!.text()).toContain('OK')
  })

  it('shows magic-byte dash when no preview', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, importedPreviewUrl: null },
      global: { stubs },
    })
    const validationRows = wrapper.findAll('.vrow')
    expect(validationRows[0]!.text()).toContain('—')
  })

  it('shows clean svg-sanitize when no removals', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, sanitizedRemovals: [] },
      global: { stubs },
    })
    const rows = wrapper.findAll('.vrow')
    expect(rows[1]!.text()).toContain('clean')
  })

  it('shows removed N when sanitizer stripped tags', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, sanitizedRemovals: ['<script>', '@onload', '@href'] },
      global: { stubs },
    })
    const rows = wrapper.findAll('.vrow')
    expect(rows[1]!.text()).toContain('removed 3')
  })

  it('reflects resample mode in validation row', () => {
    const wrapper = mount(CreatorPropertiesPane, {
      props: { ...baseProps, resample: 'nearest' },
      global: { stubs },
    })
    const rows = wrapper.findAll('.vrow')
    expect(rows[2]!.text()).toContain('nearest')
  })
})
