/**
 * 設定検索機能の純関数 + リアクティブステート。
 *
 * - CATALOG は全 SettingsRow を宣言的に列挙したカタログ (Source of Truth)。
 *   新しい SettingsRow を追加した際は必ずここにも追記する。テストでドリフトを検出。
 * - ja/en 両 locale の labelKey/descKey を小文字化 substring で検索。
 * - jumpTo は section ref を切替えてから DOM をスクロール + パルスハイライト。
 */
import { computed, nextTick, ref, type Ref } from 'vue'
import ja from '~/locales/ja'
import en from '~/locales/en'
import type { Locale } from '~/composables/useI18n'

export type SettingsSectionId =
  | 'general'
  | 'startup'
  | 'library'
  | 'security'
  | 'keys'
  | 'logging'
  | 'updates'
  | 'about'

export interface SearchContext {
  /** Keys セクションの SettingsRow 分岐 (v-if) を反映するフラグ */
  hasKeystore: boolean
}

export interface SettingsSearchEntry {
  section: SettingsSectionId
  /** セクション内一意。data-search-anchor 値として使用 */
  anchor: string
  labelKey: string
  descKey?: string
  /** state に応じて表示されない行を検索結果から除外するガード */
  visible?: (ctx: SearchContext) => boolean
}

export interface SearchResult {
  entry: SettingsSearchEntry
  displayLabel: string
  displaySectionLabel: string
}

/** セクション ID → セクション名 i18n キー */
const SECTION_LABEL_KEYS: Record<SettingsSectionId, string> = {
  general: 'settings.sectionGeneral',
  startup: 'settings.sectionStartup',
  library: 'settings.sectionLibrary',
  security: 'settings.sectionSecurity',
  keys: 'settings.sectionKeys',
  logging: 'settings.sectionLogging',
  updates: 'settings.sectionUpdates',
  about: 'settings.sectionAbout',
}

/** ドット区切りキーで messages を引く。存在しなければ undefined。 */
export function lookupMessage(messages: unknown, dottedKey: string): string | undefined {
  const parts = dottedKey.split('.')
  let cursor: unknown = messages
  for (const p of parts) {
    if (typeof cursor !== 'object' || cursor === null) return undefined
    cursor = (cursor as Record<string, unknown>)[p]
  }
  return typeof cursor === 'string' ? cursor : undefined
}

function normalize(s: string): string {
  return s.toLowerCase()
}

// CATALOG: 全 SettingsRow を宣言的に列挙
// IMPORTANT: 各 Section SFC で <SettingsRow> を増減した際は必ずここを同期する。
//            CATALOG integrity テストでドリフト検知される。
export const CATALOG: SettingsSearchEntry[] = [
  // ---- general ----
  {
    section: 'general',
    anchor: 'language',
    labelKey: 'settings.languageLabel',
    descKey: 'settings.languageDesc',
  },
  {
    section: 'general',
    anchor: 'showApplyToast',
    labelKey: 'settings.showApplyToastLabel',
    descKey: 'settings.showApplyToastDesc',
  },
  {
    section: 'general',
    anchor: 'applyShadowControl',
    labelKey: 'settings.applyShadowControlLabel',
    descKey: 'settings.applyShadowControlDesc',
  },

  // ---- startup ----
  {
    section: 'startup',
    anchor: 'autoStart',
    labelKey: 'settings.autoStartLabel',
    descKey: 'settings.autoStartDesc',
  },
  {
    section: 'startup',
    anchor: 'startMinimized',
    labelKey: 'settings.startMinimizedLabel',
    descKey: 'settings.startMinimizedDesc',
  },

  // ---- library ----
  {
    section: 'library',
    anchor: 'storageThreshold',
    labelKey: 'settings.storageThresholdLabel',
    descKey: 'settings.storageThresholdDesc',
  },
  {
    section: 'library',
    anchor: 'storageWarnEnabled',
    labelKey: 'settings.storageWarnEnabledLabel',
    descKey: 'settings.storageWarnEnabledDesc',
  },
  {
    section: 'library',
    anchor: 'profileExport',
    labelKey: 'settings.profileExportLabel',
    descKey: 'settings.profileExportDesc',
  },
  {
    section: 'library',
    anchor: 'profileImport',
    labelKey: 'settings.profileImportLabel',
    descKey: 'settings.profileImportDesc',
  },

  // ---- security ----
  {
    section: 'security',
    anchor: 'requireSigned',
    labelKey: 'settings.requireSignedLabel',
    descKey: 'settings.requireSignedDesc',
  },
  {
    section: 'security',
    anchor: 'warnUnsigned',
    labelKey: 'settings.warnUnsignedLabel',
    descKey: 'settings.warnUnsignedDesc',
  },

  // ---- keys (鍵あり時のみ表示) ----
  {
    section: 'keys',
    anchor: 'keyId',
    labelKey: 'settings.keyIdLabel',
    visible: (c) => c.hasKeystore,
  },
  {
    section: 'keys',
    anchor: 'publicKey',
    labelKey: 'settings.publicKeyLabel',
    descKey: 'settings.publicKeyDesc',
    visible: (c) => c.hasKeystore,
  },
  {
    section: 'keys',
    anchor: 'exportPrivate',
    labelKey: 'settings.exportPrivateLabel',
    descKey: 'settings.exportPrivateDesc',
    visible: (c) => c.hasKeystore,
  },
  {
    section: 'keys',
    anchor: 'regenerate',
    labelKey: 'settings.regenerateLabel',
    descKey: 'settings.regenerateDesc',
    visible: (c) => c.hasKeystore,
  },
  {
    section: 'keys',
    anchor: 'deleteKey',
    labelKey: 'settings.deleteKeyLabel',
    descKey: 'settings.deleteKeyDesc',
    visible: (c) => c.hasKeystore,
  },

  // ---- keys (鍵なし時のみ表示) ----
  {
    section: 'keys',
    anchor: 'generate',
    labelKey: 'settings.generateLabel',
    descKey: 'settings.generateDesc',
    visible: (c) => !c.hasKeystore,
  },
  {
    section: 'keys',
    anchor: 'importExisting',
    labelKey: 'settings.importExistingLabel',
    descKey: 'settings.importExistingDesc',
    visible: (c) => !c.hasKeystore,
  },

  // ---- logging ----
  {
    section: 'logging',
    anchor: 'logLevel',
    labelKey: 'settings.logLevelLabel',
    descKey: 'settings.logLevelDesc',
  },
  {
    section: 'logging',
    anchor: 'retention',
    labelKey: 'settings.retentionLabel',
    descKey: 'settings.retentionDesc',
  },
  {
    section: 'logging',
    anchor: 'maxSize',
    labelKey: 'settings.maxSizeLabel',
    descKey: 'settings.maxSizeDesc',
  },
  {
    section: 'logging',
    anchor: 'openLogFolder',
    labelKey: 'settings.openLogFolderLabel',
    descKey: 'settings.openLogFolderDesc',
  },
  {
    section: 'logging',
    anchor: 'crashReporting',
    labelKey: 'settings.crashReportingLabel',
    descKey: 'settings.crashReportingDesc',
  },
  {
    section: 'logging',
    anchor: 'crashCount',
    labelKey: 'settings.crashReportsCountLabel',
    // 件数表示の desc は動的 (件数によって切り替わる) のため、空時の文言を検索対象に含める。
    descKey: 'settings.crashReportsEmptyDesc',
  },
  {
    section: 'logging',
    anchor: 'submitCrash',
    labelKey: 'settings.crashReportSubmitLabel',
    descKey: 'settings.crashReportSubmitDesc',
  },
  {
    section: 'logging',
    anchor: 'clearCrash',
    labelKey: 'settings.crashReportClearLabel',
    descKey: 'settings.crashReportClearDesc',
  },

  // ---- updates ----
  {
    section: 'updates',
    anchor: 'autoUpdate',
    labelKey: 'settings.autoUpdateLabel',
    descKey: 'settings.autoUpdateDesc',
  },
  { section: 'updates', anchor: 'checkNow', labelKey: 'settings.checkNowLabel' },

  // ---- about ----
  { section: 'about', anchor: 'homepage', labelKey: 'settings.homepageLabel' },
  { section: 'about', anchor: 'issues', labelKey: 'settings.issuesLabel' },
  { section: 'about', anchor: 'ossLicense', labelKey: 'settings.ossLicenseLabel' },
]

const MESSAGES: Record<Locale, typeof ja> = { ja, en: en as typeof ja }

function entryHaystacks(e: SettingsSearchEntry): string[] {
  const out: string[] = []
  for (const lang of ['ja', 'en'] as const) {
    const l = lookupMessage(MESSAGES[lang], e.labelKey)
    if (l) out.push(l)
    if (e.descKey) {
      const d = lookupMessage(MESSAGES[lang], e.descKey)
      if (d) out.push(d)
    }
  }
  return out
}

/** 純関数の検索本体。Vitest からも直接呼べる */
export function searchSettings(query: string, locale: Locale, ctx: SearchContext): SearchResult[] {
  const trimmed = query.trim()
  if (!trimmed) return []
  const nq = normalize(trimmed)
  const msgs = MESSAGES[locale]

  const out: SearchResult[] = []
  for (const e of CATALOG) {
    if (e.visible && !e.visible(ctx)) continue
    const hit = entryHaystacks(e).some((h) => normalize(h).includes(nq))
    if (!hit) continue
    out.push({
      entry: e,
      displayLabel: lookupMessage(msgs, e.labelKey) ?? e.labelKey,
      displaySectionLabel: lookupMessage(msgs, SECTION_LABEL_KEYS[e.section]) ?? e.section,
    })
  }
  return out
}

/** UI 状態管理付きのコンポーザブル本体 */
export function useSettingsSearch(opts: {
  query: Ref<string>
  locale: Ref<Locale>
  context: Ref<SearchContext>
  sectionRef: Ref<SettingsSectionId>
}) {
  const open = ref(false)
  const activeIndex = ref(0)
  const HARD_LIMIT = 8

  const results = computed<SearchResult[]>(() =>
    searchSettings(opts.query.value, opts.locale.value, opts.context.value),
  )
  const visibleResults = computed(() => results.value.slice(0, HARD_LIMIT))
  const overflowCount = computed(() => Math.max(0, results.value.length - HARD_LIMIT))

  function focus() {
    open.value = true
    activeIndex.value = 0
  }
  function close() {
    open.value = false
  }
  function moveActive(delta: number) {
    const n = visibleResults.value.length
    if (n === 0) return
    activeIndex.value = (activeIndex.value + delta + n) % n
  }
  function resetActive() {
    activeIndex.value = 0
  }

  async function jumpTo(entry: SettingsSearchEntry) {
    opts.sectionRef.value = entry.section
    await nextTick()
    if (typeof document === 'undefined') return
    const el = document.querySelector(
      `[data-search-anchor="${entry.anchor}"]`,
    ) as HTMLElement | null
    if (!el) return
    // scrollIntoView は ancestor 全てを巻き込む (overflow:hidden の .main / .body
    // でも scrollTop が動いてしまい、ユーザーから見えるスクロールバーで戻せなくなる)。
    // 設定コンテンツ内だけ手動でスクロールする。
    const container = el.closest('[data-settings-scroll]') as HTMLElement | null
    if (container) {
      const cRect = container.getBoundingClientRect()
      const eRect = el.getBoundingClientRect()
      const targetTop =
        container.scrollTop + (eRect.top - cRect.top) - (cRect.height - eRect.height) / 2
      const maxTop = Math.max(0, container.scrollHeight - container.clientHeight)
      container.scrollTo({
        top: Math.max(0, Math.min(maxTop, targetTop)),
        behavior: 'smooth',
      })
    }
    el.classList.add('is-search-hit')
    setTimeout(() => el.classList.remove('is-search-hit'), 1500)
  }

  return {
    open,
    activeIndex,
    results,
    visibleResults,
    overflowCount,
    focus,
    close,
    moveActive,
    resetActive,
    jumpTo,
  }
}
