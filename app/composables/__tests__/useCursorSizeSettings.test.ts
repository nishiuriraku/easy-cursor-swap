/**
 * useCursorSizeSettings のテスト (P08a Step 3 / S2)。
 *
 * 換算関数・commit の snap・失敗時ロールバック・a11y gate を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

const invokeTauriMock = vi.fn()
const getAccessibilityConflictsMock = vi.fn()
const openExternalUrlMock = vi.fn()

vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invokeTauriMock(...args),
}))

vi.mock('../useAccessibility', () => ({
  useAccessibility: () => ({
    getAccessibilityConflicts: getAccessibilityConflictsMock,
  }),
}))

vi.mock('../useExternalUrl', () => ({
  useExternalUrl: () => ({ openExternalUrl: openExternalUrlMock }),
}))

import { useCursorSizeSettings } from '../useCursorSizeSettings'

beforeEach(() => {
  invokeTauriMock.mockReset()
  getAccessibilityConflictsMock.mockReset()
  openExternalUrlMock.mockReset()
})

describe('useCursorSizeSettings conversions', () => {
  it('dwordToSlider maps 32->1, 256->15, 40->2 (rounded)', () => {
    const s = useCursorSizeSettings()
    expect(s.dwordToSlider(32)).toBe(1)
    expect(s.dwordToSlider(256)).toBe(15)
    expect(s.dwordToSlider(40)).toBe(2)
  })

  it('sliderToDword is the inverse', () => {
    const s = useCursorSizeSettings()
    expect(s.sliderToDword(1)).toBe(32)
    expect(s.sliderToDword(15)).toBe(256)
    expect(s.sliderToDword(0)).toBe(32)
    expect(s.sliderToDword(99)).toBe(256)
  })
})

describe('useCursorSizeSettings commit', () => {
  it('snaps slider to clamped backend value', async () => {
    invokeTauriMock.mockResolvedValueOnce(48)
    const s = useCursorSizeSettings()
    await s.onCursorSizeCommit(3)
    expect(invokeTauriMock).toHaveBeenCalledWith('set_cursor_base_size', { size: 64 })
    expect(s.cursorSizeSlider.value).toBe(2)
    expect(s.cursorSizeBusy.value).toBe(false)
  })

  it('rolls back from OS on failure', async () => {
    invokeTauriMock.mockRejectedValueOnce(new Error('denied'))
    getAccessibilityConflictsMock.mockResolvedValueOnce({
      cursor_base_size: 32,
      cursor_size_slider: 1,
      cursor_type: 0,
    })
    const s = useCursorSizeSettings()
    await s.onCursorSizeCommit(5)
    expect(s.cursorSizeError.value).toBe('denied')
    expect(s.cursorSizeSlider.value).toBe(1)
  })
})

describe('useCursorSizeSettings a11y gate', () => {
  it('marks accessibility active when slider != 1', async () => {
    getAccessibilityConflictsMock.mockResolvedValueOnce({
      cursor_base_size: 80,
      cursor_size_slider: 3,
      cursor_type: 0,
    })
    const s = useCursorSizeSettings()
    await s.refreshCursorSizeFromOs()
    expect(s.cursorAccessibilityActive.value).toBe(true)
    expect(s.cursorSizeSliderRaw.value).toBe(3)
  })

  it('opens Windows settings deep link', async () => {
    const s = useCursorSizeSettings()
    await s.onOpenWindowsCursorSettings()
    expect(openExternalUrlMock).toHaveBeenCalledWith('ms-settings:easeofaccess-mousepointer')
  })
})
