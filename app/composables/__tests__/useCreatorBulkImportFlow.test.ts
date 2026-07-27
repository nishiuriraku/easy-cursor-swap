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
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import { useCreatorBulkImportFlow } from '~/composables/useCreatorBulkImportFlow'
import { BulkImportCancelledError } from '~/composables/withProgressJob'
import type { ResolvedAsset } from '~/composables/useBulkImport'
import { useI18n } from '~/composables/useI18n'

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

function makeDeps(
  overrides: {
    resolveAssets?: ReturnType<typeof vi.fn>
    parseCursorpack?: ReturnType<typeof vi.fn>
  } = {},
) {
  const resolveAssets =
    overrides.resolveAssets ??
    vi.fn().mockResolvedValue({ assets: [makeResolvedAsset()], failures: [] })
  const parseCursorpack =
    overrides.parseCursorpack ??
    vi.fn().mockResolvedValue({
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
      sourceThemeId: ref<string | null>(null),
      metaName: ref(''),
      metaNameEn: ref(''),
      metaAuthor: ref(''),
      metaVersion: ref(''),
      metaDescription: ref(''),
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

  it('キャンセル中断時は失敗メッセージを出さずモーダルも開かない', async () => {
    const rejecting = vi.fn().mockRejectedValue(new BulkImportCancelledError())
    const { deps } = makeDeps({ resolveAssets: rejecting })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/arrow.png'])

    expect(rejecting).toHaveBeenCalledTimes(1)
    expect(deps.importMessage.value).toBeNull()
    expect(flow.bulkModalOpen.value).toBe(false)
  })
})

/**
 * G17: useCreatorBulkImportFlow.ts のハードコードされた日本語 8 箇所を t() 化し、
 * ja/en parity を取る。
 *
 * 各テストは useI18n().t() を使って期待値を組み立て、`importMessage.value` /
 * `bulkSourceLabel.value` がその値と一致することを確認する。これによって
 *
 *   - 実装が t() 経由である (文字列リテラルを直接埋め込んでいない)
 *   - ja.ts / en.ts に該当キーが存在しプレースホルダが一致している
 *
 * を同時に担保する。
 */
describe('useCreatorBulkImportFlow — G17 i18n (t() 化 + ja/en parity)', () => {
  beforeEach(() => {
    // シングルトン locale ref をテスト間で確定的に戻す
    useI18n().setLocale('ja')
  })

  it('.cursorpack parse 失敗時は t() 化されたメッセージ (creator.bulkImportParseFailed) を出す (ja)', async () => {
    const parseCursorpack = vi.fn().mockRejectedValue(new Error('boom'))
    const { deps } = makeDeps({ parseCursorpack })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/pack.cursorpack'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportParseFailed', { detail: 'boom' }))
    // 期待値は日本語のテンプレートが補間されて出来上がるはず
    expect(deps.importMessage.value).toContain('boom')
  })

  it('同じキーが en ロケールでは英語テンプレートを返す (parity)', () => {
    const { t, setLocale } = useI18n()
    setLocale('en')
    expect(t('creator.bulkImportParseFailed', { detail: 'boom' })).toBe(
      'Failed to import .cursorpack: boom',
    )
  })

  it('.cursorpack が 2 つ以上なら t(creator.bulkImportOnlyOneCursorpack) を出す', async () => {
    const { deps } = makeDeps()
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/a.cursorpack', 'C:/tmp/b.cursorpack'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportOnlyOneCursorpack'))
    // bulk preview は開かない
    expect(flow.bulkModalOpen.value).toBe(false)
  })

  it('.cursorpack と他ファイル混在は t(creator.bulkImportCursorpackExclusive) を出す', async () => {
    const { deps } = makeDeps()
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/pack.cursorpack', 'C:/tmp/extra.png'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportCursorpackExclusive'))
  })

  it('通常 bulk 経路の source label は t(creator.bulkImportFilesLabel, { count })', async () => {
    const { deps, resolveAssets } = makeDeps({
      resolveAssets: vi.fn().mockResolvedValue({
        assets: [makeResolvedAsset(), makeResolvedAsset({ sourceFile: 'pointer.png' })],
        failures: [],
      }),
    })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/a.png', 'C:/tmp/b.png'])

    expect(resolveAssets).toHaveBeenCalledTimes(1)
    const { t } = useI18n()
    expect(flow.bulkSourceLabel.value).toBe(t('creator.bulkImportFilesLabel', { count: 2 }))
  })

  it('bulk resolve 結果が 0 件なら t(creator.bulkImportNoSupportedFiles)', async () => {
    const resolveAssets = vi.fn().mockResolvedValue({ assets: [], failures: [] })
    const { deps } = makeDeps({ resolveAssets })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/a.png'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportNoSupportedFiles'))
    expect(flow.bulkModalOpen.value).toBe(false)
  })

  it('bulk resolve 失敗エントリは t(creator.bulkImportSkippedFiles, { count })', async () => {
    const resolveAssets = vi.fn().mockResolvedValue({
      assets: [makeResolvedAsset()],
      failures: [
        { path: 'C:/tmp/x.png', reason: 'unsupported' },
        { path: 'C:/tmp/y.png', reason: 'corrupt' },
        { path: 'C:/tmp/z.png', reason: 'corrupt' },
      ],
    })
    const { deps } = makeDeps({ resolveAssets })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/a.png', 'C:/tmp/x.png'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportSkippedFiles', { count: 3 }))
  })

  it('bulk resolve 失敗 (キャンセル以外) は t(creator.bulkImportFailed, { detail })', async () => {
    const resolveAssets = vi.fn().mockRejectedValue(new Error('io error'))
    const { deps } = makeDeps({ resolveAssets })
    const flow = useCreatorBulkImportFlow(deps)

    await flow.dispatchBulkPaths(['C:/tmp/a.png'])

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportFailed', { detail: 'io error' }))
  })

  it('apply 完了時のロール件数は t(creator.bulkImportRolesApplied, { count })', () => {
    const { deps } = makeDeps()
    const flow = useCreatorBulkImportFlow(deps)

    flow.applyBulkImport({
      roleAssets: [
        { roleId: 'arrow', asset: makeResolvedAsset({ sourceFile: 'a.png' }) },
        { roleId: 'pointer', asset: makeResolvedAsset({ sourceFile: 'p.png' }) },
        { roleId: 'help', asset: makeResolvedAsset({ sourceFile: 'h.png' }) },
        { roleId: 'wait', asset: makeResolvedAsset({ sourceFile: 'w.png' }) },
      ],
    })

    const { t } = useI18n()
    expect(deps.importMessage.value).toBe(t('creator.bulkImportRolesApplied', { count: 4 }))
    expect(flow.bulkModalOpen.value).toBe(false)
  })

  it('en ロケールに切り替えても同フローで英語テンプレートが返る (parity)', async () => {
    const resolveAssets = vi
      .fn()
      .mockResolvedValueOnce({ assets: [], failures: [] })
      .mockResolvedValueOnce({
        assets: [makeResolvedAsset()],
        failures: [
          { path: 'x', reason: 'r' },
          { path: 'y', reason: 'r' },
        ],
      })
    const { deps } = makeDeps({ resolveAssets })
    const flow = useCreatorBulkImportFlow(deps)
    const { t, setLocale } = useI18n()
    setLocale('en')

    // 0 件ケース → "No supported files found"
    await flow.dispatchBulkPaths(['C:/tmp/a.png'])
    expect(deps.importMessage.value).toBe(t('creator.bulkImportNoSupportedFiles'))

    // 失敗ありケース → "2 file(s) skipped"
    await flow.dispatchBulkPaths(['C:/tmp/b.png'])
    expect(deps.importMessage.value).toBe(t('creator.bulkImportSkippedFiles', { count: 2 }))
  })
})
