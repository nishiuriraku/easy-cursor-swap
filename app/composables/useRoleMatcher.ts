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

export const ROLE_ALIASES: Record<string, string[]> = {
  Arrow:       ['arrow', 'pointer', 'normal', 'select', 'default'],
  Help:        ['help', 'helpsel', 'question', 'whatsthis'],
  AppStarting: ['appstarting', 'starting', 'working', 'busy', 'loading'],
  Wait:        ['wait', 'busy', 'spinner', 'hourglass'],
  Crosshair:   ['crosshair', 'cross', 'precision'],
  IBeam:       ['ibeam', 'text', 'caret', 'insert'],
  NWPen:       ['nwpen', 'pen', 'handwriting', 'ink'],
  No:          ['no', 'unavailable', 'forbidden', 'block', 'denied'],
  SizeNS:      ['sizens', 'ns', 'verticalresize', 'rowresize', 'updown'],
  SizeWE:      ['sizewe', 'we', 'horizontalresize', 'colresize', 'leftright'],
  SizeNWSE:    ['sizenwse', 'nwse', 'diagonalresize1', 'diagresize'],
  SizeNESW:    ['sizenesw', 'nesw', 'diagonalresize2'],
  SizeAll:     ['sizeall', 'all', 'move', 'fleur'],
  UpArrow:     ['uparrow', 'up', 'alternateselect'],
  Hand:        ['hand', 'link', 'pointinghand', 'grab', 'finger'],
  Pin:         ['pin', 'location', 'marker'],
  Person:      ['person', 'user', 'avatar'],
}

export const CURSOR_ROLE_IDS = Object.keys(ROLE_ALIASES)

function levenshtein(a: string, b: string): number {
  if (a === b) return 0
  if (!a.length) return b.length
  if (!b.length) return a.length
  const dp = Array.from({ length: a.length + 1 }, () => new Array(b.length + 1).fill(0))
  for (let i = 0; i <= a.length; i++) dp[i][0] = i
  for (let j = 0; j <= b.length; j++) dp[0][j] = j
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1
      dp[i][j] = Math.min(dp[i - 1][j] + 1, dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost)
    }
  }
  return dp[a.length][b.length]
}

export function scoreRole(filename: string, roleId: string): number {
  const normFile = normalize(filename)
  const aliases = [roleId.toLowerCase(), ...(ROLE_ALIASES[roleId] ?? [])]

  let best = 0
  for (const alias of aliases) {
    if (normFile === alias) return 1.0
    if (normFile.endsWith(alias)) best = Math.max(best, 0.95)
    else if (normFile.startsWith(alias)) best = Math.max(best, 0.90)
    else if (normFile.includes(alias)) best = Math.max(best, 0.80)
    else if (alias.length >= 4 && levenshtein(normFile, alias) <= 1) best = Math.max(best, 0.70)
  }
  return best
}
