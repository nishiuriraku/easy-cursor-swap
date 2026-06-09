/**
 * 公式インデックス提出タグ ID (英語 enum, `ALLOWED_MARKETPLACE_TAGS`) を locale 化した
 * 表示ラベルに変換する純関数。提出値は英語 enum のまま、表示だけ i18n キー
 * `marketplace.tag<Capitalized>` (例 `light` → `marketplace.tagLight`) で解決する。
 *
 * 未知タグや未訳キーは `t()` がキー自体を返すため、生のタグ文字列にフォールバックする
 * (UI に i18n キー文字列をそのまま晒さないようにするため)。
 *
 * テンプレートから直接呼ぶと unimport が import を注入できないので、
 * SFC では script 側で `const tagLabel = (tg: string) => marketplaceTagLabel(tg, t)` のように
 * wrap して使うこと (`pickLocalizedName` と同じ制約)。
 */
export function marketplaceTagLabel(tag: string, t: (key: string) => string): string {
  if (!tag) return tag
  const key = `marketplace.tag${tag.charAt(0).toUpperCase()}${tag.slice(1)}`
  const label = t(key)
  return label === key ? tag : label
}
