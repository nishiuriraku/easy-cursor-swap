/**
 * 公式インデックス (Marketplace) 関連の型定義。
 * `nishiuriraku/easy-cursor-swap-index` リポジトリの `index.json` スキーマに準拠。
 *
 * 構造体型は `ts-rs` で `app/types/generated/` に自動生成される
 * (`cargo run --manifest-path src-tauri/Cargo.toml --features typegen --bin gen_types`)。
 * このファイルは **stable な再エクスポート面 (shim)** として機能し、
 * 生成物のファイル名が変わっても既存 import
 * (`import type { MarketplaceEntry } from '~/types/marketplace'`) を壊さない。
 *
 * ここで手書きするのは UI 専用のフィルタ型 (MarketplaceTag / ALLOWED_MARKETPLACE_TAGS /
 * AllowedMarketplaceTag) と、generated に無い旧名 (MarketplaceName) の後方互換 alias のみ。
 */

export type {
  BackupInfo,
  MarketplaceEntry,
  MarketplaceIndex,
  MarketplaceInstallRequest,
} from './generated'

import type { LocalizedString } from './generated'

/**
 * Marketplace エントリの name 表現。
 *
 * Rust 側 `crate::theme::LocalizedString` (`#[serde(untagged)]`) と対称で、
 * 生成出力は `LocalizedString` 名。`MarketplaceName` は旧 hand-written 名の
 * 後方互換 alias として残している。
 *
 * 表示するときは `composables/pickLocalizedName.ts` の `pickLocalizedName()` を通すこと。
 * 生で `entry.name` を描画すると plain string ケースしか動かず、localized エントリで
 * `[object Object]` が表示されるので注意。
 *
 * @deprecated generated 側の `LocalizedString` を直接 import してください。
 *             この alias は既存呼び出しを壊さないための一時的なシムです。
 */
export type MarketplaceName = LocalizedString

/**
 * UI フィルタ専用タグ列挙。`'all'` を含むのは UI 都合のためで、公式インデックス
 * のスキーマ (`easy-cursor-swap-index/schemas/index-entry.json#tags.items.enum`)
 * とは別概念。`ALLOWED_MARKETPLACE_TAGS` 側だけが index repo と同期する。
 */
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
