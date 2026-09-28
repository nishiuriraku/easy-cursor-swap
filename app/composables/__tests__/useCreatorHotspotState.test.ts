/**
 * useCreatorHotspotState のテスト (P08a Step 2 / C2)。
 *
 * 実 `useCreatorAssets()` を使い、primary/sized の読み書き分岐・center・
 * override 有効化を検証する。
 */
import { describe, expect, it } from 'vitest'
import { ref } from 'vue'
import { useCreatorAssets } from '../useCreatorAssets'
import { useCreatorHotspotState } from '../useCreatorHotspotState'

function setup() {
  const creatorAssets = useCreatorAssets()
  const activeRoleId = ref('Arrow')
  const activeSize = ref(32)
  const perSizeHotspot = ref(false)
  const state = useCreatorHotspotState({
    creatorAssets,
    activeRoleId,
    activeSize,
    perSizeHotspot,
  })
  return { creatorAssets, activeRoleId, activeSize, perSizeHotspot, state }
}

const PNG = new Uint8Array([1, 2, 3])

describe('useCreatorHotspotState', () => {
  it('returns {x:0,y:0} when no asset is assigned', () => {
    const { state } = setup()
    expect(state.activeHotspot.value).toEqual({ x: 0, y: 0 })
    expect(state.sizedOverrideActive.value).toBe(false)
    expect(state.canEditSizedOverride.value).toBe(false)
  })

  it('reads primary hotspot and writes via writeActiveHotspot', () => {
    const { creatorAssets, state } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG,
      primarySize: 32,
      hotspot: { x: 0.1, y: 0.2 },
      source: 'manual',
    })
    expect(state.activeHotspot.value).toEqual({ x: 0.1, y: 0.2 })
    state.writeActiveHotspot({ x: 0.5, y: 0.5 })
    expect(creatorAssets.assigned.value.Arrow?.hotspot).toEqual({ x: 0.5, y: 0.5 })
  })

  it('writes to sized override when perSizeHotspot is on and override exists', () => {
    const { creatorAssets, perSizeHotspot, state } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG,
      primarySize: 32,
      hotspot: { x: 0.1, y: 0.2 },
      sized: new Map([[32, { png: PNG, hotspot: { x: 0.3, y: 0.3 } }]]),
      source: 'manual',
    })
    perSizeHotspot.value = true
    expect(state.activeHotspot.value).toEqual({ x: 0.3, y: 0.3 })
    expect(state.sizedOverrideActive.value).toBe(true)
    state.writeActiveHotspot({ x: 0.9, y: 0.9 })
    // sized 側だけ変わり primary は不変
    expect(creatorAssets.assigned.value.Arrow?.sized?.get(32)?.hotspot).toEqual({
      x: 0.9,
      y: 0.9,
    })
    expect(creatorAssets.assigned.value.Arrow?.hotspot).toEqual({ x: 0.1, y: 0.2 })
  })

  it('enableSizedOverride copies primary hotspot and centerHotspot writes {0.5,0.5}', () => {
    const { creatorAssets, perSizeHotspot, state } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG,
      primarySize: 32,
      hotspot: { x: 0.1, y: 0.2 },
      source: 'manual',
    })
    expect(state.canEditSizedOverride.value).toBe(false)
    perSizeHotspot.value = true
    expect(state.canEditSizedOverride.value).toBe(true)
    state.enableSizedOverride()
    expect(state.sizedOverrideActive.value).toBe(true)
    state.centerHotspot()
    expect(state.activeHotspot.value).toEqual({ x: 0.5, y: 0.5 })
  })

  it('activeHotspotModel setter routes through writeActiveHotspot', () => {
    const { creatorAssets, state } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG,
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      source: 'manual',
    })
    state.activeHotspotModel.value = { x: 0.7, y: 0.8 }
    expect(creatorAssets.assigned.value.Arrow?.hotspot).toEqual({ x: 0.7, y: 0.8 })
  })
})
