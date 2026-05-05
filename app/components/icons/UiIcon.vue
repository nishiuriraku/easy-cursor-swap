<script lang="ts">
/**
 * UI グリフ描画コンポーネント。
 * <UiIcon name="Search" :size="14" />
 *
 * NOTE: v-html を避けるため innerHTML プロパティで SVG body を注入する。
 * 注入元は内部静的データ (UI_ICONS) のみで、ユーザー入力は含まない。
 */
import { defineComponent, h } from 'vue'
import { UI_ICONS } from './UiIcons'

export default defineComponent({
  name: 'UiIcon',
  props: {
    name: { type: String, required: true },
    size: { type: [Number, String], default: 16 },
    strokeWidth: { type: Number, default: 1.5 },
  },
  setup(props) {
    return () => {
      const def = UI_ICONS[props.name]
      if (!def) return null
      const size = typeof props.size === 'number' ? String(props.size) : props.size
      return h('svg', {
        viewBox: def.viewBox,
        width: size,
        height: size,
        fill: def.filled ? 'currentColor' : 'none',
        stroke: def.filled ? 'none' : 'currentColor',
        'stroke-width': def.filled ? 0 : props.strokeWidth,
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'aria-hidden': 'true',
        innerHTML: def.body,
      })
    }
  },
})
</script>
