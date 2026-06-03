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
