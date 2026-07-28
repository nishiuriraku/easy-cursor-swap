/**
 * Updater 関連の定数 (Wave 2B / Task 5)。
 *
 * 旧 `useUpdaterBootstrap.ts` / `app/pages/settings.vue` に重複していた
 * `CHECK_COOLDOWN_MS` / `LAST_CHECK_KEY` を 1 箇所に集約し、両者が同じ
 * localStorage キーと 24 時間クールダウンを見る契約を型レベルで強制する。
 *
 * 追加の変更を加える際は settings.vue の autoCheck ヘルパもこの定数を
 * import すること (= 別 constant の local コピーを作らない)。
 */

/** クールダウン期間 (ms)。24 時間。 */
export const UPDATE_CHECK_COOLDOWN_MS = 24 * 60 * 60 * 1000

/** localStorage キー (composable 専用)。 */
export const LAST_UPDATE_CHECK_KEY = 'ecs.updater.last_check_at'
