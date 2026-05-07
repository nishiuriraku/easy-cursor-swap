import { describe, it, expect } from 'vitest'
import { useCreatorAssets, scaleHotspot } from '~/composables/useCreatorAssets'

describe('useCreatorAssets', () => {
  it('starts empty', () => {
    const { assignedRoleCount, arrowAssigned } = useCreatorAssets()
    expect(assignedRoleCount.value).toBe(0)
    expect(arrowAssigned.value).toBe(false)
  })

  it('setAsset adds and arrowAssigned becomes true for Arrow', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array([1, 2, 3]),
      primarySize: 256,
      hotspot: { x: 4, y: 4 },
      source: 'manual',
    })
    expect(c.arrowAssigned.value).toBe(true)
    expect(c.assignedRoleCount.value).toBe(1)
  })

  it('toExportPayload includes sizedPngBytes when sized Map is set', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array([1]),
      primarySize: 256,
      hotspot: { x: 0, y: 0 },
      sized: new Map([[64, new Uint8Array([9])]]),
      source: 'cursorpack',
    })
    const payload = c.toExportPayload('lanczos')
    expect(payload[0].sizedPngBytes).toEqual({ 64: [9] })
  })

  it('removeAsset clears the role', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array(),
      primarySize: 0,
      hotspot: { x: 0, y: 0 },
      source: 'manual',
    })
    c.removeAsset('Arrow')
    expect(c.hasAsset('Arrow')).toBe(false)
  })
})

describe('scaleHotspot', () => {
  it('preserves ratio when upscaling 32 -> 256', () => {
    // (4, 4) at 32px = top-left 12.5% should map to (32, 32) at 256px
    expect(scaleHotspot({ x: 4, y: 4 }, 32, 256)).toEqual({ x: 32, y: 32 })
  })

  it('preserves ratio when downscaling 256 -> 32', () => {
    // (32, 32) at 256px = 12.5% should map to (4, 4) at 32px
    expect(scaleHotspot({ x: 32, y: 32 }, 256, 32)).toEqual({ x: 4, y: 4 })
  })

  it('returns input unchanged when fromSize equals toSize', () => {
    const h = { x: 7, y: 11 }
    expect(scaleHotspot(h, 64, 64)).toBe(h)
  })

  it('returns input unchanged when fromSize is zero (avoid divide-by-zero)', () => {
    const h = { x: 7, y: 11 }
    expect(scaleHotspot(h, 0, 64)).toBe(h)
  })

  it('clamps result to 0..toSize range', () => {
    // 過大なホットスポット入力でも toSize を超えない
    expect(scaleHotspot({ x: 99, y: 99 }, 64, 32)).toEqual({ x: 32, y: 32 })
  })

  it('rounds half-pixel values', () => {
    // (1, 1) at 3px → 1/3 ratio → 32/3 ≈ 10.67 → round to 11 at 32px
    expect(scaleHotspot({ x: 1, y: 1 }, 3, 32)).toEqual({ x: 11, y: 11 })
  })
})
