/**
 * CursorPreview の ani→ani 切替リグレッションテスト。
 *
 * ロールを跨いで別の .ani asset に差し替えたとき、内部の AniThumb が再マウントされ
 * `useAniPlayer` が新しい framePngs から blob URL を再生成して `<img>` の src が
 * 更新されることを確認する。修正前 (`:key` 無し) は AniThumb が再利用され、
 * setup() 時に焼き付けた frameUrls が固定されるため src が変化せず失敗する。
 */
import { describe, it, expect } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import CursorPreview from '../CursorPreview.vue'

function makeAniAsset(seed: number) {
  return {
    kind: 'ani' as const,
    framePngs: [new Uint8Array([seed, seed + 1, seed + 2])],
    sequence: [0],
    durations: [100],
    nativeSize: 64,
  }
}

describe('CursorPreview ani→ani 切替', () => {
  it('framePngs が別配列に変わると <img> の src が更新される', async () => {
    const wrapper = mount(CursorPreview, {
      props: {
        asset: makeAniAsset(1),
        hotspot: { x: 0, y: 0 },
      },
      global: {
        stubs: {
          CursorIcon: { template: '<span></span>' },
        },
      },
    })

    await flushPromises()
    const initialSrc = wrapper.find('img').attributes('src')
    expect(initialSrc).toBeTruthy()

    await wrapper.setProps({ asset: makeAniAsset(99) })
    await flushPromises()

    const updatedSrc = wrapper.find('img').attributes('src')
    expect(updatedSrc).toBeTruthy()
    expect(updatedSrc).not.toBe(initialSrc)
  })
})
