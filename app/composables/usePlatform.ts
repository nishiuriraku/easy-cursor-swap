/**
 * 実行 OS に依存する UI 文言 / 判定の集約点。
 * 現状 OS は 'windows' 定数。P02/P05 で Rust `get_platform_info` を導入したら
 * `syncFromBackend()` を足して `os` を書き換える (UI 側の変更は不要になる設計)。
 */
export type PlatformOs = 'windows'

const os = ref<PlatformOs>('windows')

/** CATALOG など静的キーが必要な場所向け。`platform.<os>.<key>` を組み立てる。 */
export function platformKey(key: string, forOs: PlatformOs = os.value): string {
  return `platform.${forOs}.${key}`
}

export function usePlatform() {
  const { t } = useI18n()
  /** `t('platform.<os>.<key>', params)` の短縮。 */
  function tp(key: string, params?: Record<string, string | number>): string {
    return t(platformKey(key), params)
  }
  return { os: readonly(os), tp, platformKey }
}
