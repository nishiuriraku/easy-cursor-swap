import { describe, it, expect } from 'vitest'
import { initialHotspotFor } from '../useHotspotDefaults'
import { CENTER_HOTSPOT_ROLES } from '~/components/icons/CursorIcons'

describe('initialHotspotFor', () => {
  it('中央既定ロールは primarySize / 2 を返す', () => {
    expect(initialHotspotFor('Wait', 256)).toEqual({ x: 128, y: 128 })
    expect(initialHotspotFor('Crosshair', 256)).toEqual({ x: 128, y: 128 })
    expect(initialHotspotFor('IBeam', 256)).toEqual({ x: 128, y: 128 })
    expect(initialHotspotFor('No', 256)).toEqual({ x: 128, y: 128 })
    expect(initialHotspotFor('SizeNS', 32)).toEqual({ x: 16, y: 16 })
    expect(initialHotspotFor('SizeAll', 64)).toEqual({ x: 32, y: 32 })
  })

  it('左上既定ロールは (4, 4) を返す', () => {
    expect(initialHotspotFor('Arrow', 256)).toEqual({ x: 4, y: 4 })
    expect(initialHotspotFor('Help', 256)).toEqual({ x: 4, y: 4 })
    expect(initialHotspotFor('Hand', 256)).toEqual({ x: 4, y: 4 })
    expect(initialHotspotFor('NWPen', 32)).toEqual({ x: 4, y: 4 })
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

  it('primarySize=0 でも左上は (4, 4) のまま', () => {
    expect(initialHotspotFor('Arrow', 0)).toEqual({ x: 4, y: 4 })
  })

  it('未知ロールは安全側 (4, 4) を返す', () => {
    expect(initialHotspotFor('UnknownRoleXyz', 256)).toEqual({ x: 4, y: 4 })
  })
})
