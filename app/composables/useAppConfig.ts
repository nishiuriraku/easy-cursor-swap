/**
 * Rust 側の `AppConfig` を取得 / 更新する composable。
 * グローバルなリアクティブシングルトンで全画面が同じインスタンスを参照する。
 */
import { ref } from 'vue'
import type { AppConfig } from '~/types/config'
import { invokeTauri } from './useTauri'

const config = ref<AppConfig | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)

let inflight: Promise<AppConfig | null> | null = null

async function load(force = false): Promise<AppConfig | null> {
  if (config.value && !force) return config.value
  if (inflight) return inflight

  loading.value = true
  error.value = null
  inflight = (async () => {
    try {
      const result = await invokeTauri<AppConfig>('get_config')
      config.value = result ?? null
      return config.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      console.warn('[useAppConfig] get_config failed:', err)
      return null
    } finally {
      loading.value = false
      inflight = null
    }
  })()
  return inflight
}

/** `mutator` で config を変更し、Rust に永続化する。失敗時は元の値に戻す。 */
async function update(mutator: (c: AppConfig) => void): Promise<AppConfig | null> {
  const current = config.value
  if (!current) return null

  const draft: AppConfig = JSON.parse(JSON.stringify(current))
  mutator(draft)

  try {
    const updated = await invokeTauri<AppConfig>('update_config', { updates: draft })
    if (updated) config.value = updated
    return updated
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    console.error('[useAppConfig] update_config failed:', err)
    return null
  }
}

export function useAppConfig() {
  return { config, loading, error, load, update }
}
