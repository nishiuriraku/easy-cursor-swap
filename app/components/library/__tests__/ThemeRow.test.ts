/**
 * ThemeRow から署名列を撤去したことを固定化する。
 *
 * 2026-05-13 の UI 簡略化で「Ed25519 で署名された」というジャーゴンを
 * 一般ユーザー向け画面から除去する一環として、Library 行の `.lt-sig`
 * セルを撤去した (signed バッジは ThemeDetailDrawer 内の「公式」ピルに
 * 集約)。Stage 1 で追加した signed バッジ表示テストのリプレースメント。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeRow from '../ThemeRow.vue'
import type { ThemeCardData } from '~/types/theme'

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

describe('ThemeRow', () => {
  it('signed の値によらず .lt-sig セルを描画しない', () => {
    for (const signed of [true, false, undefined] as const) {
      const w = mount(ThemeRow, { props: { theme: makeTheme({ signed }) } })
      expect(w.find('.lt-sig').exists()).toBe(false)
    }
  })
})
