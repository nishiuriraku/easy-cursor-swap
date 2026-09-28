/**
 * usePlatform のテスト (P04 Step 6)。
 *
 * os 定数・platformKey 組み立て・tp の両ロケール解決と補間・
 * 未知キーのフォールバックを検証する。
 */
import { describe, it, expect, beforeEach } from 'vitest'
import { useI18n } from '../useI18n'
import { usePlatform, platformKey } from '../usePlatform'

describe('usePlatform', () => {
  beforeEach(() => useI18n().setLocale('ja'))

  it('os is windows (constant until get_platform_info lands)', () => {
    expect(usePlatform().os.value).toBe('windows')
  })

  it('platformKey builds platform.<os>.<key>', () => {
    expect(platformKey('autoStartHint')).toBe('platform.windows.autoStartHint')
    expect(platformKey('autoStartHint', 'windows')).toBe('platform.windows.autoStartHint')
  })

  it('tp resolves in both locales and interpolates', () => {
    const { tp } = usePlatform()
    const { setLocale } = useI18n()
    expect(tp('autoStartHint')).toBe('HKCU\\…\\Run')
    expect(tp('cursorSizeEoaSizeMessage', { currentSlider: 3 })).toContain('3')
    setLocale('en')
    expect(tp('panicStage1Label')).toBe('Windows default')
  })

  it('unknown key falls back to the key itself (useI18n contract)', () => {
    expect(usePlatform().tp('nope')).toBe('platform.windows.nope')
  })
})
