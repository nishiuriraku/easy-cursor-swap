import { execSync } from 'node:child_process'
import { browser, $, expect } from '@wdio/globals'

function regArrow(): string {
  // 例: "    Arrow    REG_EXPAND_SZ    %SystemRoot%\cursors\aero_arrow.cur"
  const out = execSync('reg query "HKCU\\Control Panel\\Cursors" /v Arrow', { encoding: 'utf8' })
  const m = out.match(/Arrow\s+REG_(?:EXPAND_)?SZ\s*(.*)$/m)
  return (m?.[1] ?? '').trim()
}

async function runPanicStage(stage: 1 | 2) {
  await $('[data-testid="panic-open"]').click()
  const dialog = await $('[aria-labelledby="panic-dialog-title"]')
  await dialog.waitForDisplayed()
  await $(`[data-testid="panic-stage-${stage}"]`).click()
  // 完了文言 (panic.completeLabel) が出るまで待つ。文言は i18n 依存なので dialog 内の
  // aria-live 領域に "100%" 相当の完了状態が来るのを待つ代わりに、ダイアログが閉じられる
  // 状態 (close ボタンが enabled) を待つ。実装は PanicFlow.vue:185-235 を見て調整する。
  await browser.waitUntil(async () => (await dialog.getText()).match(/Stage \d/i) !== null, {
    timeout: 30_000,
  })
  await browser.keys('Escape')
}

describe('panic → registry', () => {
  it('stage 1 resets HKCU cursors to empty (Windows default), stage 2 restores snapshot', async () => {
    const baseline = regArrow() // CI runner 初期値 (aero_arrow.cur)
    await runPanicStage(1)
    // reset_to_windows_default は 17 役割を "" にする (registry/mod.rs:294-297)
    await browser.waitUntil(() => regArrow() === '', {
      timeout: 15_000,
      timeoutMsg: 'Arrow not cleared',
    })
    await runPanicStage(2)
    // _initial_snapshot.json は初回起動時の値 = baseline
    await browser.waitUntil(() => regArrow() === baseline, { timeout: 15_000 })
  })
})
