#!/usr/bin/env node
/**
 * Marketplace 投稿検証スクリプト (Phase 9-4)
 *
 * `cursorforge/index` リポジトリへの PR で動かす想定。
 * 環境変数 `CHANGED_FILES` (改行区切り) に変更ファイル一覧が入っていれば
 * その中の `entries/*.json` を検証対象とする。なければ `entries/*.json` を全走査。
 *
 * 検証ステップ:
 *  1. JSON スキーマ検証 (構造 + 必須フィールド + 型)
 *  2. ファイルサイズ閾値 (`themes/<id>.cursorpack` <= 50MB)
 *  3. SHA-256 整合性 (entry.sha256 == sha256(themes/<id>.cursorpack))
 *  4. Ed25519 署名検証 (`authors/{author_github}.json` の公開鍵)
 *  5. マルウェアハッシュ DB 照合 (`scripts/marketplace/malware-hashes.txt`)
 */
import { readFileSync, statSync, existsSync, readdirSync } from 'node:fs'
import { createHash, createPublicKey, verify } from 'node:crypto'
import { dirname, join, basename } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = join(__dirname, '..', '..')
const ENTRIES_DIR = join(ROOT, 'entries')
const THEMES_DIR = join(ROOT, 'themes')
const AUTHORS_DIR = join(ROOT, 'authors')
const MALWARE_DB = join(__dirname, 'malware-hashes.txt')

const MAX_PACK_BYTES = 50 * 1024 * 1024

function logErr(msg) {
  console.error(`::error::${msg}`)
}

function listEntriesFromEnv() {
  const env = process.env.CHANGED_FILES ?? ''
  const list = env
    .split('\n')
    .map((s) => s.trim())
    .filter((s) => s.startsWith('entries/') && s.endsWith('.json'))
  if (list.length > 0) return list.map((p) => join(ROOT, p))
  // フォールバック: 全エントリ
  if (!existsSync(ENTRIES_DIR)) return []
  return readdirSync(ENTRIES_DIR)
    .filter((f) => f.endsWith('.json'))
    .map((f) => join(ENTRIES_DIR, f))
}

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf-8'))
}

function isUuid(s) {
  return typeof s === 'string'
    && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(s)
}

function validateSchema(entry) {
  const err = []
  const required = [
    'id', 'name', 'author', 'author_github', 'author_pubkey_id',
    'sha256', 'signature', 'download_url', 'version', 'included_roles',
  ]
  for (const f of required) {
    if (!(f in entry)) err.push(`missing field: ${f}`)
  }
  if (entry.id && !isUuid(entry.id)) err.push(`invalid uuid: ${entry.id}`)
  if (entry.sha256 && !/^[0-9a-f]{64}$/i.test(entry.sha256)) {
    err.push(`invalid sha256: ${entry.sha256}`)
  }
  if (entry.included_roles && !Array.isArray(entry.included_roles)) {
    err.push('included_roles must be array')
  }
  if (entry.included_roles && Array.isArray(entry.included_roles)
      && !entry.included_roles.includes('Arrow')) {
    err.push('Arrow ロールは必須です')
  }
  return err
}

function loadAuthor(github) {
  const path = join(AUTHORS_DIR, `${github}.json`)
  if (!existsSync(path)) {
    return { ok: false, error: `authors/${github}.json が存在しません` }
  }
  try {
    return { ok: true, record: loadJson(path) }
  } catch (e) {
    return { ok: false, error: `authors/${github}.json パース失敗: ${e.message}` }
  }
}

function computeKeyId(pubkeyB64) {
  const raw = Buffer.from(pubkeyB64, 'base64')
  return createHash('sha256').update(raw).digest('hex').slice(0, 16)
}

/** Ed25519 公開鍵 (32 バイト) → SPKI DER → KeyObject */
function pubkeyFromB64(pubkeyB64) {
  const raw = Buffer.from(pubkeyB64, 'base64')
  if (raw.length !== 32) {
    throw new Error(`Ed25519 公開鍵長が不正: ${raw.length} bytes`)
  }
  // SPKI prefix for Ed25519: 30 2a 30 05 06 03 2b 65 70 03 21 00
  const prefix = Buffer.from('302a300506032b6570032100', 'hex')
  const der = Buffer.concat([prefix, raw])
  return createPublicKey({ key: der, format: 'der', type: 'spki' })
}

