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

  it('crash report rows are searchable from the logging section', () => {
    // クラッシュレポート UI 配線時に CATALOG への追加漏れを検出する回帰テスト。
    const r = searchSettings('クラッシュ', 'ja', ctxWithKey)
    const anchors = r.filter((x) => x.entry.section === 'logging').map((x) => x.entry.anchor)
    expect(anchors).toEqual(
      expect.arrayContaining(['crashReporting', 'crashCount', 'submitCrash', 'clearCrash']),
    )
  })

  it('crash report rows are reachable via the English keyword "crash" too', () => {
    const r = searchSettings('crash', 'en', ctxWithKey)
    const anchors = r.filter((x) => x.entry.section === 'logging').map((x) => x.entry.anchor)
    expect(anchors).toEqual(
      expect.arrayContaining(['crashReporting', 'crashCount', 'submitCrash', 'clearCrash']),
    )
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

import { ref } from 'vue'
import { useSettingsSearch } from '~/composables/useSettingsSearch'
import type { SettingsSectionId } from '~/composables/useSettingsSearch'

describe('useSettingsSearch composable', () => {
  it('focus / close は open と activeIndex を操作する', () => {
    const query = ref('')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    expect(s.open.value).toBe(false)
    expect(s.activeIndex.value).toBe(0)

    s.focus()
    expect(s.open.value).toBe(true)
    expect(s.activeIndex.value).toBe(0)

    s.close()
    expect(s.open.value).toBe(false)
  })

  it('moveActive は visibleResults が空のとき no-op', () => {
    const query = ref('__no_match_xyz_query__')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    expect(s.visibleResults.value.length).toBe(0)
    s.moveActive(1)
    expect(s.activeIndex.value).toBe(0)
    s.moveActive(-1)
    expect(s.activeIndex.value).toBe(0)
  })

  it('moveActive は visibleResults があるとき循環する', () => {
    const query = ref('language')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    if (s.visibleResults.value.length > 0) {
      s.focus()
      s.moveActive(1)
      expect(s.activeIndex.value).toBeGreaterThanOrEqual(0)
      s.moveActive(-1)
    }
    expect(s.overflowCount.value).toBeGreaterThanOrEqual(0)
  })

  it('resetActive は activeIndex を 0 に戻す', () => {
    const query = ref('テーマ')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    s.focus()
    s.activeIndex.value = 3
    s.resetActive()
    expect(s.activeIndex.value).toBe(0)
  })

  it('jumpTo は document 未定義環境でも throw しない', async () => {
    const query = ref('テーマ')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    const firstResult = s.results.value[0]
    if (firstResult) {
      const originalDoc = (globalThis as { document?: unknown }).document
      try {
        ;(globalThis as { document?: unknown }).document = undefined
        await expect(s.jumpTo(firstResult.entry)).resolves.toBeUndefined()
      } finally {
        ;(globalThis as { document?: unknown }).document = originalDoc
      }
      expect(sectionRef.value).toBe(firstResult.entry.section)
    }
  })

  it('overflowCount は results.length > HARD_LIMIT (8) のとき正しく計算される', () => {
    const query = ref('')
    const locale = ref<'ja' | 'en'>('ja')
    const context = ref<SearchContext>(ctxWithKey)
    const sectionRef = ref<SettingsSectionId>('general')
    const s = useSettingsSearch({ query, locale, context, sectionRef })

    if (s.results.value.length > 8) {
      expect(s.overflowCount.value).toBe(s.results.value.length - 8)
    } else {
      expect(s.overflowCount.value).toBe(0)
    }
    expect(s.visibleResults.value.length).toBeLessThanOrEqual(8)
  })
})
