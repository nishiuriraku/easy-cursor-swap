/**
 * useCreatorBulkImportFlow の dispatchBulkPaths が「単一ファイル選択」でも
 * bulk preview を必ず開くことを保証する回帰テスト。
 *
 * 経緯: 以前は paths.length === 1 かつ .cur/.ico/.png/.svg のとき
 * BulkImportPreviewModal を経由せず現在ロールへ直接代入する fast-path があった。
 * しかし「一括インポート」ボタンから 1 件選んだ場合 fast-path に流れてしまい、
 * 「プレビューが開かず Arrow ロールにだけ無断で上書きされる」現象が起きていた。
 * fast-path を撤去した上で、本テストで挙動を固定する。
 */
import { describe, it, expect, vi } from 'vitest'
import { ref } from 'vue'
import { useCreatorBulkImportFlow } from '~/composables/useCreatorBulkImportFlow'
import type { ResolvedAsset } from '~/composables/useBulkImport'

function makeResolvedAsset(overrides: Partial<ResolvedAsset> = {}): ResolvedAsset {
  return {
    sourceFile: 'arrow.png',
    sourcePath: 'C:/tmp/arrow.png',
    kind: 'png',
    pngBytes: [0x89, 0x50, 0x4e, 0x47],
    width: 32,
    height: 32,
    hotspot: { x: 0, y: 0 },
    svgText: null,
    availableSizes: [32],
    ani: null,
    ...overrides,
  }
}

function makeDeps(overrides: { resolveAssets?: ReturnType<typeof vi.fn> } = {}) {
  const resolveAssets =
    overrides.resolveAssets ??
    vi.fn().mockResolvedValue({ assets: [makeResolvedAsset()], failures: [] })
  const parseCursorpack = vi.fn().mockResolvedValue({
    metadata: { nameJa: 'X', nameEn: null, author: null, version: null, description: null },
    roles: {},
  })
  const bulkImport = {
    busy: ref(false),
    progress: ref(null),
    resolveAssets,
    parseCursorpack,
    cancel: vi.fn(),
    // useBulkImport の戻り値の型に合わせるための any キャスト用ダミー
  } as unknown as ReturnType<typeof import('~/composables/useBulkImport').useBulkImport>

  const creatorAssets = {
    setAsset: vi.fn(),
    // useCreatorAssets の他のメソッドは dispatchBulkPaths 経路では呼ばれない
  } as unknown as ReturnType<typeof import('~/composables/useCreatorAssets').useCreatorAssets>

  return {
    deps: {
      bulkImport,
      creatorAssets,
      filledRoles: new Set<string>(),
      filledSizesByRole: ref<Record<string, number[]>>({}),
      sourceThemeId: ref<string | null>(null),
      metaName: ref(''),
      metaNameEn: ref(''),
      metaAuthor: ref(''),
      metaVersion: ref(''),
      metaDescription: ref(''),
      saveModalOpen: ref(false),
      saveModalDefault: ref<'file' | 'library' | 'libraryAndApply'>('file'),
      importBusy: ref(false),
      importMessage: ref<string | null>(null),
      sanitizedRemovals: ref<string[]>([]),
    },
    resolveAssets,
    parseCursorpack,
  }
}

describe('useCreatorBulkImportFlow.dispatchBulkPaths', () => {
  it('単一の .png でも bulk preview を必ず開く (fast-path 撤去の回帰防止)', async () => {
    const { deps, resolveAssets } = makeDeps()
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/arrow.png'])

    expect(resolveAssets).toHaveBeenCalledTimes(1)
    expect(flow.bulkModalOpen.value).toBe(true)
    expect(flow.bulkResolved.value).not.toBeNull()
    expect(flow.bulkResolved.value?.length).toBe(1)
  })

  it('単一の .cur でも bulk preview を必ず開く', async () => {
    const { deps, resolveAssets } = makeDeps({
      resolveAssets: vi.fn().mockResolvedValue({
        assets: [makeResolvedAsset({ kind: 'cur', sourceFile: 'pointer.cur' })],
        failures: [],
      }),
    })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/pointer.cur'])

    expect(resolveAssets).toHaveBeenCalledTimes(1)
    expect(flow.bulkModalOpen.value).toBe(true)
  })

  it('単一の .cursorpack は parseCursorpack 経路に進む (preview は開く)', async () => {
    const { deps, resolveAssets, parseCursorpack } = makeDeps()
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/pack.cursorpack'])

    expect(parseCursorpack).toHaveBeenCalledTimes(1)
    expect(resolveAssets).not.toHaveBeenCalled()
    expect(flow.bulkModalOpen.value).toBe(true)
    expect(flow.bulkCursorpack.value).not.toBeNull()
  })
})