function verifySignature(entry, authorRecord) {
  // key_id 一致確認 (現行 or historical)
  const currentKid = computeKeyId(authorRecord.public_key)
  let pubkeyB64 = null
  if (currentKid === entry.author_pubkey_id) {
    pubkeyB64 = authorRecord.public_key
  } else if (authorRecord.historical_keys
      && authorRecord.historical_keys[entry.author_pubkey_id]) {
    pubkeyB64 = authorRecord.historical_keys[entry.author_pubkey_id]
  } else {
    return { ok: false, error: `key_id ${entry.author_pubkey_id} が著者鍵と一致しません` }
  }

  const pubkey = pubkeyFromB64(pubkeyB64)
  const sigBytes = Buffer.from(entry.signature, 'base64')
  if (sigBytes.length !== 64) {
    return { ok: false, error: `署名長が不正: ${sigBytes.length} bytes` }
  }
  // Marketplace 仕様: SHA-256 hex 文字列に対する Ed25519 署名
  const messageBytes = Buffer.from(entry.sha256.toLowerCase(), 'utf-8')
  const ok = verify(null, messageBytes, pubkey, sigBytes)
  return { ok, error: ok ? null : 'Ed25519 署名検証失敗' }
}

function checkPackFile(entry) {
  const path = join(THEMES_DIR, `${entry.id}.cursorpack`)
  if (!existsSync(path)) {
    return { ok: false, error: `themes/${entry.id}.cursorpack が存在しません` }
  }
  const stat = statSync(path)
  if (stat.size > MAX_PACK_BYTES) {
    return { ok: false, error: `${path} のサイズ ${stat.size} が 50MB を超えています` }
  }
  const sha = createHash('sha256').update(readFileSync(path)).digest('hex')
  if (sha.toLowerCase() !== entry.sha256.toLowerCase()) {
    return { ok: false, error: `SHA-256 不一致 expected=${entry.sha256} actual=${sha}` }
  }
  return { ok: true, sha, size: stat.size }
}

function checkMalware(sha) {
  if (!existsSync(MALWARE_DB)) return { ok: true } // DB 未整備
  const known = readFileSync(MALWARE_DB, 'utf-8')
    .split('\n')
    .map((l) => l.trim().toLowerCase())
    .filter(Boolean)
  if (known.includes(sha.toLowerCase())) {
    return { ok: false, error: `マルウェアハッシュ DB と一致: ${sha}` }
  }
  return { ok: true }
}

function main() {
  const targets = listEntriesFromEnv()
  if (targets.length === 0) {
    console.log('no entries to validate')
    return 0
  }

  let errors = 0
  for (const file of targets) {
    console.log(`\n--- ${basename(file)} ---`)
    let entry
    try {
      entry = loadJson(file)
    } catch (e) {
      logErr(`${file}: JSON パース失敗: ${e.message}`)
      errors++
      continue
    }

    const schemaErrors = validateSchema(entry)
    for (const e of schemaErrors) logErr(`${file}: ${e}`)
    if (schemaErrors.length > 0) {
      errors += schemaErrors.length
      continue
    }

    const author = loadAuthor(entry.author_github)
    if (!author.ok) {
      logErr(`${file}: ${author.error}`)
      errors++
      continue
    }

    const sigResult = verifySignature(entry, author.record)
    if (!sigResult.ok) {
      logErr(`${file}: ${sigResult.error}`)
      errors++
      continue
    }

    const pack = checkPackFile(entry)
    if (!pack.ok) {
      logErr(`${file}: ${pack.error}`)
      errors++
      continue
    }

    const malware = checkMalware(pack.sha)
    if (!malware.ok) {
      logErr(`${file}: ${malware.error}`)
      errors++
      continue
    }

    console.log(`  OK: ${entry.name} (${entry.version}) ${pack.size}B sha=${pack.sha.slice(0, 12)}...`)
  }

  if (errors > 0) {
    console.error(`\n${errors} validation error(s) detected`)
    return 1
  }
  return 0
}

process.exit(main())
