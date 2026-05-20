/**
 * テーマ関連の型定義 (UI 層と IPC 層で共用)。
 */

/**
 * テーマカードのソース種別。
 *
 * - `local`:        ユーザー作成 / 手動取り込みテーマ。編集可・エクスポート可。
 * - `system`:       Windows のマウスのプロパティの既存スキーム。**適用のみ可**。
 * - `marketplace`:  公式インデックスから取得したテーマ。**編集とエクスポート不可**。
 *                   適用・お気に入り・複製・削除は可能。複製先は `local` 扱いになる。
 */
export type ThemeKind = 'local' | 'system' | 'marketplace'

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
   * theme.json `description` を現在のロケールで解決した文字列。
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
  /**
   * 公式インデックス由来テーマを `duplicate_theme` で複製した場合に複製元の UUID。
   * SubmitThemeDialog はこのフィールドが truthy なテーマを提出選択肢から除外し、
   * 詳細モーダルでは「公式テーマの複製」バッジ表示判定にも使う。
   * 通常の Local テーマ (Creator 新規 / .cursorpack 取り込み) は `null/undefined`。
   */
  clonedFromMarketplaceId?: string | null
}
