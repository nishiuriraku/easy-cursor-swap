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
import { sanitizeSvg } from './sanitizeSvg'
import type { RasterizeErrorTexts } from './rasterizeSvgToPng'
import { rasterizeSvgToPng as defaultRasterizeSvgToPng } from './rasterizeSvgToPng'

export interface CreatorImportDeps {
  creatorAssets: ReturnType<typeof useCreatorAssets>
  activeRoleId: Ref<string>
  rasterizeSvgToPng?: (svg: string, size: number) => Promise<Uint8Array>
  t: (key: string, params?: Record<string, string | number>) => string
}

const TOAST_AUTO_DISMISS_MS = 3500

export function useCreatorImport(deps: CreatorImportDeps) {
  const { creatorAssets, activeRoleId, t } = deps
  const { assigned, setAsset } = creatorAssets
  // 既定のラスタライザは純関数版に i18n 文言を束ねたもの。テスト時は mock を注入する。
  const rasterizeSvgToPng =
    deps.rasterizeSvgToPng ??
    ((svg: string, size: number) =>
      defaultRasterizeSvgToPng(svg, size, {
        imageLoad: t('creator.errSvgImageLoadFailed'),
        canvas2d: t('creator.errCanvas2dContext'),
        toBlob: t('creator.errToBlobFailed'),
      }))

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
    handleFileInput,
  }

  /**
   * 単一ファイル取込 (`<input type=file>` の change)。
   * PNG (magic byte 検証) / SVG (sanitize + rasterize) を受け付け、
   * `applyImportedRaster` で active ロールに反映する。
   */
  async function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    importBusy.value = true
    importMessage.value = null
    sanitizedRemovals.value = []
    try {
      if (file.size > 10 * 1024 * 1024) {
        throw new Error(t('creator.errFileSizeOverMb'))
      }

      const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
      let pngBytes: Uint8Array | null = null
      if (ext === 'svg') {
        const text = await file.text()
        const { sanitized, removed } = sanitizeSvg(text)
        if (!sanitized)
          throw new Error(t('creator.errSvgUnparsable', { removed: removed.join(', ') }))
        sanitizedRemovals.value = removed
        // SVG → 256px PNG にラスタライズして Rust 側ビルダー用に保持
        pngBytes = await rasterizeSvgToPng(sanitized, 256)
        importMessage.value =
          removed.length > 0
            ? t('creator.notifySvgSanitized', { count: removed.length })
            : t('creator.notifySvgImported')
      } else if (ext === 'png') {
        // PNG は magic byte の弱検証のみ (89 50 4E 47)
        const fullBytes = new Uint8Array(await file.arrayBuffer())
        if (
          fullBytes.length < 8 ||
          fullBytes[0] !== 0x89 ||
          fullBytes[1] !== 0x50 ||
          fullBytes[2] !== 0x4e ||
          fullBytes[3] !== 0x47
        ) {
          throw new Error(t('creator.errPngBadHeader'))
        }
        pngBytes = fullBytes
        importMessage.value = t('creator.notifyPngImported')
      } else {
        throw new Error(t('creator.errUnsupportedExt', { ext }))
      }

      // 役割マップに登録 (assigned が真のソース。setAsset 経由で filledRoleSet
      // computed が追従するので filledRoles/filledSizesByRole の手動更新は不要。)
      // エクスポート時にも assigned 経由で使用。
      // PNG/SVG はホットスポット情報を持たないので、既存 hotspot を維持するか、
      // 新規ロールならロールに応じた初期値を適用する。ratio は size 非依存。
      if (pngBytes) {
        applyImportedRaster(pngBytes, 256)
      }
    } catch (err) {
      importMessage.value = t('creator.errImportFailed', {
        detail: appErrorMessage(err),
      })
    } finally {
      importBusy.value = false
      // 同一ファイル再選択を許すため、change イベントの元 input をクリア。
      // input 要素は子コンポーネント (CreatorEditorCanvas) にあるので、
      // e.target 経由で参照する (親の ref は持たない)。
      input.value = ''
    }
  }
}
