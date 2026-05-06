import { describe, it, expect } from 'vitest'
import { normalize, scoreRole } from '~/composables/useRoleMatcher'

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

  it('returns 0 for unrelated names', () => {
    expect(scoreRole('totally-random.png', 'Arrow')).toBe(0)
  })
})
