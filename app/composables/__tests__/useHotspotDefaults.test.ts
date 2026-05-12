import { describe, it, expect } from 'vitest'
import { initialHotspotFor } from '../useHotspotDefaults'
import { CENTER_HOTSPOT_ROLES } from '~/components/icons/CursorIcons'

describe('initialHotspotFor', () => {
  it('中央既定ロールは ratio 0.5 を返す', () => {
    expect(initialHotspotFor('Wait', 256)).toEqual({ x: 0.5, y: 0.5 })
    expect(initialHotspotFor('Crosshair', 256)).toEqual({ x: 0.5, y: 0.5 })
    expect(initialHotspotFor('IBeam', 256)).toEqual({ x: 0.5, y: 0.5 })
    expect(initialHotspotFor('No', 256)).toEqual({ x: 0.5, y: 0.5 })
    expect(initialHotspotFor('SizeNS', 32)).toEqual({ x: 0.5, y: 0.5 })
    expect(initialHotspotFor('SizeAll', 64)).toEqual({ x: 0.5, y: 0.5 })
  })

  it('左上既定ロールは 4/primarySize の ratio を返す', () => {
    expect(initialHotspotFor('Arrow', 32)).toEqual({ x: 4 / 32, y: 4 / 32 })
    expect(initialHotspotFor('Arrow', 256)).toEqual({ x: 4 / 256, y: 4 / 256 })
    expect(initialHotspotFor('Help', 256)).toEqual({ x: 4 / 256, y: 4 / 256 })
    expect(initialHotspotFor('Hand', 256)).toEqual({ x: 4 / 256, y: 4 / 256 })
    expect(initialHotspotFor('NWPen', 32)).toEqual({ x: 4 / 32, y: 4 / 32 })
  })

  it('CENTER_HOTSPOT_ROLES が 9 件で期待 ID を含む', () => {
    expect(CENTER_HOTSPOT_ROLES.size).toBe(9)
    expect(CENTER_HOTSPOT_ROLES.has('Wait')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('Crosshair')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('IBeam')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('No')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('SizeNS')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('SizeWE')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('SizeNWSE')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('SizeNESW')).toBe(true)
    expect(CENTER_HOTSPOT_ROLES.has('SizeAll')).toBe(true)
  })

  it('中央既定ロールには Arrow 等は含まれない', () => {
    expect(CENTER_HOTSPOT_ROLES.has('Arrow')).toBe(false)
    expect(CENTER_HOTSPOT_ROLES.has('Help')).toBe(false)
    expect(CENTER_HOTSPOT_ROLES.has('Hand')).toBe(false)
  })

  it('primarySize=0 のとき左上ロールは ratio 0 を返す', () => {
    expect(initialHotspotFor('Arrow', 0)).toEqual({ x: 0, y: 0 })
  })

  it('primarySize < 4 のとき 4/primarySize は 1.0 にクランプされる', () => {
    expect(initialHotspotFor('Arrow', 1)).toEqual({ x: 1, y: 1 })
    expect(initialHotspotFor('Arrow', 2)).toEqual({ x: 1, y: 1 })
    expect(initialHotspotFor('Arrow', 3)).toEqual({ x: 1, y: 1 })
    expect(initialHotspotFor('Arrow', 4)).toEqual({ x: 1, y: 1 })
  })

  it('未知ロールは primarySize > 0 なら 4/primarySize の ratio を返す', () => {
    expect(initialHotspotFor('UnknownRoleXyz', 256)).toEqual({ x: 4 / 256, y: 4 / 256 })
  })
})
