#!/usr/bin/env node
/**
 * Tauri Updater 鍵ペア / latest.json 整合性チェッカー。
 *
 * 検証項目:
 *   1. `src-tauri/tauri.conf.json` の `plugins.updater.pubkey` が
 *      `src-tauri/signing/easycursorswap.pub` を base64 エンコードした内容と一致するか
 *   2. `plugins.updater.endpoints[*]` が到達可能で JSON として妥当か
 *   3. latest.json の `platforms.*.{url,signature}` が存在し、各 URL が HEAD で 200 / 302 を返すか
 *
 * Usage:
 *   node scripts/check-updater-keypair.mjs           # 全項目チェック
 *   node scripts/check-updater-keypair.mjs --offline # ネットワーク呼び出しを skip (1 のみ)
 */
import { readFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = join(__dirname, '..')
const offline = process.argv.includes('--offline')

const results = []
let failed = 0

function ok(label, detail = '') {
  results.push({ kind: 'ok', label, detail })
}
function ng(label, detail = '') {
  results.push({ kind: 'ng', label, detail })
  failed++
}
function info(label, detail = '') {
  results.push({ kind: 'info', label, detail })
}

// ---------------------------------------------------------------
// 1. pubkey 同一性チェック
// ---------------------------------------------------------------
const tauriConfPath = join(ROOT, 'src-tauri/tauri.conf.json')
const pubKeyPath = join(ROOT, 'src-tauri/signing/easycursorswap.pub')

let tauriConf
try {
  tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf-8'))
} catch (e) {
  ng('tauri.conf.json の読み込み失敗', String(e))
  printAndExit()
}

const configuredPubkey = tauriConf?.plugins?.updater?.pubkey
if (!configuredPubkey) {
  ng('tauri.conf.json に plugins.updater.pubkey が無い')
} else if (!existsSync(pubKeyPath)) {
  ng('signing/easycursorswap.pub が見つからない', pubKeyPath)
} else {
  // `tauri signer generate` が出力する `.pub` は「base64 でエンコードされた
  // minisign フォーマット文字列」をそのまま 1 行で保存する形式。
  // tauri.conf.json の pubkey フィールドもまったく同じ文字列を入れる契約なので、
  // 再 base64 化はしない (= ファイル内容を trim して直接比較)。
  const pubFileContent = readFileSync(pubKeyPath, 'utf-8').trim()
  if (configuredPubkey === pubFileContent) {
    ok('pubkey 同一性', `${configuredPubkey.slice(0, 24)}...`)
    // 復号確認: base64 デコードして minisign 行が見えるか
    try {
      const decoded = Buffer.from(pubFileContent, 'base64').toString('utf-8')
      const m = decoded.match(/minisign public key: ([A-Fa-f0-9]+)/)
      if (m) {
        info('minisign key id', m[1])
      } else {
        ng('pubkey が minisign フォーマットでデコードできない')
      }
    } catch (e) {
      ng('pubkey の base64 デコードに失敗', String(e))
    }
  } else {
    ng(
      'pubkey 不一致',
      `tauri.conf.json と easycursorswap.pub が乖離しています。鍵ローテーション後の再エンコード漏れの可能性。`,
    )
  }
}

// ---------------------------------------------------------------
// 2-3. latest.json の到達性と中身
// ---------------------------------------------------------------
const endpoints = tauriConf?.plugins?.updater?.endpoints ?? []
if (endpoints.length === 0) {
  ng('endpoints が空')
} else {
  info('endpoints', endpoints.join(', '))
}

if (offline) {
  info('オフラインモード', 'ネットワーク検査を skip')
  printAndExit()
}

for (const url of endpoints) {
  await checkEndpoint(url)
}

printAndExit()

async function checkEndpoint(url) {
  let json
  try {
    const res = await fetch(url, { redirect: 'follow' })
    if (!res.ok) {
      ng(`endpoint 取得失敗 (${res.status})`, url)
      return
    }
    json = await res.json()
    ok('endpoint 取得', url)
  } catch (e) {
    ng('endpoint 取得失敗', `${url} (${e.message})`)
    return
  }

  if (typeof json?.version !== 'string') {
    ng('latest.json に version が無い', url)
    return
  }
  ok('latest.json.version', json.version)

  const platforms = json.platforms ?? {}
  const names = Object.keys(platforms)
  if (names.length === 0) {
    ng('latest.json.platforms が空', url)
    return
  }

  for (const name of names) {
    const p = platforms[name]
    if (!p?.url) {
      ng(`platforms.${name}.url が無い`)
      continue
    }
    if (!p?.signature) {
      ng(`platforms.${name}.signature が無い`)
      continue
    }
    // signature は minisign フォーマット (untrusted comment 行 + base64 行 + ...)
    if (!p.signature.includes('untrusted comment')) {
      ng(`platforms.${name}.signature が minisign 形式に見えない`)
    }

    // HEAD で URL の存在確認 (GitHub Releases は HEAD で 302 → 200)
    try {
      const r = await fetch(p.url, { method: 'HEAD', redirect: 'follow' })
      if (r.ok) {
        ok(`platforms.${name}.url`, p.url)
      } else {
        ng(`platforms.${name}.url HEAD 失敗 (${r.status})`, p.url)
      }
    } catch (e) {
      ng(`platforms.${name}.url HEAD 例外`, `${p.url} (${e.message})`)
    }
  }
}

function printAndExit() {
  console.log('\n=== Updater Keypair / Endpoint Check ===\n')
  for (const r of results) {
    const tag = r.kind === 'ok' ? '✓' : r.kind === 'ng' ? '✗' : '·'
    console.log(`  ${tag} ${r.label}${r.detail ? `  — ${r.detail}` : ''}`)
  }
  console.log(`\n${failed === 0 ? 'ALL GREEN' : `FAILED: ${failed}`}\n`)
  process.exit(failed === 0 ? 0 : 1)
}
