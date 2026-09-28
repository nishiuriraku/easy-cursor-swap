#!/usr/bin/env node
// 同梱フォントを node_modules から app/assets/fonts/ へコピーする (出所を固定するための手順書き)。
// 対象: Inter 可変ウェイト latin / latin-ext (OFL-1.1)。更新時は @fontsource-variable/inter を bump してこれを再実行。
import { copyFileSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
const SRC = 'node_modules/@fontsource-variable/inter'
const DST = 'app/assets/fonts'
mkdirSync(DST, { recursive: true })
for (const f of ['inter-latin-wght-normal.woff2', 'inter-latin-ext-wght-normal.woff2']) {
  copyFileSync(join(SRC, 'files', f), join(DST, f))
}
copyFileSync(join(SRC, 'LICENSE'), join(DST, 'LICENSE-Inter.txt'))
console.log('fonts synced')
