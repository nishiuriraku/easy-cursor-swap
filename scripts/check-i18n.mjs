#!/usr/bin/env node
/**
 * i18n キー差分チェッカー。
 * `app/locales/ja.ts` と `app/locales/en.ts` のキー集合を比較し、
 * どちらかにしか存在しないキーがあれば exit 1 で CI を落とす。
 */
import { readFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = join(__dirname, '..')

/**
 * `.ts` ファイルから `export default` のオブジェクトリテラルを雑に抽出して
 * JSON 風にパースする。`as const` などの構文は除去。
 */
function parseLocale(filePath) {
  const src = readFileSync(filePath, 'utf-8')
  const match = src.match(/export default\s*({[\s\S]*?})\s*as const/)
  if (!match) {
    throw new Error(`export default not found in ${filePath}`)
  }
  // TS 構文を JSON5 風に近づける: シングルクォート → ダブル、トレーリングカンマ削除、
  // コメント除去、プロパティ名にダブルクォート付与。
  let body = match[1]
  body = body.replace(/\/\/.*$/gm, '') // 行コメント
  body = body.replace(/\/\*[\s\S]*?\*\//g, '') // ブロックコメント
  body = body.replace(/'([^'\\]*(?:\\.[^'\\]*)*)'/g, (_m, s) => `"${s.replace(/"/g, '\\"')}"`)
  body = body.replace(/([{,]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:/g, '$1"$2":')
  body = body.replace(/,(\s*[}\]])/g, '$1')
  return JSON.parse(body)
}

function flatten(obj, prefix = '') {
  const out = []
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      out.push(...flatten(v, key))
    } else {
      out.push(key)
    }
  }
  return out
}

const ja = parseLocale(join(ROOT, 'app/locales/ja.ts'))
const en = parseLocale(join(ROOT, 'app/locales/en.ts'))

const jaKeys = new Set(flatten(ja))
const enKeys = new Set(flatten(en))

const missingInEn = [...jaKeys].filter((k) => !enKeys.has(k))
const missingInJa = [...enKeys].filter((k) => !jaKeys.has(k))

if (missingInEn.length === 0 && missingInJa.length === 0) {
  console.log(`OK: ja=${jaKeys.size}, en=${enKeys.size} keys (parity)`)
  process.exit(0)
}

if (missingInEn.length > 0) {
  console.error('Keys missing in en.ts:')
  for (const k of missingInEn) console.error(`  - ${k}`)
}
if (missingInJa.length > 0) {
  console.error('Keys missing in ja.ts:')
  for (const k of missingInJa) console.error(`  - ${k}`)
}
process.exit(1)
