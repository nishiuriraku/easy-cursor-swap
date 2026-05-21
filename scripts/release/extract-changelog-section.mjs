#!/usr/bin/env node
/**
 * CHANGELOG.md から `## [<version>]` セクションだけを抜き出して stdout に出力する。
 *
 * 背景: GitHub Release ページの本文 (`releaseBody`) に、その版での変更内容を
 *       自動で載せたい。release.yml が tag push をトリガに走るとき、本スクリプトを
 *       呼んで該当セクションを取り出し、`tauri-action` の releaseBody に埋め込む。
 *
 * Usage:
 *   node scripts/release/extract-changelog-section.mjs v0.0.4
 *   node scripts/release/extract-changelog-section.mjs 0.0.4
 *
 *   `v` prefix は許容 (引数からは取り除いて比較する)。
 *
 * Exit codes:
 *   0 — 該当セクションが見つかった (stdout に書き出し済み)
 *   1 — 該当セクションが見つからなかった
 *   2 — 引数不正
 */
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(__dirname, '..', '..')

const raw = process.argv[2]
if (!raw) {
  console.error('Usage: extract-changelog-section.mjs <version>')
  process.exit(2)
}
const version = raw.replace(/^v/, '')

const changelog = readFileSync(resolve(repoRoot, 'CHANGELOG.md'), 'utf8')
const lines = changelog.split('\n')

const headerRe = new RegExp(`^## \\[${version.replace(/\./g, '\\.')}\\]`)
const nextSectionRe = /^## \[/
const linkRefsRe = /^\[[^\]]+\]:\s/

let start = -1
let end = lines.length
for (let i = 0; i < lines.length; i++) {
  if (start === -1) {
    if (headerRe.test(lines[i])) start = i
    continue
  }
  if (nextSectionRe.test(lines[i]) || linkRefsRe.test(lines[i])) {
    end = i
    break
  }
}

if (start === -1) {
  console.error(`No CHANGELOG section found for [${version}]`)
  process.exit(1)
}

const section = lines.slice(start, end).join('\n').trimEnd()
process.stdout.write(section + '\n')
