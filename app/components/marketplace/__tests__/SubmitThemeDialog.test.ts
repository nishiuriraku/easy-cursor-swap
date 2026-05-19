import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

const invokeMock = vi.fn()
const listenMock = vi.fn().mockResolvedValue(() => {})

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (...a: unknown[]) => invokeMock(...a),
  listenTauri: (...a: unknown[]) => listenMock(...a),
}))

vi.mock('~/components/icons/UiIcon.vue', () => ({
  default: { template: '<span data-test="icon" />' },
}))

vi.mock('../SubmitDeviceFlowModal.vue', () => ({
  default: { template: '<div data-test="device-flow-modal" />' },
}))

// i18n は日本語ロケールで固定する (happy-dom の navigator.language は en-US のため)
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

import SubmitThemeDialog from '../SubmitThemeDialog.vue'

function mountOpen() {
  return mount(SubmitThemeDialog, {
    props: { open: true },
    attachTo: document.body,
  })
}

describe('SubmitThemeDialog Auto tab', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    })
  })

  it('shows auto tab as default and renders submit mode label', async () => {
    // get_themes → []
    invokeMock.mockResolvedValueOnce([])
    // keystore_info
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    // get_config (no github_account)
    invokeMock.mockResolvedValueOnce({
      schema_version: 2,
      general: {
        auto_start: true,
        auto_update: true,
        language: 'ja',
        active_theme_id: null,
        panic_hotkey: 'x',
        crash_reporting: false,
        favorites: [],
        usage: {},
      },
      security: {
        max_pack_compressed_size: 1,
        max_pack_uncompressed_size: 1,
        max_image_file_size: 1,
        storage_warning_threshold: 1,
      },
      logging: { level: 'INFO', retention_days: 14, max_total_size: 1 },
      github_account: null,
    })
    const w = mountOpen()
    await flushPromises()
    // 自動タブが既定で active であり、submitMode ラベルが見える
    expect(document.body.textContent).toContain('提出モード')
    w.unmount()
  })

  it('shows "Link GitHub" CTA when no github_account is set', async () => {
    invokeMock.mockResolvedValueOnce([])
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    invokeMock.mockResolvedValueOnce({ github_account: null })
    const w = mountOpen()
    await flushPromises()
    expect(document.body.textContent).toMatch(/GitHub と連携/)
    w.unmount()
  })

  it('shows linked-as label when github_account is present', async () => {
    invokeMock.mockResolvedValueOnce([
      { id: 'aaa', name: 'X', author: 'me', version: '1.0', included_roles: [], is_active: false },
    ])
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    invokeMock.mockResolvedValueOnce({
      github_account: { login: 'octocat', token_saved_at: '2026-05-14T00:00:00Z' },
    })
    const w = mountOpen()
    await flushPromises()
    expect(document.body.textContent).toContain('octocat')
    w.unmount()
  })

  it('renders localized theme name in select options instead of [object Object]', async () => {
    // 回帰テスト: Rust 側 `ThemeSummary.name` は `LocalizedString` で
    // localized object (`{ja, en, ...}`) が JSON 化されて届く。
    // UiSelect の option label で `${th.name}` を生展開すると `"[object Object] (v...)"`
    // に化けるため、`pickLocalizedName(th.name, locale)` を介して解決する必要がある。
    invokeMock.mockResolvedValueOnce([
      {
        id: 'aaa',
        name: { ja: 'ローカライズドテーマ', en: 'Localized Theme' },
        author: 'me',
        version: '1.0',
        included_roles: [],
        is_active: false,
      },
    ])
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    invokeMock.mockResolvedValueOnce({ github_account: null })
    const w = mountOpen()
    await flushPromises()

    // UiSelect の listbox を開いて option を露出させる
    const trigger = document.body.querySelector('#submit-auto-theme') as HTMLButtonElement | null
    expect(trigger).toBeTruthy()
    trigger!.click()
    await flushPromises()

    const body = document.body.textContent ?? ''
    expect(body).toContain('ローカライズドテーマ')
    expect(body).not.toContain('[object Object]')
    w.unmount()
  })

  it('switches to Manual tab when clicked', async () => {
    invokeMock.mockResolvedValueOnce([])
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    invokeMock.mockResolvedValueOnce({ github_account: null })
    const w = mountOpen()
    await flushPromises()
    const tabs = document.body.querySelectorAll('.tab')
    const manualTab = Array.from(tabs).find((t) => t.textContent?.includes('手動'))
    expect(manualTab).toBeTruthy()
    ;(manualTab as HTMLButtonElement).click()
    await flushPromises()
    // 手動タブのコンテンツは「ローカルライブラリのテーマを公式インデックスへ申請できます」(submitHint)
    expect(document.body.textContent).toContain(
      'ローカルライブラリのテーマを公式インデックスへ申請',
    )
    w.unmount()
  })

  it('excludes marketplace-derived themes (source=marketplace AND clones) from submit options', async () => {
    // 公式インデックス由来テーマと「その複製 (cloned_from_marketplace_id 持ち)」は
    // 再提出禁止なので、Auto タブの選択肢に出てはいけない。Rust 側 submit_theme_auto も
    // 同条件で hard-reject するので、UI は誤操作を未然に防ぐ defense-in-depth として効く。
    invokeMock.mockResolvedValueOnce([
      {
        id: 'local-original',
        name: 'Local Original',
        author: 'me',
        version: '1.0',
        included_roles: [],
        is_active: false,
        // source 未指定 = local。提出可能。
      },
      {
        id: 'mkt-original',
        name: 'Marketplace Original',
        author: 'them',
        version: '1.0',
        included_roles: [],
        is_active: false,
        source: 'marketplace',
      },
      {
        id: 'mkt-clone',
        name: 'Cloned From Marketplace',
        author: 'me',
        version: '1.0',
        included_roles: [],
        is_active: false,
        cloned_from_marketplace_id: 'mkt-original',
      },
    ])
    invokeMock.mockResolvedValueOnce({ has_keypair: true, key_id: 'k', public_key_b64: 'p' })
    invokeMock.mockResolvedValueOnce({ github_account: null })
    const w = mountOpen()
    await flushPromises()

    const trigger = document.body.querySelector('#submit-auto-theme') as HTMLButtonElement | null
    expect(trigger).toBeTruthy()
    trigger!.click()
    await flushPromises()

    const body = document.body.textContent ?? ''
    expect(body).toContain('Local Original')
    expect(body).not.toContain('Marketplace Original')
    expect(body).not.toContain('Cloned From Marketplace')
    w.unmount()
  })
})
