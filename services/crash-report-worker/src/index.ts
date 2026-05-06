/**
 * EasyCursorSwap Crash Report Receiver (Cloudflare Worker)
 *
 * 役割:
 *   1. Tauri アプリから匿名 POST /crash で送られてくる JSON を検証
 *   2. SHA-256 デデュープ + IP レート制限 (KV)
 *   3. nishiuriraku/easy-cursor-swap-index/issues に Issue を作成
 *
 * 既存 Issue (同シグネチャ) があればコメント追記、なければ新規作成。
 *
 * Env vars / bindings (wrangler.toml で定義):
 *   - GITHUB_TOKEN  : repo:issues 権限を持つ PAT (secret)
 *   - GITHUB_OWNER  : "nishiuriraku"
 *   - GITHUB_REPO   : "easy-cursor-swap-index"
 *   - ALLOWED_ORIGIN: アプリ識別用の任意トークン (Tauri 側で X-App-Token に同じ値)
 *   - RATE_LIMIT_KV : KV 名前空間バインディング (IP × 時間ウィンドウ)
 *   - DEDUP_KV      : KV 名前空間バインディング (signature → issue number)
 */

export interface Env {
  GITHUB_TOKEN: string
  GITHUB_OWNER: string
  GITHUB_REPO: string
  ALLOWED_ORIGIN: string
  RATE_LIMIT_KV: KVNamespace
  DEDUP_KV: KVNamespace
}

interface CrashReport {
  app_version: string
  os: string
  message: string
  location?: string | null
  timestamp_utc: string
  /** 任意: クライアント側でつけたシグネチャ (例: stack の SHA-256 head) */
  signature?: string
}

/** 1 IP あたりの最大投稿数 (時間ウィンドウ) */
const RATE_LIMIT_PER_HOUR = 5
/** Issue 本文の最大長 (アプリ側で制限済みだが二重防御) */
const MAX_BODY_LEN = 8 * 1024
/** message + location 合計の最大長 */
const MAX_FIELD_LEN = 4 * 1024

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url)

    if (request.method === 'OPTIONS') {
      return cors(new Response(null, { status: 204 }))
    }

    if (url.pathname === '/health') {
      return cors(json({ ok: true }))
    }

    if (url.pathname !== '/crash' || request.method !== 'POST') {
      return cors(json({ error: 'not found' }, 404))
    }

    // App identity (誰でも投げ込めない緩い壁)
    const appToken = request.headers.get('X-App-Token') ?? ''
    if (!appToken || appToken !== env.ALLOWED_ORIGIN) {
      return cors(json({ error: 'unauthorized' }, 401))
    }

    // Rate limit by IP
    const ip = request.headers.get('CF-Connecting-IP') ?? 'unknown'
    const rlKey = `rl:${ip}:${hourBucket()}`
    const rlCount = parseInt((await env.RATE_LIMIT_KV.get(rlKey)) ?? '0', 10)
    if (rlCount >= RATE_LIMIT_PER_HOUR) {
      return cors(json({ error: 'rate limited' }, 429))
    }

    let body: CrashReport
    try {
      const raw = await request.text()
      if (raw.length > MAX_BODY_LEN) {
        return cors(json({ error: 'payload too large' }, 413))
      }
      body = JSON.parse(raw) as CrashReport
    } catch {
      return cors(json({ error: 'invalid json' }, 400))
    }

    const validation = validate(body)
    if (validation) {
      return cors(json({ error: validation }, 400))
    }

    // Dedup by signature (or hash of message+location+app_version)
    const sig = body.signature ?? (await sha256Short(
      `${body.app_version}|${body.os}|${body.message}|${body.location ?? ''}`,
    ))

    const dedupKey = `sig:${sig}`
    const existing = await env.DEDUP_KV.get(dedupKey)

    let result: { issue_number: number; action: 'created' | 'commented' }
    try {
      if (existing) {
        await commentIssue(env, parseInt(existing, 10), formatComment(body))
        result = { issue_number: parseInt(existing, 10), action: 'commented' }
      } else {
        const issue = await createIssue(env, body, sig)
        // 30 日 TTL: 古い Issue が closed されてもいずれ新規にする
        await env.DEDUP_KV.put(dedupKey, String(issue.number), {
          expirationTtl: 30 * 24 * 60 * 60,
        })
        result = { issue_number: issue.number, action: 'created' }
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      return cors(json({ error: `github upstream: ${msg}` }, 502))
    }

    // Bump rate limit (TTL 1h)
    await env.RATE_LIMIT_KV.put(rlKey, String(rlCount + 1), { expirationTtl: 3600 })

    return cors(json({ ok: true, ...result }))
  },
}

