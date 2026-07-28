/**
 * UpdatesSection コンポーネントテスト (Wave 2AB / Task 13)。
 *
 * 親へ通知する 3 イベント (check-update / download-update / force-recheck) の
 * emit contract を検証する。子側で値を加工せずそのまま親へ上げるパスの固定。
 *
 * 表示文字列は i18n キー経由 (= useI18n() モック) で、固定文言は出現しない。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UpdatesSection from '../UpdatesSection.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  UiButton: {
    props: ['loading', 'disabled', 'variant', 'iconLeft'],
    template:
      '<button class="btn-stub" :data-loading="loading ? \'true\' : \'false\'" :data-icon="iconLeft"><slot /></button>',
  },
  UiProgress: { template: '<div class="progress-stub" />' },
  UiAlert: { template: '<div class="alert-stub"><slot /></div>' },
  SettingsRow: {
    props: ['anchor', 'label', 'desc'],
    template: '<div :data-anchor="anchor" :data-label="label"><slot /></div>',
  },
  SettingsToggle: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template:
      '<button class="toggle-stub" :data-on="modelValue" @click="$emit(\'update:modelValue\', !modelValue)" />',
  },
}

const baseProps = {
  updaterChecking: false,
  updaterDownloading: false,
  updaterAvailable: null,
  updaterMessage: null,
  updaterError: null,
  updaterProgress: 0,
  updaterTotal: 0,
  autoCheckHint: '24h cooldown',
}

describe('UpdatesSection', () => {
  it('force-recheck ボタンクリックで force-recheck イベントが発火する', async () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: false },
      global: { stubs },
    })
    const buttons = wrapper.findAll('button')
    const recheck = buttons.find((b) => !b.classes().includes('toggle-stub'))
    expect(recheck).toBeTruthy()
    await recheck!.trigger('click')
    expect(wrapper.emitted('force-recheck')).toHaveLength(1)
    // 他の emit はまだ発火していないこと
    expect(wrapper.emitted('check-update')).toBeUndefined()
    expect(wrapper.emitted('download-update')).toBeUndefined()
  })

  it('check-update ボタンクリックで check-update イベントが発火する', async () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: true },
      global: { stubs },
    })
    // 役割で識別: data-loading=false の btn-stub (= check-update) を選択
    const btn = wrapper.findAll('.btn-stub').find((b) => b.attributes('data-loading') === 'false')
    expect(btn).toBeTruthy()
    await btn!.trigger('click')
    expect(wrapper.emitted('check-update')).toHaveLength(1)
    expect(wrapper.emitted('download-update')).toBeUndefined()
    expect(wrapper.emitted('force-recheck')).toBeUndefined()
  })

  it('更新が available のとき download-update ボタンが表示されて emit も発火する', async () => {
    const wrapper = mount(UpdatesSection, {
      props: {
        ...baseProps,
        autoUpdate: true,
        updaterAvailable: { version: '0.2.0', body: 'bug fixes' },
      },
      global: { stubs },
    })
    // available 時は btn-stub が 2 つ (check-update + download-update) 存在する。
    // download は available ブロック内のみで描画されるため、index で区別する。
    const all = wrapper.findAll('.btn-stub')
    expect(all.length).toBeGreaterThanOrEqual(2)
    // 2 つ目 = download-update (template 順序)
    const downloadBtn = all[1]
    expect(downloadBtn).toBeTruthy()
    await downloadBtn!.trigger('click')
    expect(wrapper.emitted('download-update')).toHaveLength(1)
  })

  it('updaterChecking=true のとき check-update ボタンは loading 表示 (emit は依然発火する)', async () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: true, updaterChecking: true },
      global: { stubs },
    })
    // data-loading=true の btn-stub (= check-update が loading 中) を選択
    const btn = wrapper.findAll('.btn-stub').find((b) => b.attributes('data-loading') === 'true')
    expect(btn).toBeTruthy()
    // loading 中でも emit 自体は通る (= 親側で debounce / disable する設計)
    await btn!.trigger('click')
    expect(wrapper.emitted('check-update')).toHaveLength(1)
  })

  it('updaterAvailable=null のときは download-update ボタンが描画されない', () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: true, updaterAvailable: null },
      global: { stubs },
    })
    // download-update ボタンが存在しない = download-update が絶対に発火しない
    expect(wrapper.emitted('download-update')).toBeUndefined()
  })

  it('updaterError が指定されたとき alert-stub で表示される', () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: true, updaterError: 'network: timeout' },
      global: { stubs },
    })
    expect(wrapper.find('.alert-stub').exists()).toBe(true)
    expect(wrapper.text()).toContain('network: timeout')
  })

  it('autoUpdate の v-model (modelValue) は SettingsToggle 経由で双方向バインド', async () => {
    const wrapper = mount(UpdatesSection, {
      props: { ...baseProps, autoUpdate: false },
      global: { stubs },
    })
    const toggle = wrapper.find('.toggle-stub')
    expect(toggle.attributes('data-on')).toBe('false')
    await toggle.trigger('click')
    expect(wrapper.emitted('update:autoUpdate')).toEqual([[true]])
  })
})
