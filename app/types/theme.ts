/**
 * テーマ関連の型定義 (UI 層と IPC 層で共用)。
 */

/**
 * テーマカードのソース種別。
 *
 * - `local`:   EasyCursorSwap が管理する `.cursorpack` 形式のテーマ。
 *              編集・エクスポート・お気に入り・署名検証の対象。
 * - `system`:  Windows のマウスのプロパティ → ポインター タブに保存された
 *              既存スキーム (`HKCU\Control Panel\Cursors\Schemes`)。
 *              **適用のみ可能** で、編集・エクスポート・お気に入り設定は不可。
 */
export type ThemeKind = 'local' | 'system'

export interface ThemeCardData {
  id: string
  name: string
  author: string | null
  version: string
  /** YYYY-MM-DD or ISO8601 文字列 */
  date: string
  /** 適用回数 */
  applyCount: number
  isFavorite: boolean
  isActive: boolean
  /** 含まれるカーソル役割 ID 一覧 */
  includedRoles: string[]
  /** テーマソース。デフォルトは `local`。 */
  kind?: ThemeKind
  /**
   * テーマタグ (一覧表示の chip 用。例: ["animated", "dark"])。
   * Windows システムスキームには付与されない (常に空配列扱い)。
   */
  tags?: string[]
  /**
   * テーマディレクトリ全体のバイト合計。一覧の「サイズ」列で `formatSize` 越しに表示する。
   * Windows システムスキームには概念が無いので未指定 (`undefined`) になり得る。
   */
  sizeBytes?: number
  /** 署名済みかどうか (Ed25519)。未取得時は `undefined` でフォールバック表示。 */
  signed?: boolean
}
