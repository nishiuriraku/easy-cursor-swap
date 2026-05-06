import { describe, it, expect } from 'vitest'
import { normalize } from '~/composables/useRoleMatcher'

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
