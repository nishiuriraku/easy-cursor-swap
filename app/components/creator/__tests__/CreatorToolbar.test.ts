/**
 * CreatorToolbar テスト。
 *
 * パンくず + Clear ボタン + 署名状態タグ + Export ボタン。
 * Export ボタンは sign フラグ付き payload を emit するので、
 * 「sign=true / false の使い分け」を 2 つの異なるボタンで生成する点を確認。
 */
import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CreatorToolbar from '../CreatorToolbar.vue'
import { useI18n } from '~/composables/useI18n'

const stubs = {
  UiIcon: { template: '<span></span>' },
}

// happy-dom の navigator.language は en-US 既定。テストは ja の aria-label セレクタを
// 使うので、各テスト前に明示的に locale=ja に固定して i18n の出力を決定論的にする。
beforeEach(() => {
  useI18n().setLocale('ja')
})

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

  it('emits save on the Save button', async () => {
    const wrapper = mount(CreatorToolbar, { props: baseProps, global: { stubs } })
    const saveBtn = wrapper.find('button.primary')
    await saveBtn.trigger('click')
    expect(wrapper.emitted('save')).toHaveLength(1)
  })

  it('disables save when arrowAssigned=false', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, arrowAssigned: false },
      global: { stubs },
    })
    const saveBtn = wrapper.find('button.primary')
    expect(saveBtn.attributes('disabled')).toBeDefined()
  })

  it('disables save when exportBusy=true', () => {
    const wrapper = mount(CreatorToolbar, {
      props: { ...baseProps, exportBusy: true },
      global: { stubs },
    })
    const saveBtn = wrapper.find('button.primary')
    expect(saveBtn.attributes('disabled')).toBeDefined()
  })
})
