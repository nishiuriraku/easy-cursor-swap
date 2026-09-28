/**
 * useCreatorPreviewUrls のテスト (P08a Step 2 / C3)。
 *
 * `URL.createObjectURL/revokeObjectURL` を stub し、Blob URL の再利用・
 * revoke・unmount 時全 revoke を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { computed, ref } from 'vue'
import type { CursorRoleDef } from '~/components/icons/CursorIcons'
import { useCreatorAssets } from '../useCreatorAssets'
import { useCreatorHotspotState } from '../useCreatorHotspotState'
import { useCreatorPreviewUrls } from '../useCreatorPreviewUrls'

let urlSeq = 0
const revoked: string[] = []

beforeEach(() => {
  urlSeq = 0
  revoked.length = 0
  vi.stubGlobal(
    'URL',
    Object.assign(Object.create(URL), {
      createObjectURL: vi.fn(() => `blob:mock-${++urlSeq}`),
      revokeObjectURL: vi.fn((url: string) => {
        revoked.push(url)
      }),
    }),
  )
})

const PNG_A = new Uint8Array([1])
const PNG_B = new Uint8Array([2])

function setup() {
  const creatorAssets = useCreatorAssets()
  const activeRoleId = ref('Arrow')
  const activeSize = ref(32)
  const perSizeHotspot = ref(false)
  const activeRole = computed(() => ({ id: 'Arrow', jp: '矢印' }) as CursorRoleDef)
  const hotspot = useCreatorHotspotState({
    creatorAssets,
    activeRoleId,
    activeSize,
    perSizeHotspot,
  })
  const previews = useCreatorPreviewUrls({
    creatorAssets,
    activeRoleId,
    activeSize,
    activeRole,
    activeAniFrames: hotspot.activeAniFrames,
  })
  return { creatorAssets, activeRoleId, previews }
}

describe('useCreatorPreviewUrls', () => {
  it('returns empty previews when nothing is assigned', () => {
    const { previews } = setup()
    expect(previews.filledSizes.value).toEqual([])
    expect(previews.activePreviewUrl.value).toBeNull()
    expect(previews.activePreviewAsset.value).toEqual({ kind: 'empty' })
    expect(previews.sizePreviewMap.value).toEqual({})
  })

  it('derives filledSizes from primary + sized keys', () => {
    const { creatorAssets, previews } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG_A,
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      sized: new Map([[48, { png: PNG_B }]]),
      source: 'manual',
    })
    expect(previews.filledSizes.value).toEqual([32, 48])
  })

  it('reuses Blob URL for the same bytes and revokes on change', () => {
    const { creatorAssets, previews } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG_A,
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      source: 'manual',
    })
    const first = previews.activePreviewUrl.value
    expect(first).toContain('blob:mock-')
    // 同じ参照なら再利用
    expect(previews.activePreviewUrl.value).toBe(first)
    expect(revoked).toHaveLength(0)
    // 別参照なら revoke → 新 URL
    creatorAssets.setAsset('Arrow', {
      primary: PNG_B,
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      source: 'manual',
    })
    const second = previews.activePreviewUrl.value
    expect(second).not.toBe(first)
    expect(revoked).toContain(first)
  })

  it('builds sizePreviewMap from sized png with primary fallback', () => {
    const { creatorAssets, previews } = setup()
    creatorAssets.setAsset('Arrow', {
      primary: PNG_A,
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      sized: new Map([[48, { png: PNG_B }]]),
      source: 'manual',
    })
    const map = previews.sizePreviewMap.value
    expect(Object.keys(map).sort()).toEqual(['32', '48'])
    expect(map[48]).not.toBe(map[32])
  })
})
