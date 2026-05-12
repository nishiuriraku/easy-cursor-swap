import { describe, it, expect } from 'vitest'
import { pointerToHotspotRatio, applyKeyboardNudge } from '../useHotspotInteraction'

describe('pointerToHotspotRatio', () => {
  // 200×200 のコンテナ、displayPct=90 → 内側 180×180 が画像領域
  // 左上の余白は各 10px (5% × 200)
  const rect = { left: 0, top: 0, width: 200, height: 200 } as DOMRect
  const displayPct = 90

  it('画像領域の中心は (0.5, 0.5)', () => {
    const r = pointerToHotspotRatio({ clientX: 100, clientY: 100 }, rect, displayPct)
    expect(r.x).toBeCloseTo(0.5, 5)
    expect(r.y).toBeCloseTo(0.5, 5)
  })

  it('画像領域の左上端 (10px,10px) は (0, 0)', () => {
    const r = pointerToHotspotRatio({ clientX: 10, clientY: 10 }, rect, displayPct)
    expect(r.x).toBeCloseTo(0, 5)
    expect(r.y).toBeCloseTo(0, 5)
  })

  it('画像領域の右下端 (190px,190px) は (1, 1)', () => {
    const r = pointerToHotspotRatio({ clientX: 190, clientY: 190 }, rect, displayPct)
    expect(r.x).toBeCloseTo(1, 5)
    expect(r.y).toBeCloseTo(1, 5)
  })

  it('画像領域の外側 (0,0) は (0, 0) にクランプ', () => {
    const r = pointerToHotspotRatio({ clientX: 0, clientY: 0 }, rect, displayPct)
    expect(r.x).toBe(0)
    expect(r.y).toBe(0)
  })

  it('画像領域の外側 (300,300) は (1, 1) にクランプ', () => {
    const r = pointerToHotspotRatio({ clientX: 300, clientY: 300 }, rect, displayPct)
    expect(r.x).toBe(1)
    expect(r.y).toBe(1)
  })
})

describe('applyKeyboardNudge', () => {
  const refPx = 256 // 参照画像の native px 幅

  it('ArrowRight は +1/256 進む', () => {
    const next = applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'ArrowRight', false, refPx)
    expect(next).toEqual({ x: 0.5 + 1 / 256, y: 0.5 })
  })

  it('Shift+ArrowRight は +10/256 進む', () => {
    const next = applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'ArrowRight', true, refPx)
    expect(next).toEqual({ x: 0.5 + 10 / 256, y: 0.5 })
  })

  it('Home は x=0、End は x=1、PageUp は y=0、PageDown は y=1', () => {
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'Home', false, refPx)).toEqual({ x: 0, y: 0.5 })
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'End', false, refPx)).toEqual({ x: 1, y: 0.5 })
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'PageUp', false, refPx)).toEqual({ x: 0.5, y: 0 })
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'PageDown', false, refPx)).toEqual({
      x: 0.5,
      y: 1,
    })
  })

  it('未対応キーは null を返す', () => {
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, 'Escape', false, refPx)).toBeNull()
    expect(applyKeyboardNudge({ x: 0.5, y: 0.5 }, ' ', false, refPx)).toBeNull()
  })

  it('結果は 0..1 にクランプされる', () => {
    expect(applyKeyboardNudge({ x: 0, y: 0 }, 'ArrowLeft', true, refPx)).toEqual({ x: 0, y: 0 })
    expect(applyKeyboardNudge({ x: 1, y: 1 }, 'ArrowDown', true, refPx)).toEqual({ x: 1, y: 1 })
  })

  it('refPx <= 0 のときは step=0 で位置は変わらない', () => {
    expect(applyKeyboardNudge({ x: 0.3, y: 0.4 }, 'ArrowRight', false, 0)).toEqual({
      x: 0.3,
      y: 0.4,
    })
    expect(applyKeyboardNudge({ x: 0.3, y: 0.4 }, 'ArrowDown', false, -1)).toEqual({
      x: 0.3,
      y: 0.4,
    })
  })
})
