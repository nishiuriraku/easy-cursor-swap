#!/usr/bin/env node
/**
 * design/app-icon.svg を 1024×1024 PNG にレンダリングする。
 *
 * Tauri v2 の `npm run tauri icon` は単一の PNG (推奨 1024×1024 以上) を
 * 入力に各プラットフォーム向けの icns / ico / 各サイズ PNG を生成する。
 * このスクリプトが master PNG を作る橋渡しとなる。
 *
 * 実行:
 *   node scripts/generate-icon.mjs            # design/app-icon.png を出力
 *   npm run tauri icon design/app-icon.png    # icns/ico 他を再生成
 */
import { readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Resvg } from '@resvg/resvg-js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = resolve(__dirname, '..')
const svgPath = resolve(root, 'design/app-icon.svg')
const outPath = resolve(root, 'design/app-icon.png')
const SIZE = 1024

const svg = await readFile(svgPath, 'utf8')
const resvg = new Resvg(svg, {
  fitTo: { mode: 'width', value: SIZE },
  background: 'rgba(0,0,0,0)',
})
const png = resvg.render().asPng()
await writeFile(outPath, png)
console.log(`generated ${outPath} (${png.length} bytes, ${SIZE}x${SIZE})`)
