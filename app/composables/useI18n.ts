/**
 * 軽量 i18n composable。
 *
 * - vue-i18n を使わずに最小依存で多言語切替を提供。
 * - グローバルなリアクティブシングルトンとして実装し、全画面で同じ locale を参照。
 * - 起動時は (1) `useAppSettings` の `general.language` (`auto`/`ja`/`en`) →
 *   (2) ブラウザ/Tauri OS ロケール (`navigator.language`) でフォールバック判定。
 * - `t('a.b.c', { name: 'foo' })` でキー解決 + `{var}` プレースホルダ展開。
 */
import ja from '~/locales/ja'
import en from '~/locales/en'

export type Locale = 'ja' | 'en'

type LocaleResource = typeof ja

// ja.ts を Source of Truth にして型を派生 (キー欠落は CI で検出予定)
const RESOURCES: Record<Locale, LocaleResource> = {
  ja,
  en: en as LocaleResource,
}

const locale = ref<Locale>('ja')
let initialized = false

function detectFromBrowser(): Locale {
  if (typeof navigator === 'undefined') return 'ja'
  return navigator.language.toLowerCase().startsWith('ja') ? 'ja' : 'en'
}

/** `'a.b.c'` を nested object から取得 */
function resolveKey(obj: unknown, path: string): string | undefined {
  const parts = path.split('.')
  let cursor: unknown = obj
  for (const p of parts) {
    if (typeof cursor !== 'object' || cursor === null) return undefined
    cursor = (cursor as Record<string, unknown>)[p]
  }
  return typeof cursor === 'string' ? cursor : undefined
}

/** `{name}` を params で展開 */
function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (_, key: string) => {
    const v = params[key]
    return v === undefined ? `{${key}}` : String(v)
  })
}

const messages = computed<LocaleResource>(() => RESOURCES[locale.value])

function t(key: string, params?: Record<string, string | number>): string {
  const resolved = resolveKey(messages.value, key)
  if (resolved === undefined) {
    // フォールバック: ja → key そのもの
    if (locale.value !== 'ja') {
      const ja = resolveKey(RESOURCES.ja, key)
      if (ja !== undefined) return interpolate(ja, params)
    }
    return key
  }
  return interpolate(resolved, params)
}

function setLocale(next: Locale) {
  locale.value = next
}

/**
 * config 由来のロケール設定 (`'auto' | 'ja' | 'en'`) を解決して適用する。
 * `auto` ならブラウザロケール、不明値も auto と同じ扱い。
 */
function syncFromConfig(configLanguage: string | undefined | null) {
  const v = configLanguage ?? 'auto'
  if (v === 'ja' || v === 'en') {
    setLocale(v)
  } else {
    setLocale(detectFromBrowser())
  }
}

export function useI18n() {
  if (!initialized) {
    locale.value = detectFromBrowser()
    initialized = true
  }
  return { locale, t, setLocale, syncFromConfig }
}
