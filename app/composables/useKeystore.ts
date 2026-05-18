/**
 * Ed25519 鍵ペアのリアクティブ状態管理 composable。
 * Rust 側 `keystore_*` IPC をラップし、UI から鍵生成 / 削除 / 状態取得を行う。
 */

export interface KeystoreInfo {
  has_keypair: boolean
  key_id: string | null
  public_key_b64: string | null
}

const info = ref<KeystoreInfo>({
  has_keypair: false,
  key_id: null,
  public_key_b64: null,
})
const busy = ref(false)
const lastError = ref<string | null>(null)

async function refresh(): Promise<KeystoreInfo> {
  busy.value = true
  lastError.value = null
  try {
    const result = await invokeTauri<KeystoreInfo>('keystore_info')
    if (result) info.value = result
    return info.value
  } catch (err) {
    lastError.value = err instanceof Error ? err.message : String(err)
    return info.value
  } finally {
    busy.value = false
  }
}

async function generate(force = false): Promise<KeystoreInfo | null> {
  busy.value = true
  lastError.value = null
  try {
    const result = await invokeTauri<KeystoreInfo>('keystore_generate', { force })
    if (result) info.value = result
    return info.value
  } catch (err) {
    lastError.value = err instanceof Error ? err.message : String(err)
    return null
  } finally {
    busy.value = false
  }
}

async function exportPrivate(passphrase: string, outputPath: string): Promise<number | null> {
  busy.value = true
  lastError.value = null
  try {
    return await invokeTauri<number>('keystore_export', { passphrase, outputPath })
  } catch (err) {
    lastError.value = err instanceof Error ? err.message : String(err)
    return null
  } finally {
    busy.value = false
  }
}

async function importPrivate(passphrase: string, inputPath: string): Promise<KeystoreInfo | null> {
  busy.value = true
  lastError.value = null
  try {
    const result = await invokeTauri<KeystoreInfo>('keystore_import', { passphrase, inputPath })
    if (result) info.value = result
    return result
  } catch (err) {
    lastError.value = err instanceof Error ? err.message : String(err)
    return null
  } finally {
    busy.value = false
  }
}

async function remove(): Promise<boolean> {
  busy.value = true
  lastError.value = null
  try {
    await invokeTauri<void>('keystore_delete')
    info.value = { has_keypair: false, key_id: null, public_key_b64: null }
    return true
  } catch (err) {
    lastError.value = err instanceof Error ? err.message : String(err)
    return false
  } finally {
    busy.value = false
  }
}

export function useKeystore() {
  return { info, busy, lastError, refresh, generate, remove, exportPrivate, importPrivate }
}
