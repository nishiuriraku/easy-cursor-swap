import { describe, it, expect } from 'vitest'
import {
  matchAssetToRole,
  matchAssetWithContext,
  normalize,
  resolveCollisions,
  scoreRole,
  type MatchCandidate,
} from '~/composables/useRoleMatcher'

describe('normalize', () => {
  it('lowercases and strips separators', () => {
    expect(normalize('Easy-Cursor_Swap.Mint__Arrow.png')).toBe('easycursorswapmintarrow')
  })

  it('strips trailing size suffixes', () => {
    expect(normalize('Arrow_64.png')).toBe('arrow')
    expect(normalize('Arrow128px.png')).toBe('arrow')
    expect(normalize('arrow_32x32.png')).toBe('arrow')
  })

  it('strips version tags', () => {
    expect(normalize('arrow_v1.0.2.png')).toBe('arrow')
  })

  it('keeps the role name itself', () => {
    expect(normalize('Arrow.png')).toBe('arrow')
  })

  it('folds full-width digits to half-width', () => {
    // `Ａｒｒｏｗ＿６４．png` → `arrow` (全角は ASCII 経路と同じ結果になる)
    expect(normalize('Ａｒｒｏｗ＿６４．png')).toBe('arrow')
    // 全角数字でも 2 桁以上ならサイズサフィックス扱いになる
    expect(normalize('arrow_６４.png')).toBe('arrow')
    // 単一桁 (1 桁) はロール識別子保護のため残る
    expect(normalize('斜め１.ani')).toBe('斜め1')
  })

  it('folds full-width ASCII letters to half-width', () => {
    // `ＰＯＩＮＴＥＲ.png` → `pointer`
    expect(normalize('ＰＯＩＮＴＥＲ.png')).toBe('pointer')
  })
})

describe('scoreRole', () => {
  it('returns 1.0 for exact normalized match', () => {
    expect(scoreRole('Arrow.png', 'Arrow')).toBe(1.0)
  })

  it('returns 0.95 for suffix match (sample-icon style)', () => {
    expect(scoreRole('easy-cursor-swap-mint__Arrow.png', 'Arrow')).toBe(0.95)
  })

  it('returns 0.90 for prefix match', () => {
    expect(scoreRole('arrow_decoration.png', 'Arrow')).toBe(0.9)
  })

  it('returns 0.80 for substring match', () => {
    expect(scoreRole('my-cursor-arrow-icon.png', 'Arrow')).toBe(0.8)
  })

  it('matches via aliases', () => {
    expect(scoreRole('pointer.png', 'Arrow')).toBe(1.0)
    expect(scoreRole('spinner.svg', 'Wait')).toBe(1.0)
  })

  it('rejects too-short levenshtein candidates', () => {
    expect(scoreRole('arr.png', 'Arrow')).toBe(0)
  })

  it('returns 0.70 for typo with levenshtein distance 1', () => {
    // 'arrov' → alias 'arrow' (1 substitution)
    expect(scoreRole('arrov.png', 'Arrow')).toBe(0.7)
  })

  it('returns 0 for unrelated names', () => {
    expect(scoreRole('totally-random.png', 'Arrow')).toBe(0)
  })
})

describe('matchAssetToRole', () => {
  it('returns best matching role', () => {
    const m = matchAssetToRole('easy-cursor-swap-mint__Arrow.png')
    expect(m).toEqual({ role: 'Arrow', score: 0.95 })
  })

  it('returns null when below threshold', () => {
    expect(matchAssetToRole('random-thing.png')).toBeNull()
  })
})

describe('resolveCollisions', () => {
  it('picks the highest resolution when scores tie', () => {
    const cands: MatchCandidate[] = [
      { sourceFile: 'arrow_64.png', nativeSize: 64, match: { role: 'Arrow', score: 0.95 } },
      { sourceFile: 'arrow_256.png', nativeSize: 256, match: { role: 'Arrow', score: 0.95 } },
    ]
    const { winners, demoted } = resolveCollisions(cands)
    expect(winners).toHaveLength(1)
    expect(winners[0].sourceFile).toBe('arrow_256.png')
    expect(demoted).toHaveLength(1)
    expect(demoted[0].sourceFile).toBe('arrow_64.png')
  })

  it('picks the higher score over higher resolution', () => {
    const cands: MatchCandidate[] = [
      { sourceFile: 'arrow.png', nativeSize: 64, match: { role: 'Arrow', score: 1.0 } },
      { sourceFile: 'arrow_decor.png', nativeSize: 256, match: { role: 'Arrow', score: 0.9 } },
    ]
    const { winners } = resolveCollisions(cands)
    expect(winners[0].sourceFile).toBe('arrow.png')
  })
})

