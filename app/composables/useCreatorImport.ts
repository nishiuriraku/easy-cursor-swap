/**
 * Creator の「単一ファイル取り込み」フローを creator.vue から分離した composable。
 *
 * 役割:
 *  - applyImportedRaster で active ロールにバイト列を反映する
 *  - importBusy / importMessage / sanitizedRemovals の UI 状態を管理する
 *  - トーストの自動消去 (~3.5s)
 *
 * 依存:
 *  - `creatorAssets` (useCreatorAssets の戻値)
 *  - active ロール ref
 *
 * これらは creator.vue が所有しているため、依存注入 (factory deps) でやり取りする。
 */
import type { Ref } from 'vue'
import type { useCreatorAssets } from './useCreatorAssets'

export interface CreatorImportDeps {
  creatorAssets: ReturnType<typeof useCreatorAssets>
  activeRoleId: Ref<string>
}

const TOAST_AUTO_DISMISS_MS = 3500

export function useCreatorImport(deps: CreatorImportDeps) {
  const { creatorAssets, activeRoleId } = deps
  const { assigned, setAsset } = creatorAssets

  const importBusy = ref(false)
  const importMessage = ref<string | null>(null)
  const sanitizedRemovals = ref<string[]>([])

  // 自動消去 timer。`importMessage = null` を一定時間後に発火させる。
  let importMessageTimer: ReturnType<typeof setTimeout> | null = null
  watch(importMessage, (msg) => {
    if (importMessageTimer) {
      clearTimeout(importMessageTimer)
      importMessageTimer = null
    }
    if (msg !== null) {
      importMessageTimer = setTimeout(() => {
        importMessage.value = null
        importMessageTimer = null
      }, TOAST_AUTO_DISMISS_MS)
    }
  })

  /**
   * 取り込んだ raster バイト列を「現在の activeRole」に反映する共通ロジック。
   * 新規ロールならデフォルト hotspot を当て、既存ロールは現在の hotspot を維持。
   *
   * assigned へ setAsset するだけでよい (filled* 状態は creator.vue 側で
   * assigned 由来の computed が拾うので二重管理しない)。
   */
  function applyImportedRaster(png: Uint8Array, primarySize: number) {
    const existing = assigned.value[activeRoleId.value]
    const hotspot = existing?.hotspot ?? initialHotspotFor(activeRoleId.value, primarySize)
    setAsset(activeRoleId.value, {
      primary: png,
      primarySize,
      hotspot,
      source: 'manual',
    })
  }

  return {
    importBusy,
    importMessage,
    sanitizedRemovals,
    applyImportedRaster,
  }
}
