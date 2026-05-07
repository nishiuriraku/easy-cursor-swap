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
