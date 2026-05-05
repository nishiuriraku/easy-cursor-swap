/**
 * テーマ関連の型定義 (UI 層と IPC 層で共用)。
 */

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
}
