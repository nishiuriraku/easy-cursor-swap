import { describe, expect, it, vi } from 'vitest'
import { usePngBlobCache } from '../usePngBlobCache'

describe('usePngBlobCache', () => {
  it('caches the fetched value and skips fetcher on subsequent get', async () => {
    const fetcher = vi.fn(async (key: string) => `value:${key}`)
    const cache = usePngBlobCache<string, string>({ fetcher })

    expect(await cache.get('a')).toBe('value:a')
    expect(await cache.get('a')).toBe('value:a')
    expect(fetcher).toHaveBeenCalledTimes(1)
    expect(cache.size()).toBe(1)
  })

  it('shares the inflight Promise for concurrent get on the same key', async () => {
    let resolve: ((v: string | null) => void) | null = null
    const fetcher = vi.fn(
      () =>
        new Promise<string | null>((r) => {
          resolve = r
        }),
    )
    const cache = usePngBlobCache<string, string>({ fetcher })

    const a = cache.get('k')
    const b = cache.get('k')
    expect(fetcher).toHaveBeenCalledTimes(1)
    resolve!('value')
    expect(await a).toBe('value')
    expect(await b).toBe('value')
  })

  it('does not cache null returns but releases inflight (retryable)', async () => {
    let n = 0
    const fetcher = vi.fn(async () => {
      n += 1
      return n === 1 ? null : 'v'
    })
    const cache = usePngBlobCache<string, string>({ fetcher })

    expect(await cache.get('k')).toBeNull()
    expect(cache.size()).toBe(0)
    expect(await cache.get('k')).toBe('v')
    expect(fetcher).toHaveBeenCalledTimes(2)
  })

  it('invalidate removes entry and calls dispose hook', async () => {
    const dispose = vi.fn()
    const cache = usePngBlobCache<string, string>({
      fetcher: async (k) => `value:${k}`,
      dispose,
    })

    await cache.get('a')
    expect(cache.size()).toBe(1)
    cache.invalidate('a')
    expect(cache.size()).toBe(0)
    expect(dispose).toHaveBeenCalledWith('value:a')
  })

  it('invalidateAll disposes every cached value', async () => {
    const dispose = vi.fn()
    const cache = usePngBlobCache<string, string>({
      fetcher: async (k) => `value:${k}`,
      dispose,
    })

    await Promise.all([cache.get('a'), cache.get('b'), cache.get('c')])
    expect(cache.size()).toBe(3)
    cache.invalidateAll()
    expect(cache.size()).toBe(0)
    expect(dispose).toHaveBeenCalledTimes(3)
  })

  it('supports compound keys via keyOf', async () => {
    const fetcher = vi.fn(async (k: { ns: string; id: number }) => `v:${k.ns}:${k.id}`)
    const cache = usePngBlobCache<{ ns: string; id: number }, string>({
      fetcher,
      keyOf: (k) => `${k.ns}#${k.id}`,
    })

    await cache.get({ ns: 'a', id: 1 })
    await cache.get({ ns: 'a', id: 1 })
    await cache.get({ ns: 'a', id: 2 })
    expect(fetcher).toHaveBeenCalledTimes(2)
    expect(cache.size()).toBe(2)
  })
})
