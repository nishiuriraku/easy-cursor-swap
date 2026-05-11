import { ref, computed } from 'vue'

export type AssetSource = 'manual' | 'bulk-file' | 'bulk-folder' | 'cursorpack'

export interface Hotspot {
  x: number
  y: number
}

export interface RoleAsset {
  /** プライマリ画像 — 単一インポート / 外部素材の最大解像度版。Rust が他サイズをリサンプルする際のソース。 */
  primary: Uint8Array
  primarySize: number
  hotspot: Hotspot
  /** 解像度別オーバーライド。`.cursorpack` 取り込みのみ存在しうる。 */
  sized?: Map<number, Uint8Array>
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

/**
 * `fromSize` 基準のホットスポット px 値を `toSize` 基準に変換する純粋関数。
 *
 * 画像の差し替えやリサンプル時に、UI 上の見た目位置（= ratio）を維持するために使う。
 * 例: 256px キャンバス上の (4, 4) は ratio (1.5%, 1.5%)。これを 32px キャンバスに移すと
 * `round(0.015 * 32) = (0, 0)` になり、左上隅という同じ意味を保つ。
 *
 * `fromSize <= 0` の場合はゼロ除算を避けて元の値をそのまま返す。
 */
export function scaleHotspot(hotspot: Hotspot, fromSize: number, toSize: number): Hotspot {
  if (fromSize <= 0 || toSize <= 0 || fromSize === toSize) return hotspot
  const ratioX = hotspot.x / fromSize
  const ratioY = hotspot.y / fromSize
  return {
    x: Math.max(0, Math.min(toSize, Math.round(ratioX * toSize))),
    y: Math.max(0, Math.min(toSize, Math.round(ratioY * toSize))),
  }
}

/**
 * クリエイターのアセット状態を 1 か所で管理する composable。
 * `creator.vue` から `assignedPng` / `assignedHotspot` を取り除き、ここに集約する。
 */
export function useCreatorAssets() {
  const assigned = ref<Record<string, RoleAsset>>({})

  /** 指定ロールのアセットを設定する。reactivity を確実にトリガーするため immutable spread を使用。 */
  function setAsset(roleId: string, asset: RoleAsset) {
    assigned.value = { ...assigned.value, [roleId]: asset }
  }

  /** 指定ロールのアセットを削除する。 */
  function removeAsset(roleId: string) {
    const next = { ...assigned.value }
    delete next[roleId]
    assigned.value = next
  }

  /** 指定ロールにアセットが割り当てられているか。 */
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
      hotspotX: a.hotspot.x,
      hotspotY: a.hotspot.y,
      resample: resampleMode,
      sizedPngBytes: a.sized
        ? Object.fromEntries(Array.from(a.sized.entries()).map(([k, v]) => [k, Array.from(v)]))
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
