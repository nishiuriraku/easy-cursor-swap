/**
 * ファイル名 → ロール ID マッチングのためのファジーマッチャ。
 * 純粋関数モジュール (Rust 非依存)。
 */

export function normalize(name: string): string {
  return name
    .toLowerCase()
    .replace(/\.[a-z0-9]+$/, '')
    .replace(/v\d+(\.\d+)*/g, '')
    .replace(/\d+x\d+/g, '')
    .replace(/\d+(?:px)?(?=[._\s\-]|$)/g, '')
    .replace(/[\s_\-.]+/g, '')
}
