/**
 * BulkImportPreviewModal の ApplyPayload コントラクトと割当ライフサイクルを固定化する。
 *
 * - applyImmediately フィールドが含まれないこと (2026-05-13 UI 簡略化由来)
 * - unassignRole で matches → unmatched へ戻ること
 * - pickRoleFromUnmatched で unmatched → matches へ移動すること
 * - 割当済ロールを未マッチから選ぶと swap が起きること (既存ファイルが未マッチに戻る)
 *
 * UiModal が `<Teleport to="body">` を使うため、テンプレート上のボタン操作は
 * `document.querySelector` 経由で取得する。
 */
import { describe, it, expect, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import BulkImportPreviewModal from '../BulkImportPreviewModal.vue'
import type { ResolvedAsset } from '~/composables/useBulkImport'

const stubs = {
  UiIcon: { template: '<span></span>' },
  UiSelect: { template: '<select></select>' },
  BulkImportRoleRow: { template: '<div></div>' },
}

const baseProps = {
  open: true,
  resolved: [],
  cursorpack: null,
  existingRoles: new Set<string>(),
  sourceLabel: 'test',
}

/** ファイル名と width だけ振った最小 ResolvedAsset を作る。 */
function makeAsset(sourceFile: string, width = 64): ResolvedAsset {
  return {
    sourceFile,
    sourcePath: `/tmp/${sourceFile}`,
    kind: 'cur',
    pngBytes: [0],
    width,
    height: width,
    hotspot: { x: 0, y: 0 },
    svgText: null,
    availableSizes: [width],
    ani: null,
  }
}

let currentWrapper: VueWrapper | null = null

afterEach(() => {
  currentWrapper?.unmount()
  currentWrapper = null
})

describe('BulkImportPreviewModal apply payload', () => {
  it('apply emit の payload に applyImmediately キーは含まない', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      attachTo: document.body,
      global: { stubs },
    })
    currentWrapper = wrapper
    const applyBtn = document.querySelector('.modal-foot button.primary') as HTMLButtonElement | null
    expect(applyBtn).not.toBeNull()
    applyBtn!.click()
    await nextTick()
    const events = wrapper.emitted('apply')
    expect(events).toHaveLength(1)
    const payload = events![0]![0] as Record<string, unknown>
    expect('applyImmediately' in payload).toBe(false)
    expect(payload).toHaveProperty('roleAssets')
    expect(payload).toHaveProperty('metadataChoice')
    expect(payload).toHaveProperty('metadata')
  })

  it('フッターに data-test="apply-immediately" を持つ要素は描画されない', () => {
    currentWrapper = mount(BulkImportPreviewModal, {
      props: baseProps,
      attachTo: document.body,
      global: { stubs },
    })
    expect(document.querySelector('[data-test="apply-immediately"]')).toBeNull()
  })
})

describe('BulkImportPreviewModal assignment lifecycle', () => {
  it('alias 名でマッチしたファイルは matches に入る', () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: { ...baseProps, resolved: [makeAsset('arrow.cur')] },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<{ role: string }>
      unmatched: Array<unknown>
    }
    expect(vm.matches.map((m) => m.role)).toEqual(['Arrow'])
    expect(vm.unmatched).toHaveLength(0)
  })

  it('ファイル名にロールヒントが無いと unmatched に入る', () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: { ...baseProps, resolved: [makeAsset('1.cur')] },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<unknown>
      unmatched: Array<{ asset: ResolvedAsset }>
    }
    expect(vm.matches).toHaveLength(0)
    expect(vm.unmatched.map((u) => u.asset.sourceFile)).toEqual(['1.cur'])
  })

  it('unassignRole で matches から unmatched へ戻る', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: { ...baseProps, resolved: [makeAsset('arrow.cur')] },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<{ role: string; asset: ResolvedAsset }>
      unmatched: Array<{ asset: ResolvedAsset }>
      unassignRole: (roleId: string) => void
    }
    expect(vm.matches).toHaveLength(1)

    vm.unassignRole('Arrow')
    await nextTick()

    expect(vm.matches).toHaveLength(0)
    expect(vm.unmatched.map((u) => u.asset.sourceFile)).toEqual(['arrow.cur'])
  })

  it('pickRoleFromUnmatched で unmatched から matches へ移る', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: { ...baseProps, resolved: [makeAsset('1.cur')] },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<{ role: string; asset: ResolvedAsset }>
      unmatched: Array<{ asset: ResolvedAsset }>
      pickRoleFromUnmatched: (item: { asset: ResolvedAsset }, roleId: string) => void
    }
    const item = vm.unmatched[0]!
    vm.pickRoleFromUnmatched(item, 'Wait')
    await nextTick()

    expect(vm.unmatched).toHaveLength(0)
    expect(vm.matches.map((m) => ({ role: m.role, file: m.asset.sourceFile }))).toEqual([
      { role: 'Wait', file: '1.cur' },
    ])
  })

  it('unassignAll で全ての matches が unmatched に戻る', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: {
        ...baseProps,
        resolved: [makeAsset('arrow.cur'), makeAsset('wait.cur'), makeAsset('help.cur')],
      },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<{ role: string }>
      unmatched: Array<{ asset: ResolvedAsset }>
      unassignAll: () => void
    }
    expect(vm.matches.length).toBe(3)
    expect(vm.unmatched.length).toBe(0)

    vm.unassignAll()
    await nextTick()

    expect(vm.matches.length).toBe(0)
    expect(vm.unmatched.map((u) => u.asset.sourceFile).sort()).toEqual(
      ['arrow.cur', 'help.cur', 'wait.cur'].sort(),
    )
  })

  it('割当済ロールを未マッチから選ぶと既存ファイルが unmatched に戻る (swap)', async () => {
    const wrapper = mount(BulkImportPreviewModal, {
      props: {
        ...baseProps,
        resolved: [makeAsset('arrow.cur'), makeAsset('mystery.cur')],
      },
      global: { stubs },
    })
    const vm = wrapper.vm as unknown as {
      matches: Array<{ role: string; asset: ResolvedAsset }>
      unmatched: Array<{ asset: ResolvedAsset }>
      pickRoleFromUnmatched: (item: { asset: ResolvedAsset }, roleId: string) => void
    }
    // 初期: arrow.cur → Arrow に自動マッチ、mystery.cur は未マッチ
    expect(vm.matches.map((m) => m.role)).toEqual(['Arrow'])
    expect(vm.unmatched.map((u) => u.asset.sourceFile)).toEqual(['mystery.cur'])

    // mystery.cur をユーザーが Arrow に指定 → arrow.cur が押し出されるはず
    vm.pickRoleFromUnmatched(vm.unmatched[0]!, 'Arrow')
    await nextTick()

    expect(vm.matches.map((m) => m.asset.sourceFile)).toEqual(['mystery.cur'])
    expect(vm.unmatched.map((u) => u.asset.sourceFile)).toEqual(['arrow.cur'])
  })
})
