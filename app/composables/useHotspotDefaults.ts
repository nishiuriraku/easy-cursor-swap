/**
 * クリエイターのホットスポット既定値ロジック。
 *
 * 新規ロールに画像が初めて割り当てられる瞬間 (= ロールに既存アセットがない、かつ
 * 元ファイルにホットスポット情報がない) に呼ばれる純粋関数。
 *
 * `CENTER_HOTSPOT_ROLES` に含まれるロールは画像中央 (primarySize / 2) を
 * デフォルトとし、それ以外は従来どおり (4, 4) を返す。
 *
 * 既に編集済みのアセットには絶対に適用しない (creator.vue 側で分岐管理)。
 */
import { CENTER_HOTSPOT_ROLES } from '~/components/icons/CursorIcons'
import type { Hotspot } from './useCreatorAssets'

export function initialHotspotFor(roleId: string, primarySize: number): Hotspot {
  if (primarySize > 0 && CENTER_HOTSPOT_ROLES.has(roleId)) {
    const c = Math.round(primarySize / 2)
    return { x: c, y: c }
  }
  return { x: 4, y: 4 }
}
