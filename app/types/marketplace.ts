/**
 * 公式インデックス (Marketplace) 関連の型定義。
 * `nishiuriraku/easy-cursor-swap-index` リポジトリの `index.json` スキーマに準拠。
 */

/**
 * Marketplace エントリの name 表現。
 *
 * Rust 側の `crate::theme::LocalizedString` (`#[serde(untagged)]`) と対称。
 * 後方互換のため 2 形式を許容する:
 *  - **plain string**: 既存の curated index と同じ。全ロケールで同じ値。
 *  - **localized map**: `{ ja: '...', en: '...', default: '...' }` のキー → 値マップ。
 *    UI 側で `useI18n().locale` に応じて表示を切り替える。
 *
 * 表示するときは `composables/pickLocalizedName.ts` の `pickLocalizedName()` を通すこと。
 * 生で `entry.name` を描画すると plain string ケースしか動かず、localized エントリで
 * `[object Object]` が表示されるので注意。
 */
export type MarketplaceName = string | Record<string, string>

export interface MarketplaceEntry {
  /** UUID */
  id: string
  name: MarketplaceName
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
  /**
   * ダウンロード回数。
   * 現状 raw.githubusercontent.com 直 DL のためカウント供給源が無く、index.json では常に 0。
   * UI からは非表示 (FeaturedCard / MarketplaceDetailModal とも DL 数表示を撤去)。
   * Rust 側 (`MarketplaceEntry.download_count`) と index スキーマには互換のため残してある。
   */
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

/**
 * 公式インデックスが受理するタグの enum (allow-list)。
 *
 * Source of truth: `easy-cursor-swap-index/schemas/index-entry.json#tags.items.enum`
 * このリストを変更する際は index repo のスキーマも同時に更新すること (drift 注意)。
 *
 * `MarketplaceTag` (filter UI 側) は `'all'` を含む UI 専用拡張のため、こちらとは別概念。
 */
export const ALLOWED_MARKETPLACE_TAGS = [
  'pixel',
  'minimal',
  'animated',
  'dark',
  'light',
  'anime',
  'retro',
  'neon',
] as const

export type AllowedMarketplaceTag = (typeof ALLOWED_MARKETPLACE_TAGS)[number]
