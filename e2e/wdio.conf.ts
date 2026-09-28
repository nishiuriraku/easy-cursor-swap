import { spawn, spawnSync, type ChildProcess } from 'node:child_process'
import path from 'node:path'

// tauri-driver は 4444 で WebDriver を待ち受け、--native-driver の msedgedriver に橋渡しする
let tauriDriver: ChildProcess | undefined
const appPath = path.resolve(__dirname, '../src-tauri/target/debug/easy-cursor-swap.exe')
const nativeDriver = process.env.MSEDGEDRIVER_PATH ?? 'msedgedriver.exe'

export const config: WebdriverIO.Config = {
  runner: 'local',
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./specs/**/*.e2e.ts'],
  maxInstances: 1,
  capabilities: [
    {
      // @ts-expect-error tauri 固有 capability
      'tauri:options': { application: appPath },
    },
  ],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { timeout: 120_000 },
  waitforTimeout: 20_000,
  onPrepare: () => {
    // 事前に debug ビルド済であること (workflow 側で `tauri build --debug --no-bundle`)
    spawnSync('cargo', ['install', 'tauri-driver', '--locked'], { stdio: 'inherit', shell: true })
    tauriDriver = spawn(
      path.resolve(process.env.USERPROFILE ?? '', '.cargo/bin/tauri-driver.exe'),
      ['--native-driver', nativeDriver],
      { stdio: [null, process.stdout, process.stderr] },
    )
  },
  onComplete: () => {
    tauriDriver?.kill()
  },
}
