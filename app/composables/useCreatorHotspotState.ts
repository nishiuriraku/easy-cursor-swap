/**
 * Creator のホットスポット編集状態 (P08a Step 2 / C2)。
 *
 * `creator.vue` から「表示・操作対象 hotspot の読み書き」一式を移動したもの。
 * perSizeHotspot=ON かつ sized.hotspot が存在すれば sized 側、
 * それ以外は primary に読み書きする。
 */
import type { ComputedRef, Ref, WritableComputedRef } from 'vue'
import type { Hotspot, RoleAsset } from './useCreatorAssets'
import { useCreatorAssets } from './useCreatorAssets'

export interface CreatorHotspotDeps {
  creatorAssets: Pick<ReturnType<typeof useCreatorAssets>, 'assigned' | 'setAsset'>
  activeRoleId: Ref<string>
  activeSize: Ref<number>
  perSizeHotspot: Ref<boolean>
}

export interface CreatorHotspotState {
  activeHotspot: ComputedRef<Hotspot>
  activeHotspotModel: WritableComputedRef<Hotspot>
  sizedOverrideActive: ComputedRef<boolean>
  canEditSizedOverride: ComputedRef<boolean>
  activeAniFrames: ComputedRef<NonNullable<RoleAsset['aniFrames']> | null>
  activeAniSourcePath: ComputedRef<string | null>
  writeActiveHotspot: (next: Hotspot) => void
  enableSizedOverride: () => void
  centerHotspot: () => void
}

export function useCreatorHotspotState(deps: CreatorHotspotDeps): CreatorHotspotState {
  const { assigned, setAsset } = deps.creatorAssets
  const { activeRoleId, activeSize, perSizeHotspot } = deps

  /**
   * 現在のロール + サイズで「表示・操作対象」のホットスポット (ratio)。
   * perSizeHotspot=ON かつ sized.hotspot=Some のとき sized 側を返す。
   */
  const activeHotspot = computed<Hotspot>(() => {
    const a = assigned.value[activeRoleId.value]
    if (!a) return { x: 0, y: 0 }
    if (perSizeHotspot.value) {
      const sized = a.sized?.get(activeSize.value)
      if (sized?.hotspot) return sized.hotspot
    }
    return a.hotspot
  })

  /**
   * 現在の編集対象 (primary or sized override) に hotspot を書き込む。
   * perSizeHotspot=ON かつそのサイズに override が既に存在 (sized.hotspot=Some) なら sized 側に、
   * それ以外は primary に書く。editor 操作 (pointer / keyboard / model setter) 専用。
   * import 系 (applyImportedRaster / pickCursorFromPath) は primary 直接書込を維持する。
   */
  function writeActiveHotspot(next: Hotspot) {
    const id = activeRoleId.value
    const a = assigned.value[id]
    if (!a) return
    const sized = a.sized?.get(activeSize.value)
    if (perSizeHotspot.value && sized?.hotspot) {
      const nextSizedMap = new Map(a.sized ?? new Map())
      nextSizedMap.set(activeSize.value, { ...sized, hotspot: next })
      setAsset(id, { ...a, sized: nextSizedMap })
    } else {
      setAsset(id, { ...a, hotspot: next })
    }
  }

  /**
   * activeHotspot の writable 版。pointer / keyboard ハンドラから setter 経由で更新する。
   * writeActiveHotspot 経由で perSizeHotspot=ON 時に sized へ書き込む。
   */
  const activeHotspotModel = computed<Hotspot>({
    get: () => activeHotspot.value,
    set: (next) => {
      writeActiveHotspot(next)
    },
  })

  /**
   * 現在のアクティブサイズに sized.hotspot override が存在するか。
   * enableSizedOverride を押した後に true になる。
   */
  const sizedOverrideActive = computed(() => {
    const a = assigned.value[activeRoleId.value]
    return !!a?.sized?.get(activeSize.value)?.hotspot
  })

  /**
   * sized override の有効化ボタンを押せる条件 (アセット割り当て済み + perSizeHotspot=ON)。
   */
  const canEditSizedOverride = computed(() => {
    const a = assigned.value[activeRoleId.value]
    return !!a && perSizeHotspot.value
  })

  /**
   * このサイズの sized.hotspot を primary hotspot からコピーして初期化する。
   * 以後 writeActiveHotspot が sized 側に書き込むようになる。
   */
  function enableSizedOverride() {
    const id = activeRoleId.value
    const a = assigned.value[id]
    if (!a) return
    const nextSizedMap = new Map(a.sized ?? new Map())
    const existing = nextSizedMap.get(activeSize.value)
    nextSizedMap.set(activeSize.value, {
      png: existing?.png ?? a.primary,
      // 現在の primary hotspot をコピーして編集起点にする
      hotspot: { ...a.hotspot },
    })
    setAsset(id, { ...a, sized: nextSizedMap })
  }

  /** アクティブロールに .ani フレームデータが存在する場合にそれを返す。 */
  const activeAniFrames = computed(() => {
    const id = activeRoleId.value
    if (!id) return null
    return assigned.value[id]?.aniFrames ?? null
  })

  /** アクティブロールの .ani 元ファイルパス (存在する場合のみ)。 */
  const activeAniSourcePath = computed(() => {
    const id = activeRoleId.value
    if (!id) return null
    return assigned.value[id]?.aniSourcePath ?? null
  })

  /**
   * 現在ロールのホットスポットを画像中央 (0.5, 0.5) に移動する。
   */
  function centerHotspot() {
    writeActiveHotspot({ x: 0.5, y: 0.5 })
  }

  return {
    activeHotspot,
    activeHotspotModel,
    sizedOverrideActive,
    canEditSizedOverride,
    activeAniFrames,
    activeAniSourcePath,
    writeActiveHotspot,
    enableSizedOverride,
    centerHotspot,
  }
}
