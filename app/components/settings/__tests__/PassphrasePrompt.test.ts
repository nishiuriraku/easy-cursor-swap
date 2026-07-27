/**
 * PassphrasePrompt コンポーネントテスト (Wave 2AB / Task 13)。
 *
 * 親へ通知する 2 イベント (update:open / confirm) の emit contract を検証する。
 * confirm() は 8 文字以上 (export 時は確認入力一致) を満たしたときのみ発火する契約の固定。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'

// useI18n をモック。日本語ロケールを実 import して本物の翻訳文字列で検証する。
vi.mock('~/composables/useI18n', async () => {
  const ja = (await import('~/locales/ja')).default
  function resolveKey(obj: unknown, path: string): string | undefined {
    const parts = path.split('.')
    let cursor: unknown = obj
    for (const p of parts) {
      if (typeof cursor !== 'object' || cursor === null) return undefined
      cursor = (cursor as Record<string, unknown>)[p]
    }
    return typeof cursor === 'string' ? cursor : undefined
  }
  function t(key: string, params?: Record<string, string | number>): string {
    const resolved = resolveKey(ja, key)
    if (!resolved) return key
    if (!params) return resolved
    return resolved.replace(/\{(\w+)\}/g, (_, k: string) =>
      params[k] !== undefined ? String(params[k]) : `{${k}}`,
    )
  }
  return {
    useI18n: () => ({ t, locale: { value: 'ja' }, setLocale: () => {}, syncFromConfig: () => {} }),
  }
})

import PassphrasePrompt from '../PassphrasePrompt.vue'

const stubs = {
  UiModal: {
    props: ['open'],
    emits: ['close'],
    template:
      '<div class="modal-stub" :data-open="open"><slot /><div class="modal-actions-stub"><slot name="actions" /></div></div>',
  },
  UiIcon: { template: '<span></span>' },
  UiButton: {
    props: ['variant', 'iconLeft', 'disabled'],
    template:
      '<button class="btn-stub" :data-disabled="disabled ? \'true\' : \'false\'"><slot /></button>',
  },
}

function mountWith(props: Record<string, unknown>) {
  return mount(PassphrasePrompt, {
    props: { mode: 'export', open: true, ...props },
    global: { stubs },
  })
}

describe('PassphrasePrompt', () => {
  it('open=true でモーダルが表示される', () => {
    const wrapper = mountWith({ open: true })
    expect(wrapper.find('.modal-stub').attributes('data-open')).toBe('true')
  })

  it('open=false でモーダルが閉じる', () => {
    const wrapper = mountWith({ open: false })
    expect(wrapper.find('.modal-stub').attributes('data-open')).toBe('false')
  })

  it('8 文字未満の入力では confirm は発火しない', async () => {
    const wrapper = mountWith({ mode: 'export', open: true })
    const inputs = wrapper.findAll('input')
    await inputs[0]!.setValue('short') // 5 chars
    await inputs[1]!.setValue('short')
    // primary ボタンは disabled、押下しても confirm は発火しない
    const btn = wrapper.findAll('.btn-stub').find((b) => b.text().includes('エクスポート'))
    expect(btn?.attributes('data-disabled')).toBe('true')
    await btn!.trigger('click')
    expect(wrapper.emitted('confirm')).toBeUndefined()
  })

  it('export モード: 一致する 8 文字以上で confirm イベントが payload と共に発火', async () => {
    const wrapper = mountWith({ mode: 'export', open: true })
    const inputs = wrapper.findAll('input')
    await inputs[0]!.setValue('correct horse battery staple')
    await inputs[1]!.setValue('correct horse battery staple')
    const btn = wrapper.findAll('.btn-stub').find((b) => b.text().includes('エクスポート'))
    expect(btn?.attributes('data-disabled')).toBe('false')
    await btn!.trigger('click')
    expect(wrapper.emitted('confirm')).toEqual([['correct horse battery staple']])
    // confirm 直後は update:open(false) も連鎖 (= モーダルが閉じる)
    expect(wrapper.emitted('update:open')).toEqual([[false]])
  })

  it('export モード: 確認入力が不一致なら confirm は発火しない', async () => {
    const wrapper = mountWith({ mode: 'export', open: true })
    const inputs = wrapper.findAll('input')
    await inputs[0]!.setValue('password1234')
    await inputs[1]!.setValue('password5678')
    const btn = wrapper.findAll('.btn-stub').find((b) => b.text().includes('エクスポート'))
    expect(btn?.attributes('data-disabled')).toBe('true')
    await btn!.trigger('click')
    expect(wrapper.emitted('confirm')).toBeUndefined()
  })

  it('import モード: 確認入力欄は出ない (1 入力のみ)', () => {
    const wrapper = mountWith({ mode: 'import', open: true })
    expect(wrapper.findAll('input')).toHaveLength(1)
  })

  it('import モード: 8 文字以上で confirm 発火', async () => {
    const wrapper = mountWith({ mode: 'import', open: true })
    const input = wrapper.find('input')
    await input.setValue('import-pp-1234')
    const btn = wrapper.findAll('.btn-stub').find((b) => b.text().includes('インポート'))
    expect(btn?.attributes('data-disabled')).toBe('false')
    await btn!.trigger('click')
    expect(wrapper.emitted('confirm')).toEqual([['import-pp-1234']])
  })

  it('cancel ボタンクリックで update:open(false) のみ発火し confirm は出ない', async () => {
    const wrapper = mountWith({ mode: 'export', open: true })
    const cancel = wrapper.findAll('.btn-stub').find((b) => b.text().includes('キャンセル'))
    expect(cancel).toBeTruthy()
    await cancel!.trigger('click')
    expect(wrapper.emitted('update:open')).toEqual([[false]])
    expect(wrapper.emitted('confirm')).toBeUndefined()
  })

  it('open が true に戻ったとき入力欄がクリアされる', async () => {
    const wrapper = mountWith({ mode: 'export', open: true })
    const inputs = wrapper.findAll('input')
    await inputs[0]!.setValue('leftover-1234')
    await inputs[1]!.setValue('leftover-1234')
    // open=false → true でリセット
    await wrapper.setProps({ open: false })
    await wrapper.setProps({ open: true })
    const fresh = wrapper.findAll('input')
    expect((fresh[0]!.element as HTMLInputElement).value).toBe('')
    expect((fresh[1]!.element as HTMLInputElement).value).toBe('')
  })
})
