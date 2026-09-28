import { readFileSync } from 'node:fs'
import path from 'node:path'
import { browser, $, expect } from '@wdio/globals'

const cfgPath = path.join(process.env.LOCALAPPDATA ?? '', 'EasyCursorSwap', 'config.json')
const readCfg = () => JSON.parse(readFileSync(cfgPath, 'utf8'))

describe('settings persist', () => {
  it('toggling start_minimized survives Save + relaunch', async () => {
    await $('[data-testid="nav-settings"]').click()
    const toggle = await $('[data-testid="toggle-startMinimized"]')
    await toggle.waitForDisplayed()
    const before = (await toggle.getAttribute('aria-pressed')) === 'true'
    await toggle.click()
    // settings.vue の Save ボタン (common.save)。文言非依存にするため form 内の submit を使う。
    await $('button[type="submit"], button.primary*=Save').click()
    await browser.waitUntil(() => readCfg().general.start_minimized === !before, {
      timeout: 10_000,
    })

    await browser.reloadSession() // tauri-driver がアプリを再起動する
    await $('[data-testid="nav-settings"]').waitForDisplayed()
    await $('[data-testid="nav-settings"]').click()
    const after = await $('[data-testid="toggle-startMinimized"]')
    await after.waitForDisplayed()
    expect(await after.getAttribute('aria-pressed')).toBe(String(!before))
    // 後片付け: 元に戻す
    await after.click()
    await $('button[type="submit"], button.primary*=Save').click()
    await browser.waitUntil(() => readCfg().general.start_minimized === before, {
      timeout: 10_000,
    })
  })
})
