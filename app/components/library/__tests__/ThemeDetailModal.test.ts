/**
 * ThemeDetailModal コンポーネントテスト。
 *
 * Drawer (Hero + Strip) を内包しつつ、フッターのアクション群を UiModal の
 * `#leftNote` / `#actions` slot に配置する構造を固定化する。
 *
 * - delete ボタンは theme.isActive のときに disabled / aria-label を切替
 * - apply / 適用中 の出し分け
 * - marketplace / system 由来のときの代替表示
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import ThemeDetailModal from '../ThemeDetailModal.vue'
import type { ThemeCardData } from '~/types/theme'

vi.mock('~/composables/useI18n', async () => {
  const ja = (await import('~/locales/ja')).default
  function resolveJa(key: string, params?: Record<string, string | number>): string {
    const parts = key.split('.')
    let cursor: unknown = ja
    for (const p of parts) {
      if (typeof cursor !== 'object' || cursor === null) return key
      cursor = (cursor as Record<string, unknown>)[p]
    }
    if (typeof cursor !== 'string') return key
    if (!params) return cursor
    return cursor.replace(/\{(\w+)\}/g, (_, k: string) =>
      params[k] === undefined ? `{${k}}` : String(params[k]),
    )
  }
  return { useI18n: () => ({ t: resolveJa }) }
})

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: vi.fn().mockResolvedValue(undefined),
}))

const stubs = {
  UiIcon: { template: '<span data-testid="icon"></span>', props: ['name', 'size'] },
  CursorIcon: { template: '<span data-testid="cursor-icon"></span>' },
  CursorPreview: {
    props: ['asset', 'hotspot', 'roleId', 'displayPct', 'fallbackIconSize', 'hideDot'],
    template: '<div data-testid="cursor-preview"></div>',
  },
}

const wrappers: VueWrapper[] = []
afterEach(() => {
  while (wrappers.length) wrappers.pop()!.unmount()
  document.body.style.overflow = ''
})

function makeTheme(overrides: Partial<ThemeCardData> = {}): ThemeCardData {
  return {
    id: '00000000-0000-0000-0000-000000000001',
    name: 'Sample',
    author: 'Tester',
    version: '1.0.0',
    date: '2026-05-12T00:00:00Z',
    applyCount: 3,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow', 'Hand'],
    kind: 'local',
    tags: [],
    sizeBytes: 2048,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: 1,
    license: null,
    homepage: null,
    ...overrides,
  }
}

function mountModal(theme: ThemeCardData | null) {
  const w = mount(ThemeDetailModal, {
    props: { theme, previewMap: null, previewDetails: null },
    global: { stubs },
    attachTo: document.body,
  })
  wrappers.push(w)
  return w
}

describe('ThemeDetailModal — フッターアクション (UiModal slots)', () => {
  it('isActive=true のとき削除ボタンが disabled', async () => {
    const w = mountModal(makeTheme({ isActive: true }))
    await w.vm.$nextTick()
    const deleteBtn = document.querySelector('.td-act.danger') as HTMLButtonElement | null
    expect(deleteBtn).not.toBeNull()
    expect(deleteBtn!.disabled).toBe(true)
  })

  it('isActive=true のとき削除ボタンの aria-label が「適用中のため削除できません」を含む', async () => {
    const w = mountModal(makeTheme({ isActive: true, name: 'TestTheme' }))
    await w.vm.$nextTick()
    const deleteBtn = document.querySelector('.td-act.danger') as HTMLButtonElement | null
    expect(deleteBtn?.getAttribute('aria-label')).toContain('適用中')
    expect(deleteBtn?.getAttribute('aria-label')).toContain('TestTheme')
  })

  it('isActive=false のとき削除ボタンは disabled でない', async () => {
    const w = mountModal(makeTheme({ isActive: false }))
    await w.vm.$nextTick()
    const deleteBtn = document.querySelector('.td-act.danger') as HTMLButtonElement | null
    expect(deleteBtn).not.toBeNull()
    expect(deleteBtn!.disabled).toBe(false)
  })

  it('isActive=true のとき #actions に「適用中」ボタン、isActive=false のとき「テーマを適用」', async () => {
    const wActive = mountModal(makeTheme({ isActive: true }))
    await wActive.vm.$nextTick()
    const actionsActive = Array.from(document.querySelectorAll('.modal-foot .actions button')).map(
      (b) => b.textContent?.trim(),
    )
    expect(actionsActive.some((t) => t?.includes('適用中'))).toBe(true)

    wActive.unmount()
    wrappers.pop()
    document.body.style.overflow = ''

    const wIdle = mountModal(makeTheme({ isActive: false }))
    await wIdle.vm.$nextTick()
    const actionsIdle = Array.from(document.querySelectorAll('.modal-foot .actions button')).map(
      (b) => b.textContent?.trim(),
    )
    expect(actionsIdle.some((t) => t?.includes('適用') && !t?.includes('適用中'))).toBe(true)
  })

  it('marketplace 由来テーマでは edit / export ボタンが非表示で marketplace hint が出る', async () => {
    const w = mountModal(makeTheme({ kind: 'marketplace' }))
    await w.vm.$nextTick()
    const acts = Array.from(document.querySelectorAll('.td-act')).map((b) => b.textContent?.trim())
    // 編集 / エクスポート の文言が出ていないこと
    expect(acts.some((t) => t?.includes('Creator で編集'))).toBe(false)
    // hint が出ている
    const hint = document.querySelector('.td-hint')
    expect(hint?.textContent).toContain('編集')
  })

  it('system scheme では export のみ、read-only ヒントが出る', async () => {
    const w = mountModal(makeTheme({ kind: 'system' }))
    await w.vm.$nextTick()
    const acts = Array.from(document.querySelectorAll('.td-act')).map(
      (b) => b.textContent?.trim() ?? '',
    )
    // 削除ボタンは描画されない
    expect(document.querySelector('.td-act.danger')).toBeNull()
    // export ボタンはある (system scheme は `.cursorpack に書き出し` ラベル)
    expect(acts.some((t) => t.includes('書き出し'))).toBe(true)
    // read-only 表示
    expect(document.querySelector('.td-source-readonly')).not.toBeNull()
  })

  it('theme=null のときモーダルは非表示', () => {
    mountModal(null)
    expect(document.querySelector('.modal-page')).toBeNull()
  })
})
