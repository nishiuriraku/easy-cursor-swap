#!/usr/bin/env node
/**
 * gen-architecture.mjs — reference/index.json (Layer3 機械マニフェスト) を生成する。
 *
 * 正準は「markdown (Layer1) + コード」の 1 系統のみ。index.json は生成物で手書き禁止。
 * 本スクリプトの責務:
 *   1. コードから実測カウントを採る (lib.rs pub mod / generate_handler! / glob)。
 *   2. vault の Layer1 ノート frontmatter から ipc:/modules:/invariants: を収集し
 *      name -> note の所有権マップを作る。
 *   3. index.json (measured_counts + modules[] + ipc[] + invariants[]) を出力。
 *   4. --check: source の IPC/module がちょうど 1 ノートに claim されているか検査。
 *      未記載 (source にあるが claim 無し) / stale (claim あるが source 無し) /
 *      重複 claim / count 不一致 を hardDrift として exit 1。
 *
 * vault パスは ARCH_DOCS_DIR で上書き可。vault が無い環境 (CI) では skip して exit 0。
 */

import { readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs'
import { join, dirname, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const DEFAULT_DOCS_DIR = join(REPO_ROOT, '..', 'Obsidian', 'develop', 'easy-cursor-swap')
const DOCS_DIR = process.env.ARCH_DOCS_DIR ?? DEFAULT_DOCS_DIR
const INDEX_JSON = join(DOCS_DIR, 'reference', 'index.json')
const CHECK_MODE = process.argv.includes('--check')

function today() {
  const d = new Date()
  const p = (n) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

function readText(rel) {
  return readFileSync(join(REPO_ROOT, rel), 'utf8')
}

function listDir(rel) {
  const abs = join(REPO_ROOT, rel)
  if (!existsSync(abs)) return []
  return readdirSync(abs, { withFileTypes: true })
}

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

/** lib.rs の pub mod 名一覧。 */
function rustModules() {
  const text = readText('src-tauri/src/lib.rs')
  return [...text.matchAll(/^pub mod\s+([a-z_][a-z0-9_]*)\s*;/gm)].map((m) => m[1])
}

/** generate_handler![] に登録された IPC 名一覧 (公開 IPC サーフェスの唯一の正準)。 */
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
    const line = raw.replace(/\/\/.*$/, '').trim()
    if (!line) continue
    const path = line.replace(/,\s*$/, '').trim()
    const seg = path.split('::').pop()
    if (/^[a-z_][a-z0-9_]*$/.test(seg)) names.push(seg)
  }
  return names
}

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

/** vault 配下の .md を再帰収集 (絶対パス)。legacy/superpowers/log は凍結・スキャン対象外。 */
function walkDocs(absDir) {
  if (!existsSync(absDir)) return []
  const out = []
  for (const e of readdirSync(absDir, { withFileTypes: true })) {
    const child = join(absDir, e.name)
    if (e.isDirectory()) {
      if (e.name === 'legacy' || e.name === 'superpowers' || e.name === 'log') continue
      out.push(...walkDocs(child))
    } else if (e.name.endsWith('.md')) {
      out.push(child)
    }
  }
  return out
}

/**
 * frontmatter から指定キーの配列値を取り出す。inline (`key: [a, b]`) と
 * block (`key:\n  - a\n  - b`) の両形式に対応。値が無ければ [] を返す。
 */
function fmList(fm, key) {
  const inline = fm.match(new RegExp(`^${key}\\s*:\\s*\\[([^\\]]*)\\]`, 'm'))
  if (inline) {
    return inline[1]
      .split(',')
      .map((s) => s.trim().replace(/^['"]|['"]$/g, ''))
      .filter(Boolean)
  }
  const block = fm.match(new RegExp(`^${key}\\s*:\\s*\\n((?:\\s*-\\s*.+\\n?)+)`, 'm'))
  if (block) {
    return block[1]
      .split('\n')
      .map((l) =>
        l
          .replace(/^\s*-\s*/, '')
          .trim()
          .replace(/^['"]|['"]$/g, ''),
      )
      .filter(Boolean)
  }
  return []
}

/** ファイル先頭の `---\n...\n---` frontmatter ブロックを返す (無ければ '')。 */
function frontmatterOf(text) {
  const m = text.match(/^---\n([\s\S]*?)\n---/)
  return m ? m[1] : ''
}

/** rust ファイルパス (src-tauri/src/x.rs or src-tauri/src/x/mod.rs) -> モジュール名。 */
function moduleNameOf(filePath) {
  const m = filePath.match(/src-tauri\/src\/([a-z_][a-z0-9_]*)/)
  return m ? m[1] : null
}

function diff(a, b) {
  const set = new Set(b)
  return a.filter((x) => !set.has(x))
}

function main() {
  if (!existsSync(DOCS_DIR)) {
    console.log(`[gen-architecture] skip: vault docs dir が無い (${DOCS_DIR})`)
    console.log('  vault が無い環境 (CI 等) では正常。ARCH_DOCS_DIR で明示可。')
    process.exit(0)
  }

  const counts = measure()
  const srcModules = rustModules()
  const srcIpc = ipcCommands()

  // --- vault frontmatter から所有権マップ収集 (決定的順序: 絶対パスでソート) ---
  // shared/invariants.md は specs/* より前に処理され、invariant の正準ホームになる (first-wins)。
  const docFiles = walkDocs(DOCS_DIR).sort()
  const ipcOwner = new Map() // ipcName -> noteRel
  const moduleOwner = new Map() // moduleName -> { file, note }
  const invariantOwner = new Map() // invariantId -> noteRel
  const ipcDup = []
  const moduleDup = []

  for (const abs of docFiles) {
    const noteRel = relative(DOCS_DIR, abs).replace(/\\/g, '/')
    const fm = frontmatterOf(readFileSync(abs, 'utf8'))
    if (!fm) continue
    for (const name of fmList(fm, 'ipc')) {
      const ex = ipcOwner.get(name)
      if (ex) {
        if (ex !== noteRel) ipcDup.push(`${name} (${ex} & ${noteRel})`)
      } else ipcOwner.set(name, noteRel)
    }
    for (const file of fmList(fm, 'modules')) {
      const name = moduleNameOf(file)
      if (!name) continue
      const ex = moduleOwner.get(name)
      if (ex) {
        if (ex.note !== noteRel) moduleDup.push(`${name} (${ex.note} & ${noteRel})`)
      } else moduleOwner.set(name, { file, note: noteRel })
    }
    for (const id of fmList(fm, 'invariants')) {
      if (!invariantOwner.has(id)) invariantOwner.set(id, noteRel)
    }
  }

  // --- ドリフト判定 ---
  const claimedIpc = [...ipcOwner.keys()]
  const claimedModules = [...moduleOwner.keys()]
  // main / lib は pub mod でない (バイナリ entry + module 宣言集約) ので source 実測に出ない
  //   → backend-overview.md が claim しても stale 扱いしない。
  // commands は pub mod だが IPC ハブで spec を持たない → undocumented 扱いしない。
  const MODULE_ALLOWLIST = new Set(['main', 'lib', 'commands'])
  const ipcUndocumented = diff(srcIpc, claimedIpc)
  const ipcStale = diff(claimedIpc, srcIpc)
  const moduleUndocumented = diff(srcModules, claimedModules).filter(
    (n) => !MODULE_ALLOWLIST.has(n),
  )
  const moduleStale = diff(claimedModules, srcModules).filter((n) => !MODULE_ALLOWLIST.has(n))

  // --- 既存 index.json の現状値 (count diff 用) ---
  let currentCounts = {}
  if (existsSync(INDEX_JSON)) {
    try {
      const prev = JSON.parse(readFileSync(INDEX_JSON, 'utf8'))
      currentCounts = prev.measured_counts ?? {}
    } catch {
      /* 壊れていれば再生成で上書き */
    }
  }
  const changed = []
  for (const [k, v] of Object.entries(counts)) {
    if (currentCounts[k] !== v) changed.push(`${k}: ${currentCounts[k]} → ${v}`)
  }

  // --- レポート ---
  console.log('=== gen-architecture (index.json) report ===')
  console.log(`target: ${INDEX_JSON}`)
  console.log('measured counts:', JSON.stringify(counts))
  console.log(changed.length ? 'count changes:\n  ' + changed.join('\n  ') : 'count changes: none')

  const driftLines = []
  if (ipcUndocumented.length)
    driftLines.push(`IPC 未記載 (どの note も claim せず): ${ipcUndocumented.join(', ')}`)
  if (ipcStale.length)
    driftLines.push(`IPC stale (note が claim するが source 無し): ${ipcStale.join(', ')}`)
  if (moduleUndocumented.length) driftLines.push(`module 未記載: ${moduleUndocumented.join(', ')}`)
  if (moduleStale.length) driftLines.push(`module stale: ${moduleStale.join(', ')}`)
  if (ipcDup.length) driftLines.push(`IPC 二重 claim: ${ipcDup.join(', ')}`)
  if (moduleDup.length) driftLines.push(`module 二重 claim: ${moduleDup.join(', ')}`)
  console.log(
    driftLines.length
      ? '\nownership drift:\n  ⚠ ' + driftLines.join('\n  ⚠ ')
      : 'ownership drift: none',
  )

  const hardDrift =
    ipcUndocumented.length ||
    ipcStale.length ||
    moduleUndocumented.length ||
    moduleStale.length ||
    ipcDup.length ||
    moduleDup.length ||
    changed.length

  if (CHECK_MODE) {
    if (hardDrift) {
      console.error(
        '\n[--check] ドリフト検出。`node scripts/gen-architecture.mjs` で index.json を再生成し、' +
          '未記載/stale はノート frontmatter を手修正してください。',
      )
      process.exit(1)
    }
    console.log('\n[--check] OK: ドリフトなし。')
    process.exit(0)
  }

  // --- 生成 (決定的な順序: source 順) ---
  const modules = srcModules
    .filter((n) => moduleOwner.has(n))
    .map((n) => ({ name: n, file: moduleOwner.get(n).file, note: moduleOwner.get(n).note }))
  const ipc = srcIpc.filter((n) => ipcOwner.has(n)).map((n) => ({ name: n, note: ipcOwner.get(n) }))
  const invariants = [...invariantOwner.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([id, note]) => ({ id, note }))

  const out = {
    $schema_version: '0.2.0',
    generated_at: today(),
    generated_from: [
      'src-tauri/src/lib.rs (pub mod)',
      'src-tauri/src/commands/mod.rs (generate_handler!)',
      'app/** globs',
      'vault Layer1 frontmatter (ipc/modules/invariants)',
    ],
    measured_counts: counts,
    modules,
    ipc,
    invariants,
  }
  writeFileSync(INDEX_JSON, JSON.stringify(out, null, 2) + '\n')
  console.log(
    `\nindex.json を生成しました (modules ${modules.length} / ipc ${ipc.length} / invariants ${invariants.length})。`,
  )
  if (driftLines.length) console.log('※ 上記 drift は手修正が必要 (生成物には反映されない)。')
}

main()
