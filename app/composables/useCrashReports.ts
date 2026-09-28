/**
 * クラッシュレポート関連 IPC (crash.rs ドメイン) の集約。
 * settings.vue が直接 invoke していた 3 コマンド
 * (list_crash_reports / submit_crash_reports / clear_crash_reports) をまとめる。
 * 凝集した 3 操作なので grab-bag な「system actions」ではなく専用 composable にする。
 */
import type { CrashSubmitSummary } from '~/types/config'

export function useCrashReports() {
  /** 保存済みクラッシュレポートを取得する (件数表示用)。 */
  function listCrashReports(): Promise<unknown[] | null> {
    return invokeTauri<unknown[]>('list_crash_reports')
  }
  /** 保留中レポートを Cloudflare Worker に送信する (opt-in)。 */
  function submitCrashReports(): Promise<CrashSubmitSummary | null> {
    return invokeTauri<CrashSubmitSummary>('submit_crash_reports')
  }
  /** 保存済みレポートを全削除する。削除件数を返す。 */
  function clearCrashReports(): Promise<number | null> {
    return invokeTauri<number>('clear_crash_reports')
  }
  return { listCrashReports, submitCrashReports, clearCrashReports }
}

export interface CrashReportsStateDeps {
  t: (key: string, params?: Record<string, string | number>) => string
  isOptedIn: () => boolean
}

/**
 * 設定画面のクラッシュレポート UI 状態 (P08a Step 3 / S5)。
 *
 * 件数は `list_crash_reports` の戻り長、メッセージは送信/クリア後の
 * ユーザー向けトースト相当の文字列。
 */
export function useCrashReportsState(deps: CrashReportsStateDeps) {
  const { t, isOptedIn } = deps
  const { listCrashReports, submitCrashReports, clearCrashReports } = useCrashReports()
  const count = ref(0)
  const busy = ref(false)
  const message = ref<string | null>(null)

  async function load() {
    try {
      const reports = await listCrashReports()
      count.value = reports?.length ?? 0
    } catch (err) {
      message.value = t('settings.crashLoadFailed', {
        error: err instanceof Error ? err.message : String(err),
      })
    }
  }

  async function submit() {
    busy.value = true
    message.value = null
    try {
      const summary = await submitCrashReports()
      if (!summary || (summary.sent === 0 && summary.failed === 0 && summary.skipped === 0)) {
        // 全部 0 のときは「opt-in OFF」か「ビルド時 env 未設定」のどちらかだが、
        // フロントからは区別できないため crash_reporting フラグで判定する。
        message.value = isOptedIn()
          ? t('settings.crashSubmitNoCredentials')
          : t('settings.crashSubmitOptedOut')
      } else {
        message.value = t('settings.crashSubmitResult', {
          sent: summary.sent,
          failed: summary.failed,
          skipped: summary.skipped,
        })
      }
      await load()
    } catch (err) {
      message.value = err instanceof Error ? err.message : String(err)
    } finally {
      busy.value = false
    }
  }

  async function clear() {
    busy.value = true
    message.value = null
    try {
      const removed = await clearCrashReports()
      message.value = t('settings.crashClearedCount', { count: removed })
      await load()
    } catch (err) {
      message.value = err instanceof Error ? err.message : String(err)
    } finally {
      busy.value = false
    }
  }

  return { count, busy, message, load, submit, clear }
}
