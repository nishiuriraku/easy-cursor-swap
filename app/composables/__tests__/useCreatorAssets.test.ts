import { describe, it, expect } from 'vitest'
import { useCreatorAssets } from '~/composables/useCreatorAssets'

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
      hotspot: { x: 0.5, y: 0.5 },
      source: 'manual',
    })
    expect(c.arrowAssigned.value).toBe(true)
    expect(c.assignedRoleCount.value).toBe(1)
  })

  it('toExportPayload uses sizedOverrides when sized Map is set', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array([1]),
      primarySize: 256,
      hotspot: { x: 0, y: 0 },
      sized: new Map([[64, { png: new Uint8Array([9]) }]]),
      source: 'cursorpack',
    })
    const payload = c.toExportPayload('lanczos')
    expect(payload[0].sizedOverrides).toEqual({ 64: { pngBytes: [9], hotspot: null } })
  })

  it('toExportPayload includes hotspot as ratio object', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array([1, 2]),
      primarySize: 32,
      hotspot: { x: 0.125, y: 0.125 },
      source: 'manual',
    })
    const payload = c.toExportPayload('lanczos')
    expect(payload[0].hotspot).toEqual({ x: 0.125, y: 0.125 })
  })

  it('toExportPayload sizedOverrides includes per-size hotspot when set', () => {
    const c = useCreatorAssets()
    c.setAsset('Arrow', {
      primary: new Uint8Array([1]),
      primarySize: 256,
      hotspot: { x: 0.5, y: 0.5 },
      sized: new Map([[32, { png: new Uint8Array([7]), hotspot: { x: 0.1, y: 0.1 } }]]),
      source: 'cursorpack',
    })
    const payload = c.toExportPayload('lanczos')
    expect(payload[0].sizedOverrides).toEqual({
      32: { pngBytes: [7], hotspot: { x: 0.1, y: 0.1 } },
    })
  })

  describe('SizedAsset hotspot override', () => {
    it('sized.hotspot は per-size override として保持される', () => {
      const { setAsset, assigned } = useCreatorAssets()
      const sized = new Map([
        [64, { png: new Uint8Array([1, 2, 3]), hotspot: { x: 0.25, y: 0.75 } }],
      ])
      setAsset('Arrow', {
        primary: new Uint8Array([0]),
        primarySize: 32,
        hotspot: { x: 0.5, y: 0.5 },
        sized,
        source: 'manual',
      })
      expect(assigned.value.Arrow?.sized?.get(64)?.hotspot).toEqual({ x: 0.25, y: 0.75 })
    })

    it('toExportPayload は sizedOverrides に hotspot を含め、未設定は null にする', () => {
      const { setAsset, toExportPayload } = useCreatorAssets()
      setAsset('Arrow', {
        primary: new Uint8Array([0]),
        primarySize: 32,
        hotspot: { x: 0.5, y: 0.5 },
        sized: new Map([
          [64, { png: new Uint8Array([1]), hotspot: { x: 0.25, y: 0.75 } }],
          [128, { png: new Uint8Array([2]) }],
        ]),
        source: 'manual',
      })
      const payload = toExportPayload('lanczos')
      const arrow = payload.find((p) => p.role === 'Arrow')!
      expect(arrow.sizedOverrides).not.toBeNull()
      expect(arrow.sizedOverrides?.['64']?.hotspot).toEqual({ x: 0.25, y: 0.75 })
      expect(arrow.sizedOverrides?.['128']?.hotspot).toBeNull()
    })
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
