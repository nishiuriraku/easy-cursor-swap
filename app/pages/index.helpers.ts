/**
 * Library ページ (`pages/index.vue`) の IPC 型と Card への純関数マッピング。
 *
 * 過去にここで `kind: 'local' as const` をハードコードしていたため、
 * `theme.json` の `source` フィールドが UI に届かず MARKETPLACE タグや
 * readonly ガードが効かないバグがあった (2026-05-14 修正)。
 */
import type { ThemeCardData } from '~/types/theme'
import type { MarketplaceName } from '~/types/marketplace'
import { mapSourceToKind } from '~/composables/useThemes'
import { pickLocalizedName } from '~/composables/pickLocalizedName'

/**
 * Rust 側 `theme::types::ThemeSummary` に対応する IPC ペイロード。
 * フィールド名は serde 既定 (snake_case)。`useThemes.ts` の
 * 同名インターフェースと意図的に重複しているが、Library 画面側は
 * Windows scheme をマージする独自経路を持つため別ファイルで持つ。
 *
 * `name` / `description` は Rust 側 `LocalizedString` の生形 (`string | { [locale]: string }`)
 * で渡ってくる。フロントでカードに乗せる前に `pickLocalizedName` で
 * 現在の locale に解決する。生で表示すると `{ja: "...", en: "..."}` という
 * JSON 風のテキストがそのままカードのタイトル欄に出る。
 */
export interface IpcThemeSummary {
  id: string
  name: MarketplaceName
  author: string | null
  version: string
  created_at: string
  is_active: boolean
  is_favorite: boolean
  apply_count: number
  included_roles: string[]
  path: string
  tags: string[]
  size_bytes: number
  signed: boolean
  last_applied_at: string | null
  description?: MarketplaceName | null
  schema_version: number
  license?: string | null
  homepage?: string | null
  /** `theme.json` の `source` フィールド。`mapSourceToKind` 経由で `kind` に反映する。 */
  source?: string
}

export function mapLocalSummaryToCard(tt: IpcThemeSummary, locale: string): ThemeCardData {
  return {
    id: tt.id,
    name: pickLocalizedName(tt.name, locale),
    author: tt.author,
    version: tt.version,
    date: tt.created_at,
    applyCount: tt.apply_count,
    isFavorite: tt.is_favorite,
    isActive: tt.is_active,
    includedRoles: tt.included_roles,
    kind: mapSourceToKind(tt.source),
    tags: tt.tags,
    sizeBytes: tt.size_bytes,
    signed: tt.signed,
    lastAppliedAt: tt.last_applied_at,
    description: tt.description == null ? null : pickLocalizedName(tt.description, locale) || null,
    schemaVersion: tt.schema_version,
    license: tt.license ?? null,
    homepage: tt.homepage ?? null,
  }
}
