/**
 * UI グリフ (16x16 ベース)
 * design/icons.jsx の `G` ハッシュを Vue へ移植したもの。
 */

export interface UiIconDef {
  /** デフォルト viewBox。インライン上書き可。 */
  viewBox: string
  /** SVG 内側の <path>/<circle>/<rect> マークアップ。 */
  body: string
  /** デフォルトで `fill="currentColor"` で描くか (true) 線描か (false)。 */
  filled?: boolean
}

export const UI_ICONS: Record<string, UiIconDef> = {
  Search: {
    viewBox: '0 0 16 16',
    body: '<circle cx="7" cy="7" r="4.5"/><path d="m10.5 10.5 3 3"/>',
  },
  Plus: { viewBox: '0 0 16 16', body: '<path d="M8 3v10M3 8h10"/>' },
  Star: {
    viewBox: '0 0 16 16',
    filled: true,
    body: '<path d="M8 1.5l1.95 4.05 4.45.55-3.25 3.1.85 4.4L8 11.5l-4 2.1.85-4.4L1.6 6.1l4.45-.55L8 1.5z"/>',
  },
  StarO: {
    viewBox: '0 0 16 16',
    body: '<path d="M8 1.5l1.95 4.05 4.45.55-3.25 3.1.85 4.4L8 11.5l-4 2.1.85-4.4L1.6 6.1l4.45-.55L8 1.5z"/>',
  },
  Sort: { viewBox: '0 0 16 16', body: '<path d="M3 4h10M5 8h6M7 12h2"/>' },
  Grid: {
    viewBox: '0 0 16 16',
    body: '<rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="9" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="2.5" y="9" width="4.5" height="4.5" rx="1"/><rect x="9" y="9" width="4.5" height="4.5" rx="1"/>',
  },
  List: { viewBox: '0 0 16 16', body: '<path d="M3 4h10M3 8h10M3 12h10"/>' },
  Import: { viewBox: '0 0 16 16', body: '<path d="M8 2v8M5 7l3 3 3-3M2.5 13h11"/>' },
  Export: { viewBox: '0 0 16 16', body: '<path d="M8 11V3M5 6l3-3 3 3M2.5 13h11"/>' },
  Library: {
    viewBox: '0 0 16 16',
    body: '<path d="M2.5 3h2v10h-2zM5.5 3h2v10h-2zM9 3.5l2-.5 2.5 9.5-2 .5z"/>',
  },
  Brush: { viewBox: '0 0 16 16', body: '<path d="M9.5 2.5l4 4-7 7-4-1z"/><path d="M5 9l2 2"/>' },
  Globe: {
    viewBox: '0 0 16 16',
    body: '<circle cx="8" cy="8" r="6"/><path d="M2 8h12M8 2c2 2 2 10 0 12M8 2c-2 2-2 10 0 12"/>',
  },
  Settings: {
    viewBox: '0 0 16 16',
    body: '<circle cx="8" cy="8" r="2"/><path d="M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5L13 13M3 13l1.5-1.5M11.5 4.5L13 3"/>',
  },
  Sun: {
    viewBox: '0 0 16 16',
    body: '<circle cx="8" cy="8" r="3"/><path d="M8 1.5v1.5M8 13v1.5M1.5 8h1.5M13 8h1.5M3.2 3.2l1 1M11.8 11.8l1 1M3.2 12.8l1-1M11.8 4.2l1-1"/>',
  },
  Moon: {
    viewBox: '0 0 16 16',
    body: '<path d="M13 9.5A6 6 0 0 1 6.5 3a6 6 0 1 0 6.5 6.5z"/>',
  },
  Alert: {
    viewBox: '0 0 16 16',
    body: '<path d="M8 2L1.5 13.5h13z"/><path d="M8 6.5v3M8 11.5v.1"/>',
  },
  Min: { viewBox: '0 0 12 12', body: '<path d="M2.5 6h7"/>' },
  Max: { viewBox: '0 0 12 12', body: '<rect x="2.5" y="2.5" width="7" height="7"/>' },
  X: { viewBox: '0 0 12 12', body: '<path d="m3 3 6 6M9 3l-6 6"/>' },
  Logo: {
    viewBox: '0 0 24 24',
    filled: true,
    body: '<path d="M5 3 L5 18 L9 14 L11.5 20 L14 19 L11.5 13 L17 13 Z"/>',
  },
  Check: {
    viewBox: '0 0 16 16',
    body: '<path d="m3 8 3.5 3.5L13 5"/>',
  },
  ChevD: { viewBox: '0 0 16 16', body: '<path d="m4 6 4 4 4-4"/>' },
  Pkg: {
    viewBox: '0 0 24 24',
    body: '<path d="M12 3l8 4v10l-8 4-8-4V7z"/><path d="M4 7l8 4 8-4M12 11v12"/>',
  },
  Shield: {
    viewBox: '0 0 16 16',
    body: '<path d="M8 1.5l5.5 2v4.5c0 3-2.5 5.5-5.5 6.5-3-1-5.5-3.5-5.5-6.5V3.5z"/><path d="M5.5 8 7 9.5l3.5-3.5"/>',
  },
}
