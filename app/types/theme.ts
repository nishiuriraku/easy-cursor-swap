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
  /**
   * 最終適用日時 (RFC3339)。一度も適用されていない場合は `null`。
   * 「最近使用」フィルタの判定に使う。Windows システムスキームは未追跡 (`null`)。
   */
  lastAppliedAt?: string | null
  /**
   * theme.json `description` を `"ja"` 解決した文字列。
   * `null/undefined` のとき UI は説明段落を非表示にする。
   * Windows システムスキームは `null`。
   */
  description?: string | null
  /**
   * theme.json `schema_version`。詳細モーダルの PACKAGE セルで `schema v{n}` 表記に使う。
   * Windows システムスキームは `undefined` (UI 側で `system scheme` 表記にフォールバック)。
   */
  schemaVersion?: number
  /**
   * theme.json `license` (SPDX)。`null/undefined` のとき行ごと非表示。
   */
  license?: string | null
  /**
   * theme.json `homepage`。`null/undefined` のとき行ごと非表示。
   * クリック時は `open_url` IPC 経由で外部ブラウザに渡す。
   */
  homepage?: string | null
}
