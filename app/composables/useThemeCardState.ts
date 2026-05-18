/**
 * `ThemeCard` (グリッド表示) と `ThemeRow` (リスト表示) で 5 ブロック並行重複して
 * いたプレゼンテーション状態を共通化する composable (audit B9-DUP-001)。
 *
 * 共通化対象:
 *  1. previewMap fetch (`useThemePreviews().getMap(theme.id)` の onMounted + watch)
 *  2. `isSystem` / `isMarketplace` の kind 判定
 *  3. `displayDate` の ISO トリム
 *  4. クリック / Enter / Space で詳細を開くハンドラ (内側ボタン stopPropagation)
 *  5. お気に入りトグル (system は禁止 + stopPropagation)
 *
 * 利用側 (`ThemeCard.vue` / `ThemeRow.vue`) は template + style + (Row 固有の
 * displaySize / arrowPreviewUrl 等) のみを保持すればよくなる。
 */
import type { Ref } from 'vue'
import type { ThemeCardData } from '~/types/theme'

export interface ThemeCardEmit {
  showDetails: (id: string) => void
  toggleFavorite: (id: string) => void
}

export function useThemeCardState(theme: Ref<ThemeCardData>, emit: ThemeCardEmit) {
  const previewMap = ref<Record<string, string> | null>(null)
  const { getMap } = useThemePreviews()

  async function fetchPreview() {
    if (!theme.value.id) return
    previewMap.value = await getMap(theme.value.id)
  }

  onMounted(fetchPreview)
  watch(() => theme.value.id, fetchPreview)

  /** Windows のシステムスキーム (HKCU\Cursors\Schemes) は編集・お気に入りを許可しない。 */
  const isSystem = computed(() => theme.value.kind === 'system')
  const isMarketplace = computed(() => theme.value.kind === 'marketplace')

  /**
   * ISO8601 / YYYY-MM-DD どちらでも先頭 10 文字に切り詰める。
   * 空文字 (Windows システムスキームでは date が空) は「—」で代替。
   */
  const displayDate = computed(() => {
    const d = theme.value.date
    if (!d) return '—'
    return d.length > 10 ? d.slice(0, 10) : d
  })

  /**
   * カード/行 全体クリック → 詳細モーダル。内側の `<button>`/`<a>`/`<input>` は
   * 個別ハンドラに委譲したいので、イベントターゲットが内側要素なら何もしない。
   */
  function onActivate(e: Event) {
    const target = e.target as HTMLElement | null
    if (target?.closest('button, a, input')) return
    emit.showDetails(theme.value.id)
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onActivate(e)
    }
  }

  /** お気に入り星のクリック。system kind では emit しない (ガード)。 */
  function onFavorite(e: Event) {
    e.stopPropagation()
    if (isSystem.value) return
    emit.toggleFavorite(theme.value.id)
  }

  return {
    previewMap,
    isSystem,
    isMarketplace,
    displayDate,
    onActivate,
    onKeydown,
    onFavorite,
  }
}
