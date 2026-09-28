/**
 * useLibraryImportFlow のテスト (P08a Step 4 / L3)。
 *
 * conflict 分岐・confirm 上書き・open null no-op を検証する。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest'

const inspectMock = vi.fn()
const importMock = vi.fn()

vi.mock('~/composables/useThemes', () => ({
  useThemes: () => ({
    inspectCursorpack: inspectMock,
    importCursorpack: importMock,
  }),
}))

import { useLibraryImportFlow } from '../useLibraryImportFlow'

const t = (key: string) => key

function setup() {
  const reload = vi.fn(async () => {})
  const errors: Array<string | null> = []
  const notified: string[] = []
  const flow = useLibraryImportFlow({
    reload,
    setError: (m) => errors.push(m),
    notifyImported: (id) => notified.push(id),
    t,
  })
  return { reload, errors, notified, flow }
}

const inspection = (existing: boolean) => ({
  id: 'x',
  name: 'X',
  version: '1',
  author: null,
  role_count: 3,
  existing: existing
    ? { name: 'X', version: '1', author: null, role_count: 3 }
    : null,
})

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useLibraryImportFlow', () => {
  it('opens conflict dialog instead of importing on existing', async () => {
    inspectMock.mockResolvedValueOnce(inspection(true))
    const { flow } = setup()
    await flow.importByPath('/a/x.cursorpack')
    expect(flow.conflictDialog.value?.path).toBe('/a/x.cursorpack')
    expect(importMock).not.toHaveBeenCalled()
  })

  it('confirmConflictOverwrite imports the pending path', async () => {
    inspectMock.mockResolvedValueOnce(inspection(true))
    importMock.mockResolvedValueOnce('new-id')
    const { notified, flow } = setup()
    await flow.importByPath('/a/x.cursorpack')
    await flow.confirmConflictOverwrite()
    expect(importMock).toHaveBeenCalledWith('/a/x.cursorpack')
    expect(flow.conflictDialog.value).toBeNull()
    expect(notified).toEqual(['new-id'])
  })

  it('imports directly when no conflict', async () => {
    inspectMock.mockResolvedValueOnce(inspection(false))
    importMock.mockResolvedValueOnce('id-1')
    const { notified, flow } = setup()
    await flow.importByPath('/a/y.cursorpack')
    expect(flow.conflictDialog.value).toBeNull()
    expect(notified).toEqual(['id-1'])
    expect(flow.importBusy.value).toBe(false)
  })
})
