import { describe, it, expect } from 'vitest'
import { matchAssetToRole, normalize, resolveCollisions, scoreRole, type MatchCandidate } from '~/composables/useRoleMatcher'

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
})

describe('scoreRole', () => {
  it('returns 1.0 for exact normalized match', () => {
    expect(scoreRole('Arrow.png', 'Arrow')).toBe(1.0)
  })

  it('returns 0.95 for suffix match (sample-icon style)', () => {
    expect(scoreRole('easy-cursor-swap-mint__Arrow.png', 'Arrow')).toBe(0.95)
  })

  it('returns 0.90 for prefix match', () => {
    expect(scoreRole('arrow_decoration.png', 'Arrow')).toBe(0.90)
  })

  it('returns 0.80 for substring match', () => {
    expect(scoreRole('my-cursor-arrow-icon.png', 'Arrow')).toBe(0.80)
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
    expect(scoreRole('arrov.png', 'Arrow')).toBe(0.70)
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
      { sourceFile: 'arrow_64.png',  nativeSize: 64,  match: { role: 'Arrow', score: 0.95 } },
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
      { sourceFile: 'arrow.png',       nativeSize: 64,  match: { role: 'Arrow', score: 1.0 } },
      { sourceFile: 'arrow_decor.png', nativeSize: 256, match: { role: 'Arrow', score: 0.90 } },
    ]
    const { winners } = resolveCollisions(cands)
    expect(winners[0].sourceFile).toBe('arrow.png')
  })
})
