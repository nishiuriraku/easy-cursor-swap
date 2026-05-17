/**
 * アプリ起動時の自動アップデートチェック (Phase 8-5 / B)。
 *
 * 仕様:
 *   - `general.auto_update === true` の時のみ check を実行
 *   - 前回チェックから **24 時間** 未満ならスキップ (`localStorage` の `last_update_check_at` を見る)
 *   - 更新あり → Windows Toast で告知。クリックでアプリ前面化 (= 設定 → 更新 から手動 DL)
 *   - 更新なし / エラー / Tauri 未接続 → 静かに無視
 *
 * 設計判断:
 *   - 「Rust が Source of Truth」原則の例外 — `last_update_check_at` は **UI 側のレート
 *     リミットのためだけ** の値で、Rust の Updater が見るわけではない。
 *     localStorage が消えても再チェックが走るだけで副作用なし → 設定ファイルの schema を汚さない。
 *   - 通知の permission リクエストは `useNotify` 側でキャッシュ済 (起動時に 1 度だけ)。
 */
import { useAppSettings } from './useAppSettings'
import { useUpdater } from './useUpdater'
import { notify } from './useNotify'
import { invokeTauri } from './useTauri'

/** クールダウン期間 (ms)。24 時間。 */
const CHECK_COOLDOWN_MS = 24 * 60 * 60 * 1000

/** localStorage キー (composable 専用)。 */
const LAST_CHECK_KEY = 'ecs.updater.last_check_at'

function readLastCheckedAt(): number {
  if (typeof localStorage === 'undefined') return 0
  const raw = localStorage.getItem(LAST_CHECK_KEY)
  if (!raw) return 0
  const v = Number(raw)
  return Number.isFinite(v) ? v : 0
}

function writeLastCheckedAt(ts: number): void {
  if (typeof localStorage === 'undefined') return
  try {
    localStorage.setItem(LAST_CHECK_KEY, String(ts))
  } catch {
    // QuotaExceeded などは無視 (このフィールドは消えても再チェックが走るだけ)
  }
}

/**
 * 起動時に 1 度だけ呼ぶ。条件を満たせば非同期で check → 通知を行う。
 *
 * 呼び元は await 不要 — 失敗しても起動を止めない fire-and-forget。
 */
export function bootstrapUpdaterCheck(): void {
  void run().catch((e) => {
    console.warn('[updater-bootstrap] failed:', e)
  })
}

async function run(): Promise<void> {
  const { load, config } = useAppSettings()
  await load()
  const c = config.value
  if (!c) return

  if (!c.general.auto_update) {
    return
  }

  const now = Date.now()
  const last = readLastCheckedAt()
  if (now - last < CHECK_COOLDOWN_MS) {
    return
  }

  const { check } = useUpdater()
  const info = await check()

  // 成功・失敗にかかわらずタイムスタンプは進める
  // (失敗時に毎回再試行すると Toast permission ダイアログが頻発する)
  writeLastCheckedAt(now)

  if (!info) return

  // メジャー跨ぎは startup auto-check では通知しない (UX: 大きな変更はユーザー意思で
  // 設定 → 更新 から DL してもらう)。
  // check_update_is_major_jump IPC が失敗 (Tauri 未接続 / dev モード等) した場合は
  // 安全側で通知を出す既定挙動にフォールバックする。
  try {
    const isMajor = await invokeTauri<boolean>('check_update_is_major_jump', {
      currentVersion: info.currentVersion,
      newVersion: info.version,
    })
    if (isMajor) {
      console.info('[updater-bootstrap] major bump detected, suppressing toast')
      return
    }
  } catch (e) {
    console.warn('[updater-bootstrap] major bump check failed, falling through to notify:', e)
  }

  await notify({
    title: 'EasyCursorSwap',
    body: `新しいバージョン v${info.version} が利用可能です。設定 → 更新からダウンロードできます。`,
    level: 'info',
  })
}
