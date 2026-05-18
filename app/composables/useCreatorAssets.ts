export type AssetSource = 'manual' | 'bulk-file' | 'bulk-folder' | 'cursorpack'

/** ホットスポット (比率, 0.0..=1.0)。`.cur` 書出時に Rust 側で primarySize と乗算して px に変換する。 */
export interface Hotspot {
  x: number
  y: number
}

/** サイズ別オーバーライド。`hotspot` が undefined なら親 `RoleAsset.hotspot` を継承。 */
export interface SizedAsset {
  png: Uint8Array
  hotspot?: Hotspot
}

export interface RoleAsset {
  /** プライマリ画像 — 単一インポート / 外部素材の最大解像度版。Rust が他サイズをリサンプルする際のソース。 */
  primary: Uint8Array
  primarySize: number
  hotspot: Hotspot
  /** 解像度別オーバーライド。`.cursorpack` 取り込みのみ存在しうる。 */
  sized?: Map<number, SizedAsset>
  source: AssetSource
  /** `.ani` 取り込み時の元ファイル絶対パス。Rust 側エクスポート時に使う。 */
  aniSourcePath?: string
  /** `.ani` のアニメーション情報。`aniSourcePath` がある場合のみ存在。 */
  aniFrames?: {
    framePngs: Uint8Array[]
    sequence: number[]
    perStepDurationsMs: number[]
  }
}

// scaleHotspot は削除 (ratio は size 非依存なので不要)

/**
 * クリエイターのアセット状態を 1 か所で管理する composable。
 */
export function useCreatorAssets() {
  const assigned = ref<Record<string, RoleAsset>>({})

  function setAsset(roleId: string, asset: RoleAsset) {
    assigned.value = { ...assigned.value, [roleId]: asset }
  }

  function removeAsset(roleId: string) {
    const next = { ...assigned.value }
    delete next[roleId]
    assigned.value = next
  }

  function hasAsset(roleId: string): boolean {
    return roleId in assigned.value
  }

  const assignedRoleCount = computed(() => Object.keys(assigned.value).length)
  const arrowAssigned = computed(() => 'Arrow' in assigned.value)

  /** export_cursorpack_streamed 用にロール配列に変換する。 */
  function toExportPayload(resampleMode: string) {
    return Object.entries(assigned.value).map(([role, a]) => ({
      role,
      pngBytes: Array.from(a.primary),
      hotspot: a.hotspot,
      resample: resampleMode,
      sizedOverrides: a.sized
        ? Object.fromEntries(
            Array.from(a.sized.entries()).map(([k, v]) => [
              k,
              {
                pngBytes: Array.from(v.png),
                hotspot: v.hotspot ?? null,
              },
            ]),
          )
        : null,
      aniSourcePath: a.aniSourcePath ?? null,
    }))
  }

  return {
    assigned,
    setAsset,
    removeAsset,
    hasAsset,
    assignedRoleCount,
    arrowAssigned,
    toExportPayload,
  }
}
