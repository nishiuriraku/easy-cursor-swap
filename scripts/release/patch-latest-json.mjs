#!/usr/bin/env node
/**
 * Tauri Updater latest.json の signature フィールドを、再生成された .sig
 * ファイルの内容で上書きする。
 *
 * 背景: SignPath で Authenticode 署名するとバイナリが書き換わるため、
 *       tauri-action が最初に作った .sig は無効。SignPath 後に
 *       `npx tauri signer sign` を再走させて .sig を作り直し、
 *       latest.json の中身を新内容で更新する必要がある。
 *
 * Usage:
 *   node scripts/release/patch-latest-json.mjs <bundle-dir>
 *
 *   <bundle-dir> 例: src-tauri/target/x86_64-pc-windows-msvc/release/bundle
 */
import { readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs'
import { join, basename } from 'node:path'

const bundleDir = process.argv[2]
if (!bundleDir) {
  console.error('Usage: patch-latest-json.mjs <bundle-dir>')
  process.exit(2)
}

// latest.json を bundle 配下から探す (tauri-action は updater/latest.json に置く)
const candidates = [join(bundleDir, 'latest.json'), join(bundleDir, 'updater', 'latest.json')]
const jsonPath = candidates.find(existsSync)
if (!jsonPath) {
  console.error(`latest.json not found under ${bundleDir}`)
  process.exit(2)
}

const latest = JSON.parse(readFileSync(jsonPath, 'utf-8'))
let patched = 0

for (const [platformName, platform] of Object.entries(latest.platforms ?? {})) {
  // platform.url からファイル名を抜き取り、対応する .sig を探す
  const fileName = basename(new URL(platform.url, 'https://x/').pathname)
  const sigPath = locateSig(bundleDir, fileName)
  if (!sigPath) {
    console.warn(`  ! ${platformName}: no .sig for ${fileName}`)
    continue
  }
  const newSig = readFileSync(sigPath, 'utf-8').trim()
  if (platform.signature === newSig) {
    console.log(`  · ${platformName}: signature unchanged`)
    continue
  }
  platform.signature = newSig
  patched++
  console.log(`  ✓ ${platformName}: signature updated from ${sigPath}`)
}

writeFileSync(jsonPath, JSON.stringify(latest, null, 2) + '\n', 'utf-8')
console.log(`Patched ${patched} platform entries in ${jsonPath}`)

function locateSig(bundleDir, fileName) {
  // {bundleDir}/nsis/{fileName}.sig or /msi/{fileName}.sig
  const dirs = ['nsis', 'msi']
  for (const d of dirs) {
    const candidate = join(bundleDir, d, `${fileName}.sig`)
    if (existsSync(candidate)) return candidate
  }
  // フォールバック: bundleDir 配下を再帰探索
  const stack = [bundleDir]
  while (stack.length) {
    const dir = stack.pop()
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) stack.push(join(dir, entry.name))
      else if (entry.name === `${fileName}.sig`) return join(dir, entry.name)
    }
  }
  return null
}
