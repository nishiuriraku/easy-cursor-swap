import { CENTER_HOTSPOT_ROLES } from '~/components/icons/CursorIcons'
import type { Hotspot } from './useCreatorAssets'

/**
 * 新規ロールに画像が初めて割り当てられる瞬間のホットスポット既定値。
 *
 * - `CENTER_HOTSPOT_ROLES` に含まれるロールは画像中央 (ratio 0.5, 0.5)。
 * - それ以外は左上付近 (4px @ primarySize の比率)。primarySize=0 のときは 0.0。
 *
 * 既に編集済みのアセットには絶対に適用しない (creator.vue 側で分岐管理)。
 */
export function initialHotspotFor(roleId: string, primarySize: number): Hotspot {
  if (CENTER_HOTSPOT_ROLES.has(roleId)) {
    return { x: 0.5, y: 0.5 }
  }
  const r = primarySize > 0 ? 4 / primarySize : 0
  return { x: r, y: r }
}
