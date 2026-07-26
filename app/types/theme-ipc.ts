/**
 * Theme IPC ペイロードの正準型 (Wave 2B / Task 5d)。
 *
 * Rust 側 `src-tauri/src/theme/types.rs::ThemeSummary` に対応。
 * フィールド名は serde 既定の snake_case のままで、フロント UI 層
 * (`ThemeCardData`) には各 consumer の mapper で camelCase に揃えてコピーする。
 *
 * `name` / `description` は Rust 側 `LocalizedString` (`#[serde(untagged)]`) の
 * 生形 (`string | { [locale]: string }`) で受け取り、表示時に
 * `composables/pickLocalizedName` を介して現在の locale に解決する。
 * Rust 側で固定ロケールに解決していた頃は英語 UI ユーザーが日本語名の
 * テーマを "矢印" のように見ることになっていた (audit D1/D2)。
 *
 * 過去に description / signed / tags / size_bytes / last_applied_at /
 * schema_version / license / homepage を **取りこぼしていた** ため、
 * テーマ詳細モーダルの DESCRIPTION 段落が出ず、ThemeRow の signed 判定が
 * 全テーマ "署名済" 扱いになるバグの原因になっていた (Rust を真とする)。
 *
 * 旧 `composables/useThemes.ts` / `pages/index.helpers.ts` /
 * `components/marketplace/SubmitThemeDialog.vue` に重複していた宣言を集約。
 * 3 つの consumer は本ファイルを import する。新たな consumer を足す場合も
 * ここに集約すること (= local コピーを作らない)。
 */
import type { MarketplaceName } from './marketplace'

export interface IpcThemeSummary {
  id: string
  /** LocalizedString の生形。表示時は `pickLocalizedName` で解決。 */
  name: MarketplaceName
  author: string | null
  version: string
  /** 作成日時 (RFC3339)。 */
  created_at: string
  is_active: boolean
  is_favorite: boolean
  apply_count: number
  /** 最終適用日時 (RFC3339)。一度も適用されていなければ `null`。 */
  last_applied_at: string | null
  included_roles: string[]
  /** テーマディレクトリのパス。 */
  path: string
  tags: string[]
  /** テーマディレクトリ全体の合計サイズ (bytes)。 */
  size_bytes: number
  /** 署名済みか (Ed25519)。**検証結果ではない** — 検証は marketplace::verify_signature 側。 */
  signed: boolean
  /** theme.json `description` の生データ (多言語対応)。`null/undefined` のとき UI は非表示。 */
  description?: MarketplaceName | null
  /** theme.json `schema_version`。 */
  schema_version: number
  /** theme.json `license` (SPDX)。 */
  license?: string | null
  /** theme.json `homepage`。 */
  homepage?: string | null
  /**
   * theme.json `source` フィールド。Rust `ThemeSource` (serde lowercase) の
   * `"local" | "marketplace"`。フロント側で `mapSourceToKind` を介して
   * `ThemeCardData.kind` (local / system / marketplace) に変換する。
   */
  source?: string
  /**
   * 公式インデックス由来テーマを `duplicate_theme` で複製した場合に
   * 複製元 (Marketplace 原本) の UUID。`truthy` なテーマは
   * `SubmitThemeDialog` の提出可能一覧から除外される
   * (Rust 側 `submit_theme_auto` も同条件で hard-reject する二重防御)。
   */
  cloned_from_marketplace_id?: string | null
}