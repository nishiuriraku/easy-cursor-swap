#!/usr/bin/env node
/**
 * gen-architecture.mjs — architecture.json の「実測可能な事実」を同期する。
 *
 * このスクリプトが所有するのは **derive 可能なスカラー値だけ**:
 *   - top-level `generated_at`
 *   - `meta.measured_counts.*` (modules / IPC / composables / pages / components / CI)
 *
 * narrative (backend.modules[].role, ipc_commands[].frontend_callers,
 * meta.doc_drift_warnings の散文など) は **一切触らない**。
 * architecture.json はコンパクトな inline 配列を多用しているため、
 * JSON.parse → JSON.stringify の round-trip は diff を 2〜3 倍に膨らませる。
 * よって本スクリプトは **テキストのサージカル置換** のみを行い、
 * 変更しないバイトは 1 文字も触らない。
 *
 * module / IPC の name-level ドリフト (ソースにあるが json に無い等) は
 * **コンソールに報告** する。これらは backend.modules[] / ipc_commands[] の
 * 散文更新を伴うため、人間が手で直す前提でレポートするだけ。
 *
 * 使い方:
 *   node scripts/gen-architecture.mjs           # 実測して architecture.json を更新
 *   node scripts/gen-architecture.mjs --check   # 書き込まず、差分/ドリフトがあれば exit 1
 *
 * architecture.json は Obsidian vault 内 (repo 外) にある。パスは
 * 環境変数 ARCH_JSON_PATH で上書き可。未設定時は repo と同じ Workspace/ 配下を既定とする。
 * vault が存在しない環境 (CI runner 等) では skip して exit 0 (gate を壊さない)。
 */

import { readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const DEFAULT_ARCH_JSON = join(
  REPO_ROOT,
  '..',
  'Obsidian',
  'develop',
  'easy-cursor-swap',
  'reference',
  'architecture.json',
)
const ARCH_JSON = process.env.ARCH_JSON_PATH ?? DEFAULT_ARCH_JSON
const CHECK_MODE = process.argv.includes('--check')

/** ローカル日付 (YYYY-MM-DD)。 */
function today() {
  const d = new Date()
  const p = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

function readText(rel) {
  return readFileSync(join(REPO_ROOT, rel), 'utf8')
}

/** ディレクトリ直下の Dirent 一覧 (存在しなければ空配列)。 */
function listDir(rel) {
  const abs = join(REPO_ROOT, rel)
  if (!existsSync(abs)) return []
  return readdirSync(abs, { withFileTypes: true })
}

/** 拡張子で再帰的に .vue 等を集める。 */
function walk(relDir, ext) {
  const abs = join(REPO_ROOT, relDir)
  if (!existsSync(abs)) return []
  const out = []
  for (const e of readdirSync(abs, { withFileTypes: true })) {
    const childRel = join(relDir, e.name)
    if (e.isDirectory()) out.push(...walk(childRel, ext))
    else if (e.name.endsWith(ext)) out.push(childRel)
  }
  return out
}

/** lib.rs の `pub mod` 名一覧 (登録順)。 */
function rustModules() {
  const text = readText('src-tauri/src/lib.rs')
  return [...text.matchAll(/^pub mod\s+([a-z_][a-z0-9_]*)\s*;/gm)].map((m) => m[1])
}

/**
 * commands/mod.rs の generate_handler![] に登録された IPC コマンド名一覧。
 * これが「実際に公開されている IPC サーフェス」の唯一の正準。
 * 生の `#[tauri::command]` grep は未登録関数も拾うため使わない。
 */
function ipcCommands() {
  const text = readText('src-tauri/src/commands/mod.rs')
  const start = text.indexOf('generate_handler![')
  if (start === -1) throw new Error('generate_handler![ が commands/mod.rs に見つからない')
  const open = text.indexOf('[', start)
  let depth = 0
  let end = -1
  for (let i = open; i < text.length; i++) {
    if (text[i] === '[') depth++
    else if (text[i] === ']') {
      depth--
      if (depth === 0) {
        end = i
        break
      }
    }
  }
  if (end === -1) throw new Error('generate_handler! の閉じ ] が見つからない')
  const body = text.slice(open + 1, end)
  const names = []
  for (const raw of body.split('\n')) {
    const line = raw.replace(/\/\/.*$/, '').trim() // 行コメント除去
    if (!line) continue
    const path = line.replace(/,\s*$/, '').trim() // 末尾カンマ除去
    const seg = path.split('::').pop() // 最後のパスセグメント = 関数名
    if (/^[a-z_][a-z0-9_]*$/.test(seg)) names.push(seg)
  }
  return names
}

/** 実測カウントを集計。 */
function measure() {
  const composables = listDir('app/composables').filter(
    (e) => e.isFile() && e.name.endsWith('.ts') && !e.name.endsWith('.test.ts'),
  ).length
  const pagesVue = listDir('app/pages').filter((e) => e.isFile() && e.name.endsWith('.vue')).length
  const pagesTs = listDir('app/pages').filter((e) => e.isFile() && e.name.endsWith('.ts')).length
  const components = walk('app/components', '.vue').length
  const ci = listDir('.github/workflows').filter(
    (e) => e.isFile() && (e.name.endsWith('.yml') || e.name.endsWith('.yaml')),
  ).length

  return {
    rust_modules_in_lib_rs: rustModules().length,
    tauri_ipc_commands: ipcCommands().length,
    composables,
    pages_vue: pagesVue,
    pages_ts_helpers: pagesTs,
    components_total: components,
    ci_workflows: ci,
  }
}

/** "name" : 整数 をサージカルに置換 (見つからなければ throw)。 */
function setScalarCount(text, key, value) {
  const re = new RegExp(`("${key}"\\s*:\\s*)\\d+`)
  if (!re.test(text)) throw new Error(`measured_counts に "${key}" が見つからない`)
  return text.replace(re, `$1${value}`)
}

/** top-level generated_at (YYYY-MM-DD) を置換。generated_from 配列は触らない。 */
function setGeneratedAt(text, date) {
  const re = /("generated_at"\s*:\s*")\d{4}-\d{2}-\d{2}(")/
  if (!re.test(text)) throw new Error('generated_at が見つからない')
  return text.replace(re, `$1${date}$2`)
}

/** 配列の差分: a にあって b に無いもの。 */
function diff(a, b) {
  const set = new Set(b)
  return a.filter((x) => !set.has(x))
}

function main() {
  if (!existsSync(ARCH_JSON)) {
    console.log(`[gen-architecture] skip: architecture.json が無い (${ARCH_JSON})`)
    console.log('  vault が無い環境 (CI 等) では正常。ARCH_JSON_PATH で明示可。')
    process.exit(0)
  }

  const original = readFileSync(ARCH_JSON, 'utf8')
  const arch = JSON.parse(original) // 読み取り専用 (現状値の取得にのみ使う)

  const counts = measure()
  const srcModules = rustModules()
  const srcIpc = ipcCommands()

  // --- name-level drift (コンソール報告のみ) ---
  const jsonModules = (arch.backend?.modules ?? []).map((m) => m.name)
  const jsonIpc = (arch.backend?.ipc_commands ?? []).map((c) => c.name)
  const MODULE_ALLOWLIST = new Set(['main', 'lib', 'lib.rs']) // pub mod でない正規エントリ

  const moduleUndocumented = diff(srcModules, jsonModules) // ソースにあるが json 未記載 → 要対応
  const moduleStale = diff(jsonModules, srcModules).filter((n) => !MODULE_ALLOWLIST.has(n))
  const ipcUndocumented = diff(srcIpc, jsonIpc) // 登録済だが json 未記載 → 要対応
  const ipcStale = diff(jsonIpc, srcIpc) // json にあるが未登録 → 古い記述

  // --- counts diff ---
  const current = arch.meta?.measured_counts ?? {}
  const changed = []
  for (const [k, v] of Object.entries(counts)) {
    if (current[k] !== v) changed.push(`${k}: ${current[k]} → ${v}`)
  }
  const dateStale = arch.generated_at !== today()

  // --- レポート ---
  console.log('=== gen-architecture report ===')
  console.log(`target: ${ARCH_JSON}`)
  console.log('measured counts:', JSON.stringify(counts))
  if (changed.length) console.log('count changes:\n  ' + changed.join('\n  '))
  else console.log('count changes: none')
  if (dateStale) console.log(`generated_at: ${arch.generated_at} → ${today()}`)

  const driftLines = []
  if (moduleUndocumented.length)
    driftLines.push(`backend.modules[] 未記載 (要追加): ${moduleUndocumented.join(', ')}`)
  if (moduleStale.length)
    driftLines.push(`backend.modules[] 古い記述 (要削除/確認): ${moduleStale.join(', ')}`)
  if (ipcUndocumented.length)
    driftLines.push(`backend.ipc_commands[] 未記載 (要追加): ${ipcUndocumented.join(', ')}`)
  if (ipcStale.length)
    driftLines.push(`backend.ipc_commands[] 未登録 (legacy/要削除): ${ipcStale.join(', ')}`)
  if (driftLines.length) {
    console.log('\nname-level drift (json の散文を手で更新すること):')
    for (const l of driftLines) console.log('  ⚠ ' + l)
  } else {
    console.log('name-level drift: none')
  }

  // 「明確なエラー」= 未記載 module/IPC、または stale IPC、または count 不一致
  const hardDrift =
    moduleUndocumented.length || ipcUndocumented.length || ipcStale.length || changed.length

  if (CHECK_MODE) {
    // 構造ドリフト (count 不一致 / 未記載・stale な module・IPC) のみ gate を赤にする。
    // generated_at の鮮度落ちだけでは fail させない (毎日コミットが詰まるのを防ぐ)。
    if (hardDrift) {
      console.error(
        '\n[--check] 構造ドリフト検出。`node scripts/gen-architecture.mjs` で counts を同期し、' +
          'name-level drift があれば json の散文を手修正してください。',
      )
      process.exit(1)
    }
    console.log(
      '\n[--check] OK: 構造ドリフトなし。' +
        (dateStale ? ' (generated_at は鮮度落ち — 任意で再生成可)' : ''),
    )
    process.exit(0)
  }

  // --- サージカル書き込み ---
  let text = original
  for (const [k, v] of Object.entries(counts)) text = setScalarCount(text, k, v)
  text = setGeneratedAt(text, today())

  // 安全確認: 依然として valid JSON か
  JSON.parse(text)

  if (text === original) {
    console.log('\n変更なし (既に同期済み)。')
    return
  }
  writeFileSync(ARCH_JSON, text)
  console.log('\narchitecture.json を更新しました (counts + generated_at)。')
  if (driftLines.length)
    console.log('※ name-level drift は自動修正されません。上記を手で反映してください。')
}

main()
