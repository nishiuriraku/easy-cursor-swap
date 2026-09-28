/**
 * Library ページ (`pages/index.vue`) の IPC 型と Card への純関数マッピング。
 *
 * 過去にここで `kind: 'local' as const` をハードコードしていたため、
 * `theme.json` の `source` フィールドが UI に届かず MARKETPLACE タグや
 * readonly ガードが効かないバグがあった (2026-05-14 修正)。
 */
import type { ThemeCardData } from '~/types/theme'
import type { IpcThemeSummary } from '~/types/theme-ipc'
export type { IpcThemeSummary } from '~/types/theme-ipc'
import type { IpcWindowsScheme } from '~/composables/useWindowsSchemes'
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
// `IpcThemeSummary` の正準定義は `~/types/theme-ipc.ts` (Wave 2B / Task 5d)。

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

export type FilterChip = 'all' | 'favorites' | 'recent'
/** 並び替えキー。
 *  - `updated` / `name` / `applied`: グリッド・一覧表示の両方で使う既存キー
 *  - `coverage` / `size`: 一覧表示のヘッダクリック用に追加
 *  なお Q2 で「sortKey/sortDir はグリッドと一覧で共有」と決定したため、
 *  グリッド側のソート巡回ボタンも増えたキーを順番に巡回する。 */
export type SortKey = 'name' | 'updated' | 'applied' | 'coverage' | 'size'
export type SortDir = 'asc' | 'desc'

export interface ThemeFilter {
  query: string
  filter: FilterChip
}

export function filterThemes(themes: ThemeCardData[], opts: ThemeFilter): ThemeCardData[] {
  let result = [...themes]
  if (opts.query.trim()) {
    const q = opts.query.toLowerCase()
    result = result.filter(
      (t) => t.name.toLowerCase().includes(q) || (t.author?.toLowerCase().includes(q) ?? false),
    )
  }
  if (opts.filter === 'favorites') result = result.filter((tt) => tt.isFavorite)
  else if (opts.filter === 'recent')
    result = result.filter((tt) => Boolean(tt.lastAppliedAt) || tt.applyCount > 0)
  return result
}

export function sortThemes(
  themes: ThemeCardData[],
  sortKey: SortKey,
  sortDir: SortDir,
): ThemeCardData[] {
  // Q2: sortKey/sortDir はグリッドと一覧で共有。sortDir で昇降を切替。
  const result = [...themes]
  const dirSign = sortDir === 'asc' ? 1 : -1
  result.sort((a, b) => {
    let cmp = 0
    switch (sortKey) {
      case 'name':
        cmp = a.name.localeCompare(b.name, 'ja')
        break
      case 'updated':
        cmp = a.date.localeCompare(b.date)
        break
      case 'applied':
        cmp = a.applyCount - b.applyCount
        break
      case 'coverage':
        cmp = a.includedRoles.length - b.includedRoles.length
        break
      case 'size':
        cmp = (a.sizeBytes ?? 0) - (b.sizeBytes ?? 0)
        break
    }
    return dirSign * cmp
  })
  return result
}

export function countThemes(themes: ThemeCardData[]) {
  return {
    all: themes.length,
    favorites: themes.filter((tt) => tt.isFavorite).length,
    recent: themes.filter((tt) => Boolean(tt.lastAppliedAt) || tt.applyCount > 0).length,
  }
}

/**
 * Windows レジストリのスキームを ThemeCardData に変換する。
 *
 * - id は `windows:<name>` のプレフィックスでローカルテーマと衝突を避ける
 * - kind: 'system' を立てて UI 側でバッジ・編集不可表示に切り替える
 * - included_roles は cursor_paths のキー (空でないもの) を使う
 * - active 判定は Rust 側の `paths_match_current_registry` の結果 (`is_active`)
 *   をそのまま採用する。フロントで再判定すると IPC 往復が増えるため。
 */
export function mapWindowsSchemeToCard(s: IpcWindowsScheme, authorLabel: string): ThemeCardData {
  const includedRoles = Object.entries(s.cursor_paths)
    .filter(([, path]) => path.length > 0)
    .map(([role]) => role)
  return {
    id: `windows:${s.name}`,
    name: s.name,
    author: authorLabel,
    version: '—',
    date: '',
    applyCount: 0,
    isFavorite: false,
    isActive: s.is_active === true,
    includedRoles,
    kind: 'system',
    // Windows システムスキームに付随しない情報。一覧表示では tags = []、
    // sizeBytes = undefined ('—' 表示)、signed = false として扱う。
    // signed=false の理由: Marketplace の Ed25519 検証済テーマ (= "公式" バッジ) と
    // OS 提供スキームは別概念。Windows のものは「信頼できる」が「公式インデックス由来」では
    // ないため、ThemeDetailDrawer の「公式」ピルを点灯させない。
    tags: [],
    sizeBytes: undefined,
    signed: false,
    lastAppliedAt: null,
    description: null,
    schemaVersion: undefined,
    license: null,
    homepage: null,
  }
}