// ----- helpers -----

function validate(b: CrashReport): string | null {
  if (typeof b.app_version !== 'string' || b.app_version.length > 64) {
    return 'app_version invalid'
  }
  if (typeof b.os !== 'string' || b.os.length > 32) {
    return 'os invalid'
  }
  if (typeof b.message !== 'string' || b.message.length > MAX_FIELD_LEN) {
    return 'message invalid'
  }
  if (b.location != null && (typeof b.location !== 'string' || b.location.length > MAX_FIELD_LEN)) {
    return 'location invalid'
  }
  if (typeof b.timestamp_utc !== 'string' || !/^\d{4}-\d{2}-\d{2}T/.test(b.timestamp_utc)) {
    return 'timestamp_utc invalid'
  }
  if (b.signature != null && (typeof b.signature !== 'string' || b.signature.length > 128)) {
    return 'signature invalid'
  }
  return null
}

function hourBucket(): string {
  const d = new Date()
  return `${d.getUTCFullYear()}${String(d.getUTCMonth() + 1).padStart(2, '0')}${String(d.getUTCDate()).padStart(2, '0')}${String(d.getUTCHours()).padStart(2, '0')}`
}

async function sha256Short(input: string): Promise<string> {
  const buf = new TextEncoder().encode(input)
  const digest = await crypto.subtle.digest('SHA-256', buf)
  return Array.from(new Uint8Array(digest))
    .slice(0, 8)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

function formatTitle(b: CrashReport): string {
  const head = b.message.split('\n')[0] ?? '(empty)'
  const loc = b.location ? ` @ ${b.location}` : ''
  return `[crash] ${truncate(head, 80)}${loc}`
}

function formatBody(b: CrashReport, sig: string): string {
  return [
    `**App version**: \`${b.app_version}\``,
    `**OS**: \`${b.os}\``,
    `**Timestamp (UTC)**: ${b.timestamp_utc}`,
    `**Signature**: \`${sig}\``,
    '',
    '### Panic message',
    '```',
    truncate(b.message, MAX_FIELD_LEN),
    '```',
    b.location ? `### Location\n\`${truncate(b.location, MAX_FIELD_LEN)}\`` : '',
    '',
    '> Submitted automatically by EasyCursorSwap crash reporter.',
    '> Personal paths are redacted client-side via `redact_path`.',
  ]
    .filter(Boolean)
    .join('\n')
}

function formatComment(b: CrashReport): string {
  return [
    `Recurrence reported (\`${b.app_version}\`, \`${b.os}\`) at ${b.timestamp_utc}`,
    b.location ? `\nLocation: \`${truncate(b.location, 256)}\`` : '',
  ]
    .filter(Boolean)
    .join('')
}

function truncate(s: string, max: number): string {
  return s.length > max ? `${s.slice(0, max - 3)}...` : s
}

async function createIssue(env: Env, b: CrashReport, sig: string) {
  const res = await fetch(
    `https://api.github.com/repos/${env.GITHUB_OWNER}/${env.GITHUB_REPO}/issues`,
    {
      method: 'POST',
      headers: ghHeaders(env),
      body: JSON.stringify({
        title: formatTitle(b),
        body: formatBody(b, sig),
        labels: ['crash-report', `os:${b.os}`, `version:${b.app_version}`],
      }),
    },
  )
  if (!res.ok) {
    throw new Error(`POST /issues ${res.status}: ${await res.text()}`)
  }
  return (await res.json()) as { number: number }
}

async function commentIssue(env: Env, issueNumber: number, body: string) {
  const res = await fetch(
    `https://api.github.com/repos/${env.GITHUB_OWNER}/${env.GITHUB_REPO}/issues/${issueNumber}/comments`,
    {
      method: 'POST',
      headers: ghHeaders(env),
      body: JSON.stringify({ body }),
    },
  )
  if (!res.ok) {
    throw new Error(`POST /issues/${issueNumber}/comments ${res.status}: ${await res.text()}`)
  }
}

function ghHeaders(env: Env): Record<string, string> {
  return {
    'Authorization': `Bearer ${env.GITHUB_TOKEN}`,
    'Accept': 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
    'Content-Type': 'application/json',
    'User-Agent': 'easy-cursor-swap-crash-report-worker',
  }
}

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function cors(res: Response): Response {
  const h = new Headers(res.headers)
  h.set('Access-Control-Allow-Origin', '*')
  h.set('Access-Control-Allow-Methods', 'POST, OPTIONS')
  h.set('Access-Control-Allow-Headers', 'Content-Type, X-App-Token')
  h.set('Access-Control-Max-Age', '86400')
  return new Response(res.body, { status: res.status, headers: h })
}
