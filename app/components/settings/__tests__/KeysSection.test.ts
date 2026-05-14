/**
 * KeysSection コンポーネントテスト。
 *
 * Ed25519 鍵ペア表示 + GitHub 連携状態の表示確認。
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import KeysSection from '../KeysSection.vue'

const stubs = {
  UiIcon: { template: '<span></span>' },
  SettingsRow: {
    props: ['anchor', 'label', 'desc', 'mono'],
    template: '<div :data-anchor="anchor" :data-label="label"><slot /></div>',
  },
}

const baseKeystoreInfo = {
  has_keypair: false,
  key_id: null,
  public_key_b64: null,
}

describe('KeysSection', () => {
  it('shows linked-as label when githubAccount is set', () => {
    const w = mount(KeysSection, {
      props: {
        keystoreInfo: { has_keypair: true, key_id: 'k', public_key_b64: 'p' },
        keystoreBusy: false,
        keystoreError: null,
        keystoreMessage: null,
        githubAccount: { login: 'octocat', token_saved_at: '2026-05-14T00:00:00Z' },
      },
      global: { stubs },
    })
    expect(w.text()).toContain('octocat')
  })

  it('shows unlinked label when githubAccount is null', () => {
    const w = mount(KeysSection, {
      props: {
        keystoreInfo: baseKeystoreInfo,
        keystoreBusy: false,
        keystoreError: null,
        keystoreMessage: null,
        githubAccount: null,
      },
      global: { stubs },
    })
    // ja.ts: 'GitHub と未連携' / en.ts: 'No GitHub account linked'
    expect(w.text()).toMatch(/GitHub と未連携|No GitHub account linked/)
  })

  it('emits github-unlink when Unlink button is clicked', async () => {
    const w = mount(KeysSection, {
      props: {
        keystoreInfo: baseKeystoreInfo,
        keystoreBusy: false,
        keystoreError: null,
        keystoreMessage: null,
        githubAccount: { login: 'octocat', token_saved_at: '2026-05-14T00:00:00Z' },
      },
      global: { stubs },
    })
    // Find the unlink button (last button in the component)
    const buttons = w.findAll('button')
    const unlinkBtn = buttons.find((b) => b.text().match(/連携を解除|Unlink/))
    expect(unlinkBtn).toBeTruthy()
    await unlinkBtn!.trigger('click')
    expect(w.emitted('github-unlink')).toHaveLength(1)
  })

  it('renders generate button when no keypair', () => {
    const w = mount(KeysSection, {
      props: {
        keystoreInfo: baseKeystoreInfo,
        keystoreBusy: false,
        keystoreError: null,
        keystoreMessage: null,
        githubAccount: null,
      },
      global: { stubs },
    })
    const buttons = w.findAll('button')
    expect(buttons.some((b) => b.text().match(/鍵を生成|Generate/))).toBe(true)
  })
})
