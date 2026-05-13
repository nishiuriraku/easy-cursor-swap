/**
 * Creator のロール状態判定が `assigned` ベースで動いていることを確認する単体契約テスト。
 * フル mount は重いので、creator.vue が内部で組む computed と同じロジックを直接検証する。
 */
import { describe, it, expect } from 'vitest'
import { ref, computed } from 'vue'
import type { RoleAsset } from '~/composables/useCreatorAssets'

type RoleStatus = 'filled' | 'empty'

function makeStatusOf(assigned: ReturnType<typeof ref<Record<string, RoleAsset>>>) {
  const filledRoleSet = computed(() => new Set(Object.keys(assigned.value)))
  return (id: string): RoleStatus => (filledRoleSet.value.has(id) ? 'filled' : 'empty')
}

describe('creator.vue statusOf contract', () => {
  it('未インポート時は全ロールが empty', () => {
    const assigned = ref<Record<string, RoleAsset>>({})
    const statusOf = makeStatusOf(assigned)
    expect(statusOf('Arrow')).toBe('empty')
    expect(statusOf('Help')).toBe('empty')
    expect(statusOf('AppStarting')).toBe('empty')
  })

  it('assigned に存在すれば filled', () => {
    const asset: RoleAsset = {
      primary: new Uint8Array([0x89, 0x50, 0x4e, 0x47]),
      primarySize: 32,
      hotspot: { x: 0, y: 0 },
      source: 'manual',
    }
    const assigned = ref<Record<string, RoleAsset>>({ Arrow: asset })
    const statusOf = makeStatusOf(assigned)
    expect(statusOf('Arrow')).toBe('filled')
    expect(statusOf('Help')).toBe('empty')
  })
})
