/**
 * sanitized SVG 文字列 → 指定サイズの PNG バイト列 (Canvas 経由)。
 *
 * `creator.vue` にあった DOM 依存ロジックの純関数版。i18n 文言は引数で
 * 受ける (composable 内で `t` を持たない既存 `sanitizeSvg.ts` と同じ
 * 純関数スタイル)。Canvas API 依存のため呼び出し側はブラウザ前提。
 */
export interface RasterizeErrorTexts {
  imageLoad: string
  canvas2d: string
  toBlob: string
}

export async function rasterizeSvgToPng(
  svgString: string,
  size: number,
  errors: RasterizeErrorTexts,
): Promise<Uint8Array> {
  const blob = new Blob([svgString], { type: 'image/svg+xml' })
  const url = URL.createObjectURL(blob)
  try {
    const img = new Image()
    img.decoding = 'async'
    img.src = url
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve()
      img.onerror = () => reject(new Error(errors.imageLoad))
    })
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error(errors.canvas2d)
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(img, 0, 0, size, size)
    const pngBlob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error(errors.toBlob))), 'image/png')
    })
    return new Uint8Array(await pngBlob.arrayBuffer())
  } finally {
    URL.revokeObjectURL(url)
  }
}
