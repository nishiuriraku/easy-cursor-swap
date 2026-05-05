# 2. アーキテクチャとコア機能

## 2.1 技術スタックと処理分担
- **Framework:** Tauri v2 (Windowsターゲット)
- **Frontend (Nuxt 4 / Vue.js):** UI描画、SVG/PNGのインポート・CanvasでのラスタライズとRGBAバッファ抽出。
- **Backend (Rust / windows-rs, winreg):** 設定管理(Source of Truth)、レジストリ操作、ファイルI/O(Zip解凍など)、高品質画像処理(Lanczos)、バックグラウンド常駐ループ。

## 2.2 実行モデル・配布形態
- **常駐とメモリ最適化 (エコ仕様):** システムトレイに常駐。ウィンドウ非表示時はNuxtのWebViewエンジンを破棄し、Rustのネイティブループ単独（数MB）で稼働。
- **UAC不要のクリーン設計:** レジストリ操作は `HKCU` のみ、ファイル保存先は `~/.custom_cursors/` のみに限定。日常利用で管理者権限を要求しません。アンインストール時も資産は保持します。
- **自動起動とMSIX仮想化対策:**
  - MSI配布: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
  - MSIX配布: MSIXの仮想化によりレジストリ書き込みがOS本体に反映されない問題を防ぐため、`runFullTrust` を宣言した Packaged Win32 App として構築。自動起動は `windows.startupTask` を使用。
  - WebView2依存: MSI配布時は Evergreen Bootstrapper を同梱し初回起動時サイレント取得。
- **多重起動防止:** Named Mutex を用いて既存インスタンスのトレイへフォーカス。

## 2.3 レジストリ統合と適用ロジック
- **適用:** `HKCU\Control Panel\Cursors` を書き換え。コントロールパネル連携のため `Schemes` にも登録。
  - *制約:* `Schemes` は17パスをカンマ区切りで連結した文字列のため、パスに `,` を含めないようサニタイズ。
- **反映:** `SystemParametersInfoW(SPI_SETCURSORS)` および `SPI_SETCURSORSHADOW` で再起動なしに即時反映。
- **疑似トランザクション（クラッシュ対策）:**
  1. `~/.custom_cursors/_pending_apply.snapshot` をディスク保存。
  2. レジストリ書き換え実行。
  3. 完了時にスナップショット削除。
  （OS強制終了などでファイルが残っていれば次回起動時に自動復元しトースト通知）
- **初期スナップショット:** インストール初回起動時に `_initial_snapshot.json` を保存。他の管理ツール使用状態へいつでも戻せるように担保。
- **確実な復旧機能 (パニックボタン):** `Ctrl+Alt+Shift+R` のホットキー等で、「Windows 既定」または「インストール前の状態」へ即座に復旧。
- **OS設定との同期・孤児復旧:** `WM_SETTINGCHANGE` を購読して外部変更を検知。実体ファイルが手動削除されパス切れになった場合は自動で標準に復旧。

## 2.4 ダークモード監視と Windows 11 機能競合
- **監視イベント:** `AppsUseLightTheme` の初期読込と、`WM_SETTINGCHANGE` (LPARAM `ImmersiveColorSet`) の購読でバックグラウンド検知（WinRT初期化は避ける）。
- **ペアリング:** ライト用・ダーク用のテーマを紐付け、OS切り替え時に自動反映。
- **アクセシビリティ競合:** Win11の `CursorIndicator` / `ContrastScheme` / `CursorBaseSize` と競合した場合は警告ダイアログを表示し、ユーザーにOS設定の一時無効化を判断させる。

## 2.5 パフォーマンス目標と運用
CIでの自動測定・リグレッション検出を義務付けます。102枚生成時などのパフォーマンス最適化（キャッシュ化・プログレス表示）を徹底します。

| 指標 | 目標値 | 測定タイミング |
|---|---|---|
| トレイ常駐 メモリ使用量 | ≤ 15 MB | 起動10分後 (WebView破棄済) |
| メイン画面 メモリ使用量 | ≤ 200 MB | メイン画面起動30秒後 |
| トレイ常駐 CPU 使用率 | ≤ 0.1% | 60秒平均 |
| アプリ起動 (コールド) | ≤ 1.5 秒 | OS自動起動時 |
| テーマ適用 (17×6生成) | ≤ 3.0 秒 | 適用クリックからSPI_SETCURSORS完了 |
| ダークモード切替反応 | ≤ 500 ms | WM_SETTINGCHANGE受信から完了 |

## 2.6 ロギングとアップデート
- **ロギング (`tracing`):** 
  - `%LOCALAPPDATA%\<AppName>\logs\app-YYYY-MM-DD.log` に日次ローテ保存。
  - 最大14日経過、または総容量100MB超過時に古いものから自動削除。
  - **PII（個人情報）除外ポリシー:** 絶対パスは `logging::redact_path` で `~/...` 置換。ハッシュ値は先頭12文字に短縮。RAWレジストリ値やSHA-256全文は出力しない。
- **アップデート (Tauri Updater):** 
  - 署名検証必須。ダウングレード攻撃防止。
  - トリプルハートビート方式（新版起動失敗3回連続で旧版バイナリに自動ロールバック）。
  - アップデート時に `config.json` をバージョン番号付きでマイグレーション。失敗時は強制起動せず `config.bak` に退避しエラー画面。
