/**
 * useAccessibility.getAccessibilityConflicts が get_accessibility_conflicts IPC を
 * 叩いて結果をそのまま返すことを検証する (ApplyModal / settings の直 invoke 集約)。
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

const invoke = vi.fn()
vi.mock('../useTauri', () => ({
  invokeTauri: (...args: unknown[]) => invoke(...args),
}))

import { useAccessibility } from '../useAccessibility'

describe('useAccessibility.getAccessibilityConflicts', () => {
  beforeEach(() => invoke.mockReset())

  it('get_accessibility_conflicts を invoke して結果を返す', async () => {
    const payload = {
      mouse_sonar_enabled: true,
      high_contrast_enabled: false,
      cursor_base_size: 48,
      cursor_size_slider: 2,
      cursor_type: 0,
      has_conflicts: true,
    }
    invoke.mockResolvedValue(payload)

    const { getAccessibilityConflicts } = useAccessibility()
    const r = await getAccessibilityConflicts()

    expect(invoke).toHaveBeenCalledWith('get_accessibility_conflicts')
    expect(r).toEqual(payload)
  })
})
