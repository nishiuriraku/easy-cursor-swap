/**
 * 17 種のカーソル役割アイコン (24x24 ストロークセット)
 * SVG パスデータをエクスポート。`<UiIcon>` コンポーネントで描画する。
 */

export interface CursorIconDef {
  paths: string
  // カスタム属性 (fill 指定など)。空ならデフォルトの stroke 描画。
  raw?: string
}

export const CURSOR_ICONS: Record<string, CursorIconDef> = {
  Arrow: {
    paths:
      '<path d="M5 3 L5 18 L9 14 L11.5 20 L14 19 L11.5 13 L17 13 Z" fill="currentColor" fill-opacity="0.18"/>',
  },
  Help: {
    paths: `
      <path d="M5 3 L5 16 L8 13 L10 18 L12 17 L10 12 L14 12 Z"/>
      <circle cx="18" cy="7" r="3.5"/>
      <path d="M16.8 6.2c0-0.7 0.6-1.2 1.2-1.2 0.7 0 1.2 0.5 1.2 1.2 0 0.7-1.2 0.8-1.2 1.6"/>
      <path d="M18 9.2v0.1"/>
    `,
  },
  AppStarting: {
    paths: `
      <path d="M5 3 L5 16 L8 13 L10 18 L12 17 L10 12 L14 12 Z"/>
      <circle cx="17" cy="9" r="4"/>
      <path d="M17 6.5 A2.5 2.5 0 0 1 19.5 9" stroke="currentColor" stroke-opacity="0.5"/>
    `,
  },
  Wait: {
    paths: `
      <circle cx="12" cy="12" r="7"/>
      <path d="M9 8 H15 L12 12 L15 16 H9 L12 12 Z" fill="currentColor" fill-opacity="0.2"/>
    `,
  },
  Crosshair: {
    paths: '<path d="M12 4 V20 M4 12 H20"/><circle cx="12" cy="12" r="1"/>',
  },
  IBeam: {
    paths: '<path d="M9 4 H15 M9 20 H15 M12 4 V20"/>',
  },
  NWPen: {
    paths: '<path d="M4 20 L8 19 L19 8 L16 5 L5 16 Z"/><path d="M14 7 L17 10"/>',
  },
  No: {
    paths: '<circle cx="12" cy="12" r="8"/><path d="M6.5 6.5 L17.5 17.5"/>',
  },
  SizeNS: {
    paths: '<path d="M12 4 V20 M8 8 L12 4 L16 8 M8 16 L12 20 L16 16"/>',
  },
  SizeWE: {
    paths: '<path d="M4 12 H20 M8 8 L4 12 L8 16 M16 8 L20 12 L16 16"/>',
  },
  SizeNWSE: {
    paths: '<path d="M5 5 L19 19 M5 9 V5 H9 M19 15 V19 H15"/>',
  },
  SizeNESW: {
    paths: '<path d="M19 5 L5 19 M15 5 H19 V9 M5 15 V19 H9"/>',
  },
  SizeAll: {
    paths:
      '<path d="M12 4 V20 M4 12 H20 M9 7 L12 4 L15 7 M9 17 L12 20 L15 17 M7 9 L4 12 L7 15 M17 9 L20 12 L17 15"/>',
  },
  UpArrow: {
    paths: '<path d="M12 4 V20 M7 9 L12 4 L17 9"/>',
  },
  Hand: {
    paths:
      '<path d="M9 12 V6 a1.5 1.5 0 0 1 3 0 V12 M12 11 V5 a1.5 1.5 0 0 1 3 0 V12 M15 11 V6 a1.5 1.5 0 0 1 3 0 V14 a6 6 0 0 1 -6 6 H10 a3 3 0 0 1 -2.5 -1.5 L5 14 a1.5 1.5 0 0 1 2.5 -1.5 L9 14"/>',
  },
  Pin: {
    paths:
      '<path d="M12 21 C12 21 18 14.5 18 10 a6 6 0 0 0 -12 0 c0 4.5 6 11 6 11 Z"/><circle cx="12" cy="10" r="2.2"/>',
  },
  Person: {
    paths: '<circle cx="12" cy="8" r="3.5"/><path d="M5 20 a7 7 0 0 1 14 0"/>',
  },
}

/** 17 役割の順序付き定義。Source of Truth として全画面で使用。 */
export interface CursorRoleDef {
  id: string
  jp: string
  en: string
}

export const CURSOR_ROLES: CursorRoleDef[] = [
  { id: 'Arrow', jp: '通常の選択', en: 'Normal Select' },
  { id: 'Help', jp: 'ヘルプの選択', en: 'Help Select' },
  { id: 'AppStarting', jp: 'バックグラウンド作業', en: 'Working in Bg' },
  { id: 'Wait', jp: '待ち状態', en: 'Busy' },
  { id: 'Crosshair', jp: '領域の選択', en: 'Precision Select' },
  { id: 'IBeam', jp: 'テキストの選択', en: 'Text Select' },
  { id: 'NWPen', jp: '手書き', en: 'Handwriting' },
  { id: 'No', jp: '利用不可', en: 'Unavailable' },
  { id: 'SizeNS', jp: '上下に拡大/縮小', en: 'Vertical Resize' },
  { id: 'SizeWE', jp: '左右に拡大/縮小', en: 'Horizontal Resize' },
  { id: 'SizeNWSE', jp: '斜めに拡大/縮小 1', en: 'Diagonal Resize 1' },
  { id: 'SizeNESW', jp: '斜めに拡大/縮小 2', en: 'Diagonal Resize 2' },
  { id: 'SizeAll', jp: '移動', en: 'Move' },
  { id: 'UpArrow', jp: '代替選択', en: 'Alternate Select' },
  { id: 'Hand', jp: 'リンクの選択', en: 'Link Select' },
  { id: 'Pin', jp: '場所の選択', en: 'Location Select' },
  { id: 'Person', jp: '人の選択', en: 'Person Select' },
]
