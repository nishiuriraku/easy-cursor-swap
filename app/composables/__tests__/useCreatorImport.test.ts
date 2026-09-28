/**
 * useCreatorImport の handleFileInput テスト (P08a Step 2 / C7)。
 *
 * PNG magic byte 不正 → errPngBadHeader、10 MB 超 → errFileSizeOverMb、
 * SVG → rasterizeSvgToPng mock 呼び出し、を検証する。
 */
import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useCreatorAssets } from '../useCreatorAssets'
import { useCreatorImport } from '../useCreatorImport'

const t = (key: string) => key

function setup(rasterizeSvgToPng?: (svg: string, size: number) => Promise<Uint8Array>) {
  const creatorAssets = useCreatorAssets()
  const activeRoleId = ref('Arrow')
  const imp = useCreatorImport({ creatorAssets, activeRoleId, rasterizeSvgToPng, t })
  return { creatorAssets, activeRoleId, imp }
}

function fileEvent(name: string, bytes: Uint8Array): Event {
  const file = new File([bytes as BlobPart], name)
  return { target: { files: [file], value: 'x' } } as unknown as Event
}

describe('useCreatorImport handleFileInput', () => {
  it('rejects PNG with bad magic bytes', async () => {
    const { imp } = setup()
    await imp.handleFileInput(fileEvent('a.png', new Uint8Array([0, 1, 2, 3, 4, 5, 6, 7])))
    expect(imp.importMessage.value).toBe('creator.errImportFailed')
    expect(imp.importBusy.value).toBe(false)
  })

  it('rejects files over 10 MB', async () => {
    const { imp } = setup()
    const big = { target: { files: [{ name: 'a.png', size: 11 * 1024 * 1024 }], value: '' } }
    await imp.handleFileInput(big as unknown as Event)
    expect(imp.importMessage.value).toBe('creator.errImportFailed')
  })

  it('accepts valid PNG and applies raster to active role', async () => {
    const { creatorAssets, imp } = setup()
    const header = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2])
    await imp.handleFileInput(fileEvent('a.png', header))
    expect(imp.importMessage.value).toBe('creator.notifyPngImported')
    expect(creatorAssets.assigned.value.Arrow?.primary).toEqual(header)
  })

  it('rasterizes SVG via injected rasterizer', async () => {
    const rasterizeSvgToPng = vi.fn(async () => new Uint8Array([9, 9, 9]))
    const { creatorAssets, imp } = setup(rasterizeSvgToPng)
    const svg = new TextEncoder().encode('<svg></svg>')
    await imp.handleFileInput(fileEvent('a.svg', svg))
    expect(rasterizeSvgToPng).toHaveBeenCalledTimes(1)
    const firstCall = rasterizeSvgToPng.mock.calls[0] as unknown[]
    expect(firstCall[1]).toBe(256)
    expect(creatorAssets.assigned.value.Arrow?.primary).toEqual(new Uint8Array([9, 9, 9]))
  })

  it('rejects unsupported extensions', async () => {
    const { imp } = setup()
    await imp.handleFileInput(fileEvent('a.txt', new Uint8Array([1, 2, 3])))
    expect(imp.importMessage.value).toBe('creator.errImportFailed')
  })
})
