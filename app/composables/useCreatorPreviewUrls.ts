/**
 * Creator のプレビュー Blob URL 派生状態 (P08a Step 2 / C3)。
 *
 * `creator.vue` から「ロール/サイズの Blob URL キャッシュ + プレビュー asset」
 * 一式を移動したもの。fetcher が同期なので `usePngBlobCache` の Promise API は
 * 使わず、現行の同期 Map をそのまま移す (判断はここに残す)。
 */
import type { ComputedRef, Ref } from 'vue'
import type { CursorRoleDef } from '~/components/icons/CursorIcons'
import type { CursorPreviewAsset } from '~/components/preview/CursorPreview.vue'
import type { useCreatorAssets } from './useCreatorAssets'
import type { useCreatorHotspotState } from './useCreatorHotspotState'

export interface CreatorPreviewUrlsDeps {
  creatorAssets: Pick<ReturnType<typeof useCreatorAssets>, 'assigned'>
  activeRoleId: Ref<string>
  activeSize: Ref<number>
  activeRole: ComputedRef<CursorRoleDef>
  activeAniFrames: ReturnType<typeof useCreatorHotspotState>['activeAniFrames']
}

export interface CreatorPreviewUrls {
  filledSizes: ComputedRef<number[]>
  activePreviewUrl: ComputedRef<string | null>
  activePreviewAsset: ComputedRef<CursorPreviewAsset>
  sizePreviewMap: ComputedRef<Record<number, string>>
}

export function useCreatorPreviewUrls(deps: CreatorPreviewUrlsDeps): CreatorPreviewUrls {
  const { assigned } = deps.creatorAssets
  const { activeRoleId, activeSize, activeRole, activeAniFrames } = deps

  /**
   * 現在ロールに「埋まっているサイズ」を assigned から導出。
   * primary は必ず含まれ、sized オーバーライドのキーを和集合で足す。
   */
  const filledSizes = computed<number[]>(() => {
    const a = assigned.value[activeRoleId.value]
    if (!a) return []
    const set = new Set<number>([a.primarySize])
    if (a.sized) for (const k of a.sized.keys()) set.add(k)
    return Array.from(set).sort((x, y) => x - y)
  })

  // 各ロールの primary バイト列から Blob URL を派生し、ロール切替時に正しいプレビューを表示する。
  // ロール毎にキャッシュして、リスト中のロール切替で URL を毎回作り直さない。
  const roleBlobCache = new Map<string, { url: string; ref: Uint8Array }>()
  function ensureRoleBlobUrl(roleId: string, bytes: Uint8Array): string {
    const cached = roleBlobCache.get(roleId)
    if (cached && cached.ref === bytes) return cached.url
    if (cached) URL.revokeObjectURL(cached.url)
    // Uint8Array → BlobPart: 一旦 ArrayBuffer のスライスにコピーして TS 型互換にする
    const buf = bytes.slice().buffer
    const url = URL.createObjectURL(new Blob([buf], { type: 'image/png' }))
    roleBlobCache.set(roleId, { url, ref: bytes })
    return url
  }

  /** 現在の役割に紐付いた表示用 PNG URL。assigned が無いロールは null (既定アイコン表示)。 */
  const activePreviewUrl = computed<string | null>(() => {
    const a = assigned.value[activeRoleId.value]
    if (a?.primary) return ensureRoleBlobUrl(activeRoleId.value, a.primary)
    return null
  })

  /**
   * `<CursorPreview>` に渡す現在の asset 形。
   * ANI フレームがあれば 'ani'、静止 PNG があれば 'static'、どちらもなければ 'empty'。
   */
  const activePreviewAsset = computed<CursorPreviewAsset>(() => {
    const frames = activeAniFrames.value
    if (frames) {
      const a = assigned.value[activeRoleId.value]
      return {
        kind: 'ani',
        framePngs: frames.framePngs,
        sequence: frames.sequence,
        durations: frames.perStepDurationsMs,
        nativeSize: a?.primarySize ?? activeSize.value,
      }
    }
    const url = activePreviewUrl.value
    if (url) return { kind: 'static', url, alt: activeRole.value.jp }
    return { kind: 'empty' }
  })

  /**
   * 現在ロールの各サイズに対する実画像 Blob URL マップ。
   * SizeStrip の各タイルに表示する。
   *  - sized[size] があればそれを (size 別オーバーライド)
   *  - 無く且つ size === primarySize なら primary を
   *
   * Blob URL は role + size でキャッシュし、ロール切替で revoke する。
   */
  const sizeBlobCache = new Map<string, { url: string; ref: Uint8Array }>()
  function ensureSizeBlobUrl(roleId: string, size: number, bytes: Uint8Array): string {
    const key = `${roleId}:${size}`
    const cached = sizeBlobCache.get(key)
    if (cached && cached.ref === bytes) return cached.url
    if (cached) URL.revokeObjectURL(cached.url)
    const buf = bytes.slice().buffer
    const url = URL.createObjectURL(new Blob([buf], { type: 'image/png' }))
    sizeBlobCache.set(key, { url, ref: bytes })
    return url
  }

  const sizePreviewMap = computed<Record<number, string>>(() => {
    const out: Record<number, string> = {}
    const a = assigned.value[activeRoleId.value]
    if (!a) return out
    const roleId = activeRoleId.value
    for (const size of filledSizes.value) {
      const sized = a.sized?.get(size)
      if (sized?.png) {
        out[size] = ensureSizeBlobUrl(roleId, size, sized.png)
      } else if (size === a.primarySize && a.primary) {
        out[size] = ensureSizeBlobUrl(roleId, size, a.primary)
      }
    }
    return out
  })

  onBeforeUnmount(() => {
    for (const { url } of roleBlobCache.values()) URL.revokeObjectURL(url)
    roleBlobCache.clear()
  })

  return { filledSizes, activePreviewUrl, activePreviewAsset, sizePreviewMap }
}
