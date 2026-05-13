import { describe, expect, it } from 'vitest'
import {
  CATALOG,
  lookupMessage,
  searchSettings,
  type SearchContext,
} from '~/composables/useSettingsSearch'
import ja from '~/locales/ja'
import en from '~/locales/en'

const ctxWithKey: SearchContext = { hasKeystore: true }
const ctxNoKey: SearchContext = { hasKeystore: false }

describe('CATALOG integrity', () => {
  it('(section, anchor) is unique', () => {
    const seen = new Set<string>()
    for (const e of CATALOG) {
      const key = `${e.section}:${e.anchor}`
      expect(seen.has(key), `duplicate ${key}`).toBe(false)
      seen.add(key)
    }
  })

  it('every labelKey resolves in ja and en', () => {
    for (const e of CATALOG) {
      expect(lookupMessage(ja, e.labelKey), `ja missing ${e.labelKey}`).toBeTypeOf('string')
      expect(lookupMessage(en, e.labelKey), `en missing ${e.labelKey}`).toBeTypeOf('string')
    }
  })

  it('every descKey (when present) resolves in ja and en', () => {
    for (const e of CATALOG) {
      if (!e.descKey) continue
      expect(lookupMessage(ja, e.descKey), `ja missing ${e.descKey}`).toBeTypeOf('string')
      expect(lookupMessage(en, e.descKey), `en missing ${e.descKey}`).toBeTypeOf('string')
    }
  })
})

describe('searchSettings', () => {
  it('returns empty array for empty query', () => {
    expect(searchSettings('', 'ja', ctxWithKey)).toEqual([])
    expect(searchSettings('   ', 'ja', ctxWithKey)).toEqual([])
  })

  it('matches Japanese label (UI=ja)', () => {
    const r = searchSettings('言語', 'ja', ctxWithKey)
    expect(r.some((x) => x.entry.section === 'general' && x.entry.anchor === 'language')).toBe(true)
  })

  it('matches English label even when UI=ja', () => {
    const r = searchSettings('language', 'ja', ctxWithKey)
    expect(r.some((x) => x.entry.section === 'general' && x.entry.anchor === 'language')).toBe(true)
  })

  it('matches Japanese label even when UI=en', () => {
    const r = searchSettings('言語', 'en', ctxWithKey)
    expect(r.some((x) => x.entry.section === 'general' && x.entry.anchor === 'language')).toBe(true)
  })

  it('is case-insensitive', () => {
    const lower = searchSettings('log', 'ja', ctxWithKey)
    const upper = searchSettings('LOG', 'ja', ctxWithKey)
    expect(lower.map((r) => r.entry.anchor)).toEqual(upper.map((r) => r.entry.anchor))
    expect(lower.length).toBeGreaterThan(0)
  })

  it('matches description text', () => {
    const r = searchSettings('INFO', 'ja', ctxWithKey)
    expect(r.some((x) => x.entry.section === 'logging' && x.entry.anchor === 'logLevel')).toBe(true)
  })

  it('returns no results for nonsense query', () => {
    expect(searchSettings('xyzzyzzy12345', 'ja', ctxWithKey)).toEqual([])
  })

  it('respects visible() guard (keystore=false hides keyId)', () => {
    const r = searchSettings('key_id', 'ja', ctxNoKey)
    expect(r.some((x) => x.entry.anchor === 'keyId')).toBe(false)
  })

  it('respects visible() guard (keystore=true hides generate)', () => {
    const r = searchSettings('鍵を生成', 'ja', ctxWithKey)
    expect(r.some((x) => x.entry.anchor === 'generate')).toBe(false)
  })

  it('result has displayLabel matching current UI locale', () => {
    const ja_r = searchSettings('language', 'ja', ctxWithKey).find(
      (x) => x.entry.anchor === 'language',
    )
    const en_r = searchSettings('language', 'en', ctxWithKey).find(
      (x) => x.entry.anchor === 'language',
    )
    expect(ja_r?.displayLabel).toBe('UI 言語')
    expect(en_r?.displayLabel).toMatch(/language/i)
  })
})
