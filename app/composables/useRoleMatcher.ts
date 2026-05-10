/**
 * ファイル名 → ロール ID マッチングのためのファジーマッチャ。
 * 純粋関数モジュール (Rust 非依存)。
 *
 * 対応する命名スタイル:
 *  - 英語: `arrow.png` / `easycursorswap_mint__Arrow_64.png`
 *  - 日本語: `通常.ani` / `八重神子　マウスカーソル　通常.ani`
 *  - フォルダコンテキスト: `arrow/64.png`, `通常/64.png`
 *
 * Japanese は Windows コントロールパネルの公式ロール表記
 * (例: 「通常の選択」「待ち状態」) と略称 (例: 「待機」「テキスト」) の両方を
 * エイリアスとして登録している。
 */

/**
 * ファイル名を比較しやすい形に正規化する。
 *
 *  - 全角 ASCII (`Ａ-Ｚ` `ａ-ｚ` `０-９` および `＿` `．` 等の記号) を半角に正規化
 *    → 後段の ASCII 前提処理に流す
 *  - 拡張子 (英数字のみ、ASCII の `.` 終端) を除去 → `.ani` `.cur` `.png` 等
 *  - バージョンタグ `v1` / `v1.2` 等を除去
 *  - 解像度サフィックス (`64`, `64px`, `32x32`) を除去
 *  - 区切り (`空白`, `_`, `-`, `.`, 全角空白 `　`, 中黒 `・`, 全角ハイフン類)
 *    を全て削除
 *  - toLowerCase は ASCII 大小だけが対象 (日本語には影響しない)
 */
export function normalize(name: string): string {
  return (
    name
      // 全角 ASCII (U+FF01–U+FF5E) を半角 (U+0021–U+007E) に変換。
      // ブロック全体が ASCII から +0xFEE0 のオフセットになっているので一括で fold できる。
      // 後段の `.toLowerCase()` / 拡張子除去 / サイズ剥がしは ASCII 想定のため、
      // 最初に変換しておくと全角入力 (`Ａｒｒｏｗ＿６４．png` 等) も同じ経路で処理できる
      .replace(/[！-～]/g, (ch) => String.fromCharCode(ch.charCodeAt(0) - 0xfee0))
      .toLowerCase()
      .replace(/\.[a-z0-9]+$/, '')
      .replace(/v\d+(\.\d+)*/g, '')
      .replace(/\d+x\d+/g, '')
      // サイズサフィックスは 2 桁以上のみ ( `_64`, `_128px` ) を剥がす。
      // 単一桁を残すことで、日本語ロール識別子の `斜め1` `斜め2` `対角1` 等を保護する
      .replace(/\d{2,}(?:px)?(?=[._\s\-]|$)/g, '')
      // ASCII セパレータ + 全角空白 (U+3000) + 中黒 (・) + 全角ハイフン系
      .replace(/[\s_\-.　・ーｰ‐‑–—]+/g, '')
  )
}

