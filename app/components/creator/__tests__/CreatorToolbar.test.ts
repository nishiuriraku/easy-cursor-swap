/**
 * CreatorToolbar テスト。
 *
 * パンくず + Clear ボタン + 署名状態タグ + Export ボタン。
 * Export ボタンは sign フラグ付き payload を emit するので、
 * 「sign=true / false の使い分け」を 2 つの異なるボタンで生成する点を確認。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorToolbar from '../CreatorToolbar.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
}

const baseProps = {
  metaName: 'Untitled',
  metaVersion: '1.0.0',
  hasKeystoreSigning: false,
  exportBusy: false,
  arrowAssigned: true,
}

describe('CreatorToolbar', () => {
  it('renders breadcrumb with theme name and version', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, metaName: 'MyTheme', metaVersion: '0.2.1' },
      global: { stubs },
    })
    const text = wrapper.text()
    expect(text).toContain('MyTheme')
    expect(text).toContain('v0.2.1')
  })

  it('shows "Untitled" placeholder when metaName empty', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, metaName: '' },
      global: { stubs },
    })
    expect(wrapper.text()).toContain('Untitled')
  })

  it('shows "unsigned" tag (rose) when no keystore', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, hasKeystoreSigning: false },
      global: { stubs },
    })
    // unsigned style attribute に rose 色を含む
    const tag = wrapper.findAll('.tag').find((t) => t.attributes('style')?.includes('rose'))
    expect(tag).toBeTruthy()
  })

  it('shows "ok" tag when keystore present', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, hasKeystoreSigning: true },
      global: { stubs },
    })
    const okTag = wrapper.find('.tag.ok')
    expect(okTag.exists()).toBe(true)
  })

  it('emits reset on clear button click', async () => {
    const wrapper = mount(CreatorToolbar, { props: baseProps, global: { stubs } })
    const clearBtn = wrapper.find('button[aria-label="クリアして初期画面に戻る"]')
    await clearBtn.trigger('click')
    expect(wrapper.emitted('reset')).toHaveLength(1)
  })

  it('emits export with sign=false on the ghost export button', async () => {
    const wrapper = mount(CreatorToolbar, { props: baseProps, global: { stubs } })
    // ghost export ボタン (.cursorpack export 無署名)
    const exportBtn = wrapper.findAll('button').find((b) => b.attributes('title') === '.cursorpack')
    expect(exportBtn).toBeTruthy()
    await exportBtn!.trigger('click')
    expect(wrapper.emitted('export')).toEqual([[{ sign: false }]])
  })

  it('emits export with sign=true on primary "Sign & Export" button', async () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, hasKeystoreSigning: true },
      global: { stubs },
    })
    const primary = wrapper.find('button.primary')
    await primary.trigger('click')
    expect(wrapper.emitted('export')).toEqual([[{ sign: true }]])
  })

  it('disables export button when arrowAssigned=false', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, arrowAssigned: false },
      global: { stubs },
    })
    const exportBtn = wrapper.findAll('button').find((b) => b.attributes('title') === '.cursorpack')
    expect(exportBtn?.attributes('disabled')).toBeDefined()
  })

  it('disables export when exportBusy=true', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, exportBusy: true },
      global: { stubs },
    })
    const exportBtn = wrapper.findAll('button').find((b) => b.attributes('title') === '.cursorpack')
    expect(exportBtn?.attributes('disabled')).toBeDefined()
  })
})