describe('Japanese filename matching', () => {
  it('matches Japanese role names exactly via aliases', () => {
    expect(matchAssetToRole('通常.ani')).toEqual({ role: 'Arrow', score: 1.0 })
    expect(matchAssetToRole('テキスト.ani')).toEqual({ role: 'IBeam', score: 1.0 })
    expect(matchAssetToRole('待機.ani')).toEqual({ role: 'Wait', score: 1.0 })
    expect(matchAssetToRole('手書き.ani')).toEqual({ role: 'NWPen', score: 1.0 })
    expect(matchAssetToRole('移動.ani')).toEqual({ role: 'SizeAll', score: 1.0 })
    expect(matchAssetToRole('禁止.ani')).toEqual({ role: 'No', score: 1.0 })
    expect(matchAssetToRole('ヘルプ.ani')).toEqual({ role: 'Help', score: 1.0 })
  })

  it('matches the official "通常の選択" form', () => {
    expect(matchAssetToRole('通常の選択.ani')).toEqual({ role: 'Arrow', score: 1.0 })
    expect(matchAssetToRole('テキストの選択.ani')).toEqual({ role: 'IBeam', score: 1.0 })
    expect(matchAssetToRole('リンクの選択.ani')).toEqual({ role: 'Hand', score: 1.0 })
  })

  it('handles full-width spaces (U+3000) inside the filename', () => {
    // 八重神子 マウスカーソル 通常.ani — 通常 が末尾なので suffix-match → 0.95
    const m = matchAssetToRole('八重神子　マウスカーソル　通常.ani')
    expect(m?.role).toBe('Arrow')
    expect(m?.score).toBeGreaterThanOrEqual(0.95)
  })

  it('handles 斜め1 / 斜め2 with full-width digit', () => {
    expect(matchAssetToRole('斜め1.ani')?.role).toBe('SizeNWSE')
    expect(matchAssetToRole('斜め2.ani')?.role).toBe('SizeNESW')
  })

  it('does not mis-classify 手書きカーソル as IBeam', () => {
    // `カーソル` は IBeam の alias から外しているので NWPen が勝つ
    expect(matchAssetToRole('手書きカーソル.ani')?.role).toBe('NWPen')
  })

  it('returns null for irrelevant Japanese words', () => {
    expect(matchAssetToRole('ロゴ.png')).toBeNull()
  })

  it('does not let Arrow steal UpArrow files (alias-length tie-break)', () => {
    // `右上矢印` は Arrow alias `矢印` と UpArrow alias `上矢印` がどちらも suffix-match で
    // 0.95 にタイ。alias 長が長い `上矢印` を勝たせる必要がある。
    expect(matchAssetToRole('右上矢印.cur')?.role).toBe('UpArrow')
    // `代替選択` は UpArrow に exact 一致するので 1.0 → 即勝ち
    expect(matchAssetToRole('代替選択.cur')?.role).toBe('UpArrow')
    // `up_arrow_64.png` は normalize で 'uparrow' になり UpArrow alias 'uparrow' に exact 一致
    expect(matchAssetToRole('up_arrow_64.png')?.role).toBe('UpArrow')
    // `Up Arrow.cur` (空白区切り) も同様に exact 一致
    expect(matchAssetToRole('Up Arrow.cur')?.role).toBe('UpArrow')
  })
})

describe('matchAssetWithContext', () => {
  it('falls back to folder name when filename has no signal', () => {
    // ファイル名は数字のみなので無効、フォルダ名 arrow から推定する
    const m = matchAssetWithContext('64.png', 'C:/themes/arrow/64.png')
    expect(m?.role).toBe('Arrow')
    // フォルダ由来は信頼度を 0.85 倍に下げて返す
    expect(m?.score).toBeLessThan(1.0)
    expect(m?.score).toBeGreaterThanOrEqual(0.7)
  })

  it('falls back to a Japanese folder name', () => {
    const m = matchAssetWithContext('256.png', '/themes/通常/256.png')
    expect(m?.role).toBe('Arrow')
  })

  it('prefers filename match over folder match', () => {
    // ファイル名で 1.0 取れるのでフォルダ "wait" は無視される
    const m = matchAssetWithContext('arrow.png', '/themes/wait/arrow.png')
    expect(m).toEqual({ role: 'Arrow', score: 1.0 })
  })

  it('handles backslash separators (Windows paths)', () => {
    const m = matchAssetWithContext('64.png', 'C:\\themes\\テキスト\\64.png')
    expect(m?.role).toBe('IBeam')
  })

  it('returns null when neither filename nor folders match', () => {
    expect(matchAssetWithContext('64.png', '/themes/random/64.png')).toBeNull()
  })

  it('walks up multiple folder levels', () => {
    // 直上フォルダは generic, 二段上の `pin` がヒット
    const m = matchAssetWithContext('64.png', '/themes/pin/sub/64.png')
    expect(m?.role).toBe('Pin')
  })
})