export const ROLE_ALIASES: Record<string, string[]> = {
  // 英語+日本語 (Windows マウスのプロパティ表記 + 略称) を併記。
  // どのエイリアスも `normalize` 後の表現で書く必要がある (空白/区切り無し)。
  Arrow: [
    'arrow',
    'pointer',
    'normal',
    'select',
    'default',
    '通常',
    '通常の選択',
    'ポインター',
    '矢印',
  ],
  Help: ['help', 'helpsel', 'question', 'whatsthis', 'ヘルプ', 'ヘルプの選択', '質問'],
  AppStarting: [
    'appstarting',
    'starting',
    'working',
    'busy',
    'loading',
    'バックグラウンドで作業中',
    'バックグラウンド',
    'バック',
    '作業',
    '作業中',
    '読み込み中',
  ],
  Wait: [
    'wait',
    'busy',
    'spinner',
    'hourglass',
    '待ち状態',
    '待ち',
    '待',
    '待機',
    '砂時計',
    'ビジー',
  ],
  Crosshair: ['crosshair', 'cross', 'precision', '領域の選択', '領域', '十字', '精密'],
  // `カーソル` は日本語カーソルファイル名に頻出する汎用語のためエイリアスから除外
  // (例: `手書きカーソル.ani` を IBeam に誤マッチさせない)
  IBeam: ['ibeam', 'text', 'caret', 'insert', 'テキストの選択', 'テキスト', 'アイビーム'],
  NWPen: ['nwpen', 'pen', 'handwriting', 'ink', '手書き', 'ペン', 'インク'],
  No: ['no', 'unavailable', 'forbidden', 'block', 'denied', '利用不可', '禁止', '使用不可'],
  SizeNS: [
    'sizens',
    'ns',
    'verticalresize',
    'rowresize',
    'updown',
    '上下に拡大縮小',
    '上下',
    '縦',
    '縦方向',
  ],
  SizeWE: [
    'sizewe',
    'we',
    'horizontalresize',
    'colresize',
    'leftright',
    '左右に拡大縮小',
    '左右',
    '横',
    '横方向',
  ],
  SizeNWSE: [
    'sizenwse',
    'nwse',
    'diagonalresize1',
    'diagresize',
    '斜めに拡大縮小1',
    '斜め1',
    '対角1',
    '左上右下',
  ],
  SizeNESW: [
    'sizenesw',
    'nesw',
    'diagonalresize2',
    '斜めに拡大縮小2',
    '斜め2',
    '対角2',
    '右上左下',
  ],
  SizeAll: ['sizeall', 'all', 'move', 'fleur', '移動', '全方向', '全方向に拡大縮小'],
  UpArrow: ['uparrow', 'up', 'alternateselect', '代替選択', '代替', '上矢印'],
  Hand: ['hand', 'link', 'pointinghand', 'grab', 'finger', 'リンクの選択', 'リンク', '手', '指'],
  Pin: ['pin', 'location', 'marker', '場所の選択', '場所', 'ピン', 'マーカー'],
  Person: ['person', 'user', 'avatar', '人の選択', '人', 'ユーザー', 'アバター'],
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

/**
 * 内部用: スコアと「ヒットした alias の文字数」を一緒に返す。
 *
 * 文字数情報は `matchAssetToRole` の同点 tie-break に使う。
 * 例えば `右上矢印` は Arrow alias `矢印` (len=2) と UpArrow alias `上矢印` (len=3) の
 * どちらも suffix-match して 0.95 でタイするので、より長い alias を持つ UpArrow を勝たせる。
 * これにより 17 ロール辞書順イテレーションの偶然で Arrow が勝ってしまうバグを防ぐ。
 */
function scoreRoleDetailed(
  filename: string,
  roleId: string,
): { score: number; matchedLen: number } {
  const normFile = normalize(filename)
  const aliases = [roleId.toLowerCase(), ...(ROLE_ALIASES[roleId] ?? [])]

  let best = 0
  let bestLen = 0
  for (const alias of aliases) {
    let s = 0
    if (normFile === alias) s = 1.0
    else if (normFile.endsWith(alias)) s = 0.95
    else if (normFile.startsWith(alias)) s = 0.9
    else if (normFile.includes(alias)) s = 0.8
    else if (alias.length >= 4 && levenshtein(normFile, alias) <= 1) s = 0.7
    if (s > best || (s === best && alias.length > bestLen)) {
      best = s
      bestLen = alias.length
    }
  }
  return { score: best, matchedLen: bestLen }
}

export function scoreRole(filename: string, roleId: string): number {
  return scoreRoleDetailed(filename, roleId).score
}

export interface RoleMatch {
  role: string
  score: number
}

export interface MatchCandidate {
  sourceFile: string
  nativeSize: number
  match: RoleMatch
  // 任意の追加情報をモーダル側で attach できるように緩く
  [key: string]: unknown
}

const MATCH_THRESHOLD = 0.7

export function matchAssetToRole(filename: string): RoleMatch | null {
  let best: { role: string; score: number; matchedLen: number } | null = null
  for (const roleId of CURSOR_ROLE_IDS) {
    const { score, matchedLen } = scoreRoleDetailed(filename, roleId)
    if (score <= 0) continue
    // 同点時はより長い alias を優先 (例: `上矢印` (UpArrow, len=3) > `矢印` (Arrow, len=2))。
    // これによりロール辞書順 (Arrow が先) の偶然で短い alias が勝ってしまうのを防ぐ。
    const better =
      !best || score > best.score || (score === best.score && matchedLen > best.matchedLen)
    if (better) best = { role: roleId, score, matchedLen }
  }
  return best && best.score >= MATCH_THRESHOLD ? { role: best.role, score: best.score } : null
}

/**
 * ファイル名 + パスからロールを推定する。
 *
 * まずファイル名 (basename) でマッチを試み、ヒットしなければフォルダー名を
 * 近い側から順に試すフォールバック付き。`arrow/64.png` や `通常/256.png` の
 * ような「ロール名フォルダー + サイズ番号ファイル」パターンに対応する。
 *
 * フォルダー由来のヒットは Filename ほど信頼できないので score を 0.85 倍に
 * 落として返す (下限 0.7 = `MATCH_THRESHOLD`)。
 */
export function matchAssetWithContext(filename: string, sourcePath?: string): RoleMatch | null {
  const direct = matchAssetToRole(filename)
  if (direct) return direct
  if (!sourcePath) return null

  // Win/Posix 両対応で区切りを揃え、末尾 (= filename と同じ basename) は除外
  const parts = sourcePath
    .replace(/\\/g, '/')
    .split('/')
    .filter((s) => s.length > 0)
  for (let i = parts.length - 2; i >= 0; i--) {
    const folder = parts[i]
    if (!folder) continue
    const m = matchAssetToRole(folder)
    if (m) {
      const adjusted = Math.max(MATCH_THRESHOLD, m.score * 0.85)
      return { role: m.role, score: adjusted }
    }
  }
  return null
}

export function resolveCollisions(candidates: MatchCandidate[]): {
  winners: MatchCandidate[]
  demoted: MatchCandidate[]
} {
  const byRole = new Map<string, MatchCandidate[]>()
  for (const c of candidates) {
    const list = byRole.get(c.match.role) ?? []
    list.push(c)
    byRole.set(c.match.role, list)
  }

  const winners: MatchCandidate[] = []
  const demoted: MatchCandidate[] = []
  for (const [, group] of byRole) {
    group.sort((a, b) => b.match.score - a.match.score || b.nativeSize - a.nativeSize)
    winners.push(group[0])
    demoted.push(...group.slice(1))
  }
  return { winners, demoted }
}
