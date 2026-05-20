#!/usr/bin/env node
// Re-embed docs/architecture.json into docs/architecture.html.
// Run whenever architecture.json is updated, in the same commit.
//
// Usage: node scripts/embed-arch-json.mjs

import { readFileSync, writeFileSync } from 'node:fs'

const htmlPath = 'docs/architecture.html'
const jsonPath = 'docs/architecture.json'

const html = readFileSync(htmlPath, 'utf8')
const json = readFileSync(jsonPath, 'utf8').trimEnd()

const openTag = '<script id="data" type="application/json">'
const closeTag = '</script>'

const i = html.indexOf(openTag)
const k = html.indexOf(closeTag, i)
if (i < 0 || k < 0) {
  console.error(`Could not find embedded JSON block in ${htmlPath}`)
  process.exit(1)
}

const next = html.slice(0, i) + openTag + '\n' + json + '\n' + html.slice(k)
writeFileSync(htmlPath, next)

// Safety check: re-parse the embedded JSON to catch malformed embeds.
const re = /<script id="data" type="application\/json">\n([\s\S]*?)\n<\/script>/
const match = readFileSync(htmlPath, 'utf8').match(re)
if (!match) {
  console.error('Re-embed succeeded but re-extracting the JSON block failed.')
  process.exit(1)
}
JSON.parse(match[1])

console.log(`Embedded ${jsonPath} into ${htmlPath}.`)
