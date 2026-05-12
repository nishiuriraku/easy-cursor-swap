/**
 * ThemeRow の署名バッジは signed === true のときだけ表示する。
 * 過去に `signed !== false` 判定だったため、signed が undefined のときも
 * 全テーマ "署名済" 扱いになる不具合があった。
 *
 * 注: Stage 3 で Library の署名列を撤去するため、このファイルは
 * 同 Stage で「.lt-sig セルが描画されない」仕様のテストに差し替える予定。
 * Stage 1 完了時点ではバッジ条件分岐そのものの厳密化を固定化する。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeRow from '../ThemeRow.vue'
import type { ThemeCardData } from '~/types/theme'
import jaLib from '~/locales/ja'

vi.mock('~/composables/useI18n', async () => {
  const ja = (await import('~/locales/ja')).default
  function resolveJa(key: string): string {
    const parts = key.split('.')
    let cursor: unknown = ja
    for (const p of parts) {
      if (typeof cursor !== 'object' || cursor === null) return key
      cursor = (cursor as Record<string, unknown>)[p]
    }
    return typeof cursor === 'string' ? cursor : key
  }
  return { useI18n: () => ({ t: (k: string) => resolveJa(k) }) }
})

const SIGNED_LABEL = (jaLib as unknown as { library: { sigSigned: string } }).library.sigSigned

function makeTheme(over: Partial<ThemeCardData>): ThemeCardData {
  return {
    id: 'id',
    name: 'Name',
    author: 'a',
    version: '1.0',
    date: '2026-05-01',
    applyCount: 0,
    isFavorite: false,
    isActive: false,
    includedRoles: ['Arrow'],
    ...over,
  }
}

describe('ThemeRow signed badge', () => {
  it('signed === true なら 署名バッジが出る', () => {
    const w = mount(ThemeRow, { props: { theme: makeTheme({ signed: true }) } })
    expect(w.text()).toContain(SIGNED_LABEL)
  })

  it('signed === false なら 署名バッジは出ない', () => {
    const w = mount(ThemeRow, { props: { theme: makeTheme({ signed: false }) } })
    expect(w.text()).not.toContain(SIGNED_LABEL)
  })

  it('signed が undefined でも 署名バッジは出ない (取りこぼし耐性)', () => {
    const w = mount(ThemeRow, { props: { theme: makeTheme({ signed: undefined }) } })
    expect(w.text()).not.toContain(SIGNED_LABEL)
  })
})
