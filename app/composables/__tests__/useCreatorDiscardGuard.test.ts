/**
 * useCreatorDiscardGuard のテスト (P08a Step 2 / C1)。
 *
 * `vue-router` の `onBeforeRouteLeave` をモックし、guard の保留/confirm/cancel/
 * bypass と保存後遷移スケジュールを検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'

type Guard = (_to: unknown, _from: unknown, next: (proceed?: boolean) => void) => void

let capturedGuard: Guard | null = null

vi.mock('vue-router', () => ({
  onBeforeRouteLeave: (g: Guard) => {
    capturedGuard = g
  },
}))

import { useCreatorDiscardGuard } from '../useCreatorDiscardGuard'

beforeEach(() => {
  capturedGuard = null
  vi.useRealTimers()
})

function setup() {
  const stage = ref<'start' | 'editing'>('editing')
  const assignedRoleCount = ref(0)
  const isMetaDirty = ref(false)
  const onReset = vi.fn()
  const navigate = vi.fn()
  const guard = useCreatorDiscardGuard({
    stage,
    assignedRoleCount,
    isMetaDirty,
    onReset,
    navigate,
  })
  return { stage, assignedRoleCount, isMetaDirty, onReset, navigate, guard }
}

describe('useCreatorDiscardGuard', () => {
  it('hasUnsavedEdits is false outside editing stage', () => {
    const { stage, guard } = setup()
    stage.value = 'start'
    expect(guard.hasUnsavedEdits.value).toBe(false)
  })

  it('requestReset calls onReset directly when nothing to discard', () => {
    const { onReset, guard } = setup()
    guard.requestReset()
    expect(onReset).toHaveBeenCalledTimes(1)
    expect(guard.discardDialogOpen.value).toBe(false)
  })

  it('requestReset opens dialog in clear mode when edits exist', () => {
    const { assignedRoleCount, guard } = setup()
    assignedRoleCount.value = 2
    guard.requestReset()
    expect(guard.discardDialogOpen.value).toBe(true)
    expect(guard.discardDialogMode.value).toBe('clear')
  })

  it('route guard holds navigation and confirm proceeds', () => {
    const { assignedRoleCount, onReset, guard } = setup()
    assignedRoleCount.value = 1
    const next = vi.fn()
    capturedGuard!({}, {}, next)
    expect(next).not.toHaveBeenCalled()
    expect(guard.discardDialogMode.value).toBe('navigate')
    guard.onDiscardConfirm()
    expect(next).toHaveBeenCalledWith(true)
    expect(onReset).not.toHaveBeenCalled()
  })

  it('route guard cancel calls next(false)', () => {
    const { isMetaDirty, guard } = setup()
    isMetaDirty.value = true
    const next = vi.fn()
    capturedGuard!({}, {}, next)
    guard.onDiscardCancel()
    expect(next).toHaveBeenCalledWith(false)
  })

  it('bypassUnsavedGuard passes navigation through immediately', () => {
    const { assignedRoleCount, guard } = setup()
    assignedRoleCount.value = 3
    guard.bypassUnsavedGuard.value = true
    const next = vi.fn()
    capturedGuard!({}, {}, next)
    expect(next).toHaveBeenCalledWith()
    expect(guard.discardDialogOpen.value).toBe(false)
  })

  it('scheduleNavigateAfterSave sets bypass and navigates after delay', async () => {
    vi.useFakeTimers()
    const { navigate, guard } = setup()
    guard.scheduleNavigateAfterSave('/', 1000)
    expect(guard.bypassUnsavedGuard.value).toBe(false)
    await vi.advanceTimersByTimeAsync(1000)
    expect(guard.bypassUnsavedGuard.value).toBe(true)
    expect(navigate).toHaveBeenCalledWith('/')
    vi.useRealTimers()
  })
})
