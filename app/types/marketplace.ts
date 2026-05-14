/**
 * 公式インデックス (Marketplace) 関連の型定義。
 * `nishiuriraku/easy-cursor-swap-index` リポジトリの `index.json` スキーマに準拠。
 */

export interface MarketplaceEntry {
  /** UUID */
  id: string
  name: string
  author: string
  /** GitHub username (公開鍵照合に使用) */
  authorGithub: string
  homepage?: string
  /** ZIP の SHA-256 (16進文字列) */
  sha256: string
  /** Ed25519 署名 (Base64) */
  signature: string
  /** 公開鍵 ID (公開鍵 SHA-256 の先頭 16 文字) */
  authorPubkeyId: string
  /** 直接ダウンロード URL */
  downloadUrl: string
  version: string
  /** ダウンロード回数 (CI 集計値) */
  downloadCount: number
  /** プレビュー用の役割 ID 一覧 */
  includedRoles: string[]
  /** タグ (Pixel / Minimal / Animated / Dark など) */
  tags: string[]
  /** "新着" "人気" などの強調ラベル */
  highlight?: 'new' | 'popular' | null
  /** 検証ステータス: signature 検証 + マルウェアハッシュチェック完了 */
  verified: boolean
  /**
   * 公式インデックス側 previews/<uuid>/ のベース URL。
   * 詳細モーダルで <role>.png を組み立てて取得するために使う。
   * 未定義の場合はサムネ表示を SVG にフォールバック。
   */
  previewBaseUrl?: string
}

export type MarketplaceTag = 'all' | 'pixel' | 'minimal' | 'animated' | 'dark'
