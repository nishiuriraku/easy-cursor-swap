<script lang="ts">
/**
 * 17 種カーソル役割アイコン描画コンポーネント。
 * <CursorIcon role="Arrow" :size="14" />
 */
import { CURSOR_ICONS } from './CursorIcons'

export default defineComponent({
  name: 'CursorIcon',
  props: {
    role: { type: String, required: true },
    size: { type: [Number, String], default: 14 },
    strokeWidth: { type: Number, default: 1.5 },
  },
  setup(props) {
    return () => {
      const def = CURSOR_ICONS[props.role]
      if (!def) return null
      const size = typeof props.size === 'number' ? String(props.size) : props.size
      return h('svg', {
        viewBox: '0 0 24 24',
        width: size,
        height: size,
        fill: 'none',
        stroke: 'currentColor',
        'stroke-width': props.strokeWidth,
        'stroke-linecap': 'round',
        'stroke-linejoin': 'round',
        'aria-hidden': 'true',
        innerHTML: def.paths,
      })
    }
  },
})
</script>
