/**
 * useCreatorMetaState テスト。
 *
 * - 初期値の reset
 * - isDirty: 初期状態 false / フィールド変更で true / reset 後 false
 *   (creator.vue::hasUnsavedEdits の判定根拠なので独立で検証する)
 */
import { describe, it, expect, beforeEach } from 'vitest'
import { useCreatorMetaState } from '~/composables/useCreatorMetaState'
import { useI18n } from '~/composables/useI18n'

beforeEach(() => {
  useI18n().setLocale('ja')
})

describe('useCreatorMetaState.isDirty', () => {
  it('starts false on a fresh session', () => {
    const m = useCreatorMetaState()
    expect(m.isDirty.value).toBe(false)
  })

  it('becomes true after the user types a different name', () => {
    const m = useCreatorMetaState()
    m.name.value = 'MyTheme'
    expect(m.isDirty.value).toBe(true)
  })

  it('becomes true after the user types into nameEn', () => {
    const m = useCreatorMetaState()
    m.nameEn.value = 'MyTheme'
    expect(m.isDirty.value).toBe(true)
  })

  it('becomes true after the user types an author', () => {
    const m = useCreatorMetaState()
    m.author.value = 'me'
    expect(m.isDirty.value).toBe(true)
  })

  it('becomes true after changing the version away from 1.0.0', () => {
    const m = useCreatorMetaState()
    m.version.value = '1.0.1'
    expect(m.isDirty.value).toBe(true)
  })

  it('becomes true after typing a description', () => {
    const m = useCreatorMetaState()
    m.description.value = 'hello'
    expect(m.isDirty.value).toBe(true)
  })

  it('becomes true after flipping shadowEnabled', () => {
    const m = useCreatorMetaState()
    m.shadowEnabled.value = true
    expect(m.isDirty.value).toBe(true)
  })

  it('returns to false after reset()', () => {
    const m = useCreatorMetaState()
    m.name.value = 'X'
    m.author.value = 'Y'
    m.shadowEnabled.value = true
    expect(m.isDirty.value).toBe(true)
    m.reset()
    expect(m.isDirty.value).toBe(false)
  })
})
