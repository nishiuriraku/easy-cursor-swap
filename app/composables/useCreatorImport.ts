/**
 * Creator の「単一ファイル取り込み」フローを creator.vue から分離した composable。
 *
 * 役割:
 *  - PNG / SVG / .cur / .ico を 1 つ受け取り、現在の active ロールに反映する
 *  - importBusy / importMessage / sanitizedRemovals の UI 状態を管理する
 *  - トーストの自動消去 (~3.5s)
 *
 * 依存:
 *  - `creatorAssets` (useCreatorAssets の戻値)
 *  - active ロール/サイズ ref と filledRoles/filledSizesByRole 状態
 *  - `rasterizeSvgToPng` (Canvas 経由の SVG → PNG ヘルパ)
 *
 * これらは creator.vue が所有しているため、依存注入 (factory deps) でやり取りする。
 */
import { ref, watch, type Ref } from 'vue'
import { invokeTauri } from './useTauri'
import { sanitizeSvg } from './sanitizeSvg'
import { initialHotspotFor } from './useHotspotDefaults'
import type { useCreatorAssets } from './useCreatorAssets'

export interface CreatorImportDeps {
  creatorAssets: ReturnType<typeof useCreatorAssets>
  activeRoleId: Ref<string>
  activeSize: Ref<number>
  /** `reactive(Set<string>)`。creator.vue 側で確保。 */
  filledRoles: Set<string>
  filledSizesByRole: Ref<Record<string, number[]>>
  /** SVG → PNG ラスタライズ。Canvas API 依存なので creator.vue が提供。 */
  rasterizeSvgToPng: (svgString: string, size: number) => Promise<Uint8Array>
}

const TOAST_AUTO_DISMISS_MS = 3500

export function useCreatorImport(deps: CreatorImportDeps) {
  const {
    creatorAssets,
    activeRoleId,
    activeSize,
    filledRoles,
    filledSizesByRole,
    rasterizeSvgToPng,
  } = deps
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
   */
  function applyImportedRaster(png: Uint8Array, primarySize: number) {
    filledRoles.add(activeRoleId.value)
    const map = filledSizesByRole.value[activeRoleId.value] ?? []
    if (!map.includes(activeSize.value)) {
      filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
    }
    const existing = assigned.value[activeRoleId.value]
    const hotspot = existing?.hotspot ?? initialHotspotFor(activeRoleId.value, primarySize)
    setAsset(activeRoleId.value, {
      primary: png,
      primarySize,
      hotspot,
      source: 'manual',
    })
  }

  /** Tauri plugin-fs でファイルを読み込み、PNG / SVG として現在のロールに反映する。 */
  async function pickRasterFromPath(path: string, ext: string) {
    const { readFile } = await import('@tauri-apps/plugin-fs')
    const data = await readFile(path)
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data)
    if (ext === 'svg') {
      const text = new TextDecoder().decode(bytes)
      const { sanitized, removed } = sanitizeSvg(text)
      if (!sanitized) throw new Error('SVG が解析できません: ' + removed.join(', '))
      sanitizedRemovals.value = removed
      const png = await rasterizeSvgToPng(sanitized, 256)
      applyImportedRaster(png, 256)
      importMessage.value =
        removed.length > 0
          ? `SVG を sanitize しました (除去: ${removed.length} 件)`
          : `SVG をインポートしました`
    } else {
      if (
        bytes.length < 8 ||
        bytes[0] !== 0x89 ||
        bytes[1] !== 0x50 ||
        bytes[2] !== 0x4e ||
        bytes[3] !== 0x47
      ) {
        throw new Error('PNG ヘッダーが不正です')
      }
      applyImportedRaster(bytes, 256)
      importMessage.value = 'PNG をインポートしました'
    }
  }

  /** `.cur` / `.ico` ファイルをパスから直接 Rust 側でパースする (ダイアログを開かない版)。 */
  async function pickCursorFromPath(picked: string) {
    const result = await invokeTauri<{
      isCur: boolean
      width: number
      height: number
      hotspot: { x: number; y: number }
      pngBytes: number[]
      availableSizes: number[]
    }>('import_cursor_file', { path: picked })
    if (!result) throw new Error('IPC 結果が空でした')

    const png = new Uint8Array(result.pngBytes)
    filledRoles.add(activeRoleId.value)
    const map = filledSizesByRole.value[activeRoleId.value] ?? []
    if (!map.includes(activeSize.value)) {
      filledSizesByRole.value[activeRoleId.value] = [...map, activeSize.value]
    }
    // .cur/.ico に hotspot ratio が (0, 0) かつ新規ロールなら、初期値を当てる。
    const isNewRole = !creatorAssets.hasAsset(activeRoleId.value)
    const noEmbeddedHotspot = result.hotspot.x === 0 && result.hotspot.y === 0
    const hotspot =
      isNewRole && noEmbeddedHotspot
        ? initialHotspotFor(activeRoleId.value, result.width)
        : result.hotspot
    setAsset(activeRoleId.value, {
      primary: png,
      primarySize: result.width,
      hotspot,
      source: 'manual',
    })
    const sizeList = result.availableSizes.length > 0 ? result.availableSizes.join('/') : '?'
    const kind = result.isCur ? '.cur' : '.ico'
    importMessage.value = `${kind} を取り込みました (${result.width}x${result.height}, 含解像度: ${sizeList})`
  }

  return {
    importBusy,
    importMessage,
    sanitizedRemovals,
    applyImportedRaster,
    pickRasterFromPath,
    pickCursorFromPath,
  }
}
