/**
 * Library のインポートフロー (P08a Step 4 / L3)。
 *
 * `.cursorpack` 検査 → 衝突ダイアログ → 取込 → 通知の一連をまとめる。
 * `useThemes()` の inspect/import は内包する。
 */
import { useThemes } from '~/composables/useThemes'

export interface ImportConflictPending {
  path: string
  info: {
    id: string
    name: string
    version: string
    author: string | null
    roleCount: number
    existing: {
      name: string
      version: string
      author: string | null
      roleCount: number
    }
  }
}

export interface LibraryImportFlowDeps {
  reload: () => Promise<void>
  setError: (msg: string | null) => void
  notifyImported: (id: string) => void
  t: (key: string, params?: Record<string, string | number>) => string
}

export function useLibraryImportFlow(deps: LibraryImportFlowDeps) {
  const { reload, setError, notifyImported, t } = deps
  const { inspectCursorpack: inspectCursorpackIpc, importCursorpack: importCursorpackIpc } =
    useThemes()

  // インポート衝突ダイアログ用
  const conflictDialog = ref<ImportConflictPending | null>(null)
  const importBusy = ref(false)

  async function actuallyImport(path: string) {
    const id = await importCursorpackIpc(path)
    if (id) {
      console.info('[Library] imported', id, 'from', path)
      await reload()
      notifyImported(id)
    }
  }

  async function importByPath(path: string) {
    importBusy.value = true
    try {
      // まず軽量検査して既存テーマと衝突するか確認
      const inspection = await inspectCursorpackIpc(path)
      if (inspection?.existing) {
        conflictDialog.value = {
          path,
          info: {
            id: inspection.id,
            name: inspection.name,
            version: inspection.version,
            author: inspection.author,
            roleCount: inspection.role_count,
            existing: {
              name: inspection.existing.name,
              version: inspection.existing.version,
              author: inspection.existing.author,
              roleCount: inspection.existing.role_count,
            },
          },
        }
        return
      }
      await actuallyImport(path)
    } catch (err) {
      const msg = appErrorMessage(err)
      setError(t('library.errImport', { detail: msg }))
      console.error('[Library] import failed:', err)
    } finally {
      importBusy.value = false
    }
  }

  async function confirmConflictOverwrite() {
    const pending = conflictDialog.value
    if (!pending) return
    conflictDialog.value = null
    importBusy.value = true
    try {
      await actuallyImport(pending.path)
    } catch (err) {
      const msg = appErrorMessage(err)
      setError(t('library.errImport', { detail: msg }))
    } finally {
      importBusy.value = false
    }
  }

  async function openImportDialog() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({
        multiple: true,
        filters: [{ name: 'Cursor Pack', extensions: ['cursorpack'] }],
      })
      if (!selected) return
      const paths = Array.isArray(selected) ? selected : [selected]
      for (const p of paths) await importByPath(p)
    } catch (err) {
      console.warn('[Library] dialog unavailable:', err)
    }
  }

  return {
    importBusy,
    conflictDialog,
    importByPath,
    confirmConflictOverwrite,
    openImportDialog,
  }
}
