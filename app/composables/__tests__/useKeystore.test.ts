/**
 * useKeystore composable の回帰テスト (Wave 2AB / Task 13)。
 *
 * 検証対象:
 *   1. generate() 成功時に info に新鍵ペアが反映され lastError がリセットされる
 *   2. remove() 成功時に info が空状態にリセットされる
 *   3. remove() 失敗時に lastError がセットされ info は前回値のまま
 *   4. remove() 失敗 → 再試行 → 成功 の遷移で lastError がクリアされる
 *   5. refresh() 失敗時に lastError がセットされ、戻り値は変更前の info
 *   6. busy フラグが処理中の true → 完了後 false に戻ること
 *
 * IPC はすべてモック化し、実レジストリ / DPAPI には触らない。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>>()

vi.mock('~/composables/useTauri', () => ({
  invokeTauri: (cmd: string, args?: Record<string, unknown>) => invokeMock(cmd, args),
}))

import { useKeystore, __resetForTests } from '../useKeystore'

const emptyInfo = {
  has_keypair: false,
  key_id: null,
  public_key_b64: null,
}

const sampleInfo = {
  has_keypair: true,
  key_id: 'abcd1234efgh5678',
  public_key_b64: 'MCowBQYDK2VwAyEAR9ExamplePublicKeyBase64==',
}

describe('useKeystore', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    // テスト間でモジュール singleton の状態が残らないよう必ず reset する
    __resetForTests()
  })

  afterEach(() => {
    // 念のため二重にリセット
    __resetForTests()
  })

  // ── generate() ────────────────────────────────────────────────
  it('generate() success updates info and clears lastError', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    const result = await ks.generate(false)
    expect(invokeMock).toHaveBeenCalledWith('keystore_generate', { force: false })
    expect(result).toEqual(sampleInfo)
    expect(ks.info.value).toEqual(sampleInfo)
    expect(ks.lastError.value).toBeNull()
    expect(ks.busy.value).toBe(false)
  })

  it('generate(force=true) forwards force flag to IPC', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    await ks.generate(true)
    expect(invokeMock).toHaveBeenLastCalledWith('keystore_generate', { force: true })
  })

  it('generate() failure sets lastError and returns null', async () => {
    invokeMock.mockRejectedValueOnce(new Error('crypto: DPAPI 失敗'))
    const ks = useKeystore()
    const result = await ks.generate(false)
    expect(result).toBeNull()
    expect(ks.lastError.value).toBe('crypto: DPAPI 失敗')
    expect(ks.busy.value).toBe(false)
  })

  // ── remove() ─────────────────────────────────────────────────
  it('remove() success resets info to empty and clears lastError', async () => {
    // info に既存値を入れるため refresh を先にモック
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    await ks.refresh()
    expect(ks.info.value.has_keypair).toBe(true)

    // remove() の戻り値モック
    invokeMock.mockResolvedValueOnce(undefined)
    const ok = await ks.remove()
    expect(invokeMock).toHaveBeenLastCalledWith('keystore_delete', undefined)
    expect(ok).toBe(true)
    expect(ks.info.value).toEqual(emptyInfo)
    expect(ks.lastError.value).toBeNull()
  })

  it('remove() failure sets lastError and keeps info unchanged', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo) // refresh で鍵ありにする
    const ks = useKeystore()
    await ks.refresh()
    expect(ks.info.value.has_keypair).toBe(true)

    invokeMock.mockRejectedValueOnce(new Error('crypto: delete failed'))
    const ok = await ks.remove()
    expect(ok).toBe(false)
    expect(ks.lastError.value).toBe('crypto: delete failed')
    // 失敗時は前回値 (鍵あり) のまま
    expect(ks.info.value.has_keypair).toBe(true)
  })

  it('remove() failure → success sequence clears lastError on retry', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    await ks.refresh()

    // 1回目失敗
    invokeMock.mockRejectedValueOnce(new Error('first failed'))
    await ks.remove()
    expect(ks.lastError.value).toBe('first failed')

    // 2回目成功 → lastError クリア
    invokeMock.mockResolvedValueOnce(undefined)
    const ok = await ks.remove()
    expect(ok).toBe(true)
    expect(ks.lastError.value).toBeNull()
    expect(ks.info.value.has_keypair).toBe(false)
  })

  // ── refresh() ────────────────────────────────────────────────
  it('refresh() success overwrites info and returns it', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    const result = await ks.refresh()
    expect(invokeMock).toHaveBeenCalledWith('keystore_info', undefined)
    expect(result).toEqual(sampleInfo)
    expect(ks.info.value).toEqual(sampleInfo)
    expect(ks.lastError.value).toBeNull()
  })

  it('refresh() failure sets lastError and returns last known info', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    await ks.refresh()
    expect(ks.lastError.value).toBeNull() // 成功時はクリア

    invokeMock.mockRejectedValueOnce(new Error('boom'))
    const result = await ks.refresh()
    expect(ks.lastError.value).toBe('boom')
    // 失敗しても前回値 (sampleInfo) は維持
    expect(result).toEqual(sampleInfo)
  })

  // ── busy フラグ ──────────────────────────────────────────────
  it('busy returns to false after a successful operation', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    expect(ks.busy.value).toBe(false)
    const promise = ks.generate(false)
    // 同期チェック: 呼出直後は busy = true
    expect(ks.busy.value).toBe(true)
    await promise
    expect(ks.busy.value).toBe(false)
  })

  it('busy returns to false after a failed operation (no leak on rejection)', async () => {
    invokeMock.mockRejectedValueOnce(new Error('nope'))
    const ks = useKeystore()
    expect(ks.busy.value).toBe(false)
    await ks.generate(false)
    // finally で false に戻ること
    expect(ks.busy.value).toBe(false)
    expect(ks.lastError.value).toBe('nope')
  })

  // ── importPrivate() ───────────────────────────────────────────
  // 元は関数存在チェックのみだったが、importPrivate の本体 decode 経路 (passphrase と
  // inputPath を IPC に正しく転送し、成功時は info を更新し、失敗時は lastError を
  // セットする) を contract レベルで固定する。
  it('importPrivate() success forwards passphrase + inputPath and updates info', async () => {
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const ks = useKeystore()
    const result = await ks.importPrivate('correct horse battery staple', 'C:/keys/export.cfkey')
    expect(invokeMock).toHaveBeenLastCalledWith('keystore_import', {
      passphrase: 'correct horse battery staple',
      inputPath: 'C:/keys/export.cfkey',
    })
    expect(result).toEqual(sampleInfo)
    expect(ks.info.value).toEqual(sampleInfo)
    expect(ks.lastError.value).toBeNull()
    expect(ks.busy.value).toBe(false)
  })

  it('importPrivate() success even when returned info is null (keystore is empty after wipe)', async () => {
    // Rust 側で「復号成功 → 鍵が既に破棄済み」ケースで null が返ることを想定。
    // その場合も info は empty 形にリセットされ、戻り値は null (= 例外ではない)。
    invokeMock.mockResolvedValueOnce(null)
    const ks = useKeystore()
    const result = await ks.importPrivate('pp', 'in.cfkey')
    expect(result).toBeNull()
    expect(ks.info.value).toEqual(emptyInfo)
    expect(ks.lastError.value).toBeNull()
  })

  it('importPrivate() failure (wrong passphrase / corrupt blob) sets lastError', async () => {
    invokeMock.mockRejectedValueOnce(new Error('crypto: passphrase mismatch'))
    const ks = useKeystore()
    const result = await ks.importPrivate('wrong', 'in.cfkey')
    expect(result).toBeNull()
    expect(ks.lastError.value).toBe('crypto: passphrase mismatch')
    expect(ks.busy.value).toBe(false)
  })

  it('importPrivate() failure → success sequence clears lastError on retry', async () => {
    // 1回目: パスフレーズ間違いで失敗
    invokeMock.mockRejectedValueOnce(new Error('crypto: passphrase mismatch'))
    const ks = useKeystore()
    const r1 = await ks.importPrivate('bad', 'in.cfkey')
    expect(r1).toBeNull()
    expect(ks.lastError.value).toBe('crypto: passphrase mismatch')

    // 2回目: パスフレーズ修正で成功 → lastError クリア & info 更新
    invokeMock.mockResolvedValueOnce(sampleInfo)
    const r2 = await ks.importPrivate('good', 'in.cfkey')
    expect(r2).toEqual(sampleInfo)
    expect(ks.lastError.value).toBeNull()
    expect(ks.info.value).toEqual(sampleInfo)
  })

  it('importPrivate() during busy=true (busy flag is set synchronously)', async () => {
    let resolveInvoke: (v: unknown) => void = () => {}
    invokeMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveInvoke = resolve
      }),
    )
    const ks = useKeystore()
    const promise = ks.importPrivate('pp', 'in.cfkey')
    // 同期チェック: 呼出直後は busy = true
    expect(ks.busy.value).toBe(true)
    resolveInvoke(sampleInfo)
    await promise
    expect(ks.busy.value).toBe(false)
  })

  // ── return shape ─────────────────────────────────────────────
  it('useKeystore() exposes info / busy / lastError / refresh / generate / remove / exportPrivate / importPrivate', () => {
    const ks = useKeystore()
    expect(ks.info.value).not.toBeNull()
    expect(typeof ks.busy.value).toBe('boolean')
    // lastError は beforeEach で null リセット済み
    expect(ks.lastError.value).toBeNull()
    expect(typeof ks.refresh).toBe('function')
    expect(typeof ks.generate).toBe('function')
    expect(typeof ks.remove).toBe('function')
    expect(typeof ks.exportPrivate).toBe('function')
    expect(typeof ks.importPrivate).toBe('function')
  })
})
