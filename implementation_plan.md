# 🖱️ CursorForge - 実装計画 v2

---

## 🛠️ 実装手順 (毎回厳守)

> [!IMPORTANT]
> 1 機能 = 1 コミット。下記の検証ゲートを **すべて** 通過するまでコミットしない。

### ステップ
1. **タスク選定**: 「次回セッションでの優先タスク」または「優先度の高いギャップ」から 1 つ選択
2. **実装**: コードを編集 (Rust / Vue / 設定ファイル等)
3. **検証ゲート (全部 PASS 必須)**:
    - `cargo check --manifest-path src-tauri/Cargo.toml` → エラー0
    - `cargo test --manifest-path src-tauri/Cargo.toml` → 全テスト PASS
    - `npm run tauri:build` → MSI/NSIS バンドルまで成功
       - ※ コード署名鍵が無いため `WARN skipping code signing` は許容
       - ※ フロントエンドのみの変更でも `tauri:build` で最終確認
4. **計画更新**: `implementation_plan.md` の該当 Phase / 「次回セッションでの優先タスク」/ 「優先度の高いギャップ」を更新
5. **コミット**: 日本語コミットメッセージ。`feat:` / `fix:` / `perf:` / `chore:` 等の prefix。
6. **次タスクへ**: 1 に戻る

### コーディング規約
- Rust コードのコメントは日本語
- Vue は Composition API + `<script setup>`
- CSS は Vanilla CSS (Tailwind 不使用)
- `v-html` 禁止 (XSS 対策、render 関数 + `h('svg', { innerHTML })` で代替)
- i18n キー: `ja.ts` / `en.ts` 完全一致 (parity check スクリプトで検証)

---

## 技術スタック

| レイヤー | 技術 | 役割 |
|---|---|---|
| Frontend | Nuxt 4 / Vue.js | UI描画、画像インポート、Canvas操作 |
| Framework | Tauri v2 | デスクトップアプリケーションフレームワーク |
| Backend | Rust + `windows-rs` + `winreg` | レジストリ操作、画像処理、ファイルI/O、常駐ロジック |
| ビルド | `.msi` / `.msix` | 配布形態 |

---

## Phase 1: プロジェクト基盤構築 🏗️ ✅ 完了

- [x] Nuxt 4 プロジェクト初期化（SPA モード）
- [x] Tauri v2 統合（`tauri init`）
- [x] Rust 依存クレートの追加
- [x] `cargo check` 成功（エラー0, 警告0）
- [x] ディレクトリ構成の整備
- [x] CLAUDE.md / README.md / LICENSE の作成

---

## Phase 2: Rust コアロジック（基盤） 🦀 ✅ 完了

### 2-1: 設定管理 ✅
- [x] `config.json` スキーマ定義（`schema_version` 付き）
- [x] Rust 側での設定読み書き（Source of Truth）
- [x] RwLock によるスレッドセーフなアクセス
- [x] **設定マイグレーション機構** — 古い schema_version は透過的に欠落フィールドを serde default で埋めて schema_version 更新 + バックアップ
- [x] **マイグレーション失敗時の専用エラー画面** — Win32 MessageBox でバックアップ場所と復旧手順を提示 (デフォルト強制起動はしない)
- [x] **`config.bak.v{N}.json` への退避** + `config.corrupt.{epoch}.json` 退避 (パース失敗時)

### 2-2: レジストリ操作 ✅
- [x] `HKCU\Control Panel\Cursors` の読み書き
- [x] `Schemes` への登録
- [x] `SystemParametersInfoW(SPI_SETCURSORS)` による即時反映
- [x] `SystemParametersInfoW(SPI_SETCURSORSHADOW)` 制御
- [x] 適用トランザクション（スナップショット + ロールバック）
- [x] ディスク永続スナップショット（`_pending_apply.snapshot`）
- [ ] `Schemes` の文字列フォーマット制約（セミコロン区切り、`,` 不可、`REG_EXPAND_SZ` 採用判断）
- [x] **OS 設定の外部変更検知** — `cursor_watcher.rs` で `WM_SETTINGCHANGE` の `SPI_SETCURSORS` を購読、`cursor-changed` Tauri イベントを発火、ライブラリ画面が `loadThemes()` で再読込
- [ ] 未指定役割（`SizeAll` 等）を「Windows 標準継承」とするレジストリ書き出し仕様
- [ ] 孤児カーソル復旧（ヘルスチェック）— `~/.custom_cursors/` 手動削除時の自動標準復帰

### 2-3: 初回スナップショット ✅
- [x] 初回起動時に `_initial_snapshot.json` を自動保存
- [x] パニックボタン復旧用のレジストリ復元

### 2-4: カーソルファイル保存 ✅
- [x] `~/.custom_cursors/<UUID>/` ディレクトリ管理
- [x] ファイルパスのサニタイズ（テーマ名）

---

## Phase 3: 画像処理 & `.cur` 生成 🎨 ✅ 基盤完了

### 3-1: 画像処理パイプライン ✅
- [x] Lanczos / Nearest-Neighbor リサイズ（6サイズ自動生成）
- [x] ドット絵自動判定ロジック（色数サンプリング）
- [x] PNG/SVG → RGBA バッファ変換（Nuxt 側 Canvas + sanitizeSvg → rasterizeSvgToPng）
- [x] PNG バイト列の IPC 転送 (build_cur_from_png)
- [x] **画像メタデータのパージ** — `cursor::strip_png_metadata` + `build_cur_from_png` 内部での自動剥離
  - `image::DynamicImage` 経由で tEXt / iTXt / zTXt / eXIf 等の補助チャンクを破棄
  - 単体テストで tEXt 剥離を検証 (`test_strip_png_metadata_removes_text_chunk`)
- [x] リサイズ結果のキャッシュ（17 役割 × 6 サイズ = 102 枚の最適化、`RESIZE_CACHE`）
- [ ] 進捗表示・キャンセル機能（IPC ストリーム経由）

### 3-2: `.cur` バイナリ生成 ✅
- [x] ICO/CUR ヘッダー構造体定義
- [x] 6サイズ（32/48/64/96/128/256px）のマルチ解像度パッキング
- [x] ホットスポット座標の書き込みとスケーリング
- [x] PNG エンコーディングによるデータ圧縮

### 3-3: `.ani` パーサー
- [ ] RIFF/ACON 構造解析
- [ ] フレーム抽出 + JIFF タイミング情報取得
- [ ] プレビュー用 PNG / アニメーション情報の返却

### 3-4: ICO/CUR インポーター
- [ ] 複数解像度内蔵構造の解析
- [ ] 6サイズスロットへの自動マッピング

---

## Phase 4: システムトレイ & バックグラウンド 🔔 ✅ 基盤完了

### 4-1: トレイ常駐 ✅
- [x] システムトレイアイコン設定
- [x] トレイメニュー（設定 / パニックボタン / 終了）
- [x] ダブルクリックでウィンドウ表示
- [ ] WebView 破棄 & 再生成ロジック（メモリ最適化）

### 4-2: ダークモード監視 ✅ 完全貫通
- [x] `AppsUseLightTheme` レジストリ初期読込
- [x] `WM_SETTINGCHANGE` + `ImmersiveColorSet` メッセージ購読（不可視ウィンドウ）
- [x] コールバック経由のテーマ切替通知
- [x] **テーマ A/B 自動切替実行 (`main.rs` で配線)** — config の `dark_mode.enabled` を尊重、`light/dark_theme_id` を参照して `ThemeManager::apply_theme` 呼出 + `active_theme_id` 永続化

### 4-3: クラッシュリカバリ ✅
- [x] 起動時の pending スナップショット検出
- [x] 自動復旧ロジック

### 4-4: 多重起動防止 ✅ 実装完了
- [x] Named Mutex (`Local\CursorForge.SingleInstance.<UUID>`) による排他制御
- [x] `SingleInstanceLock` RAII 型 — drop 時に `ReleaseMutex` + `CloseHandle`
- [x] main 起動時に取得失敗 → `tracing::warn` + プロセス終了
- [ ] 既存インスタンスのトレイアイコンへのフォーカス移動 (CreateEvent シグナル方式、次回)

### 4-5: OS 起動時自動起動
- [ ] `HKCU\...\Run` へのレジストリ登録/解除
- [ ] MSIX 環境では `<Extension Category="windows.startupTask">` を使い分け

### 4-6: グローバルホットキー ⬅️ NEW
- [ ] `Ctrl+Alt+Shift+R` での強制リセット（Windows 既定カーソルに復旧）

### 4-7: Windows 11 統合・競合検出 ⬅️ NEW
- [ ] アクセシビリティ機能との競合検出（`CursorIndicator` / `ContrastScheme` / `CursorBaseSize`）
- [ ] 競合検出時の警告ダイアログ表示
- [x] **動作環境マトリクス — RDP / Citrix / Server を起動時検出して警告バナー**
  - `src-tauri/src/environment.rs` (`GetSystemMetrics(SM_REMOTESESSION)` + `InstallationType` レジストリ)
  - `get_environment_report` IPC + `EnvironmentBanner.vue` (sessionStorage で「閉じる」を記憶)

---

## Phase 5: Frontend UI 実装 🎭 ✅ 5-1〜5-9 完了（全画面の Hi-Fi 移植が完了）

> [!IMPORTANT]
> Hi-Fi デザインシステム（`design/` ディレクトリ）が完成済み。
> React JSX + Vanilla CSS で作成された 7 画面のプロトタイプを Nuxt 4 / Vue 3 に移植する。

### 5-0: デザインシステム概要（`design/` ディレクトリ）✅ 作成済み

デザインの **Source of Truth** となるファイル群:

| ファイル | 内容 | 移植先 |
|---|---|---|
| `styles.css` (955行) | デザイントークン + 全コンポーネントCSS | `app/assets/css/global.css` |
| `shell.jsx` | Titlebar / Sidebar / Statusbar / CursorMatrix | Vue コンポーネント群 |
| `icons.jsx` | 全SVGアイコンセット（17カーソル役割 + UI用） | `app/components/icons/` |
| `library.jsx` | 01 テーマライブラリ画面 | `app/pages/index.vue` |
| `screens.jsx` | 02 適用確認モーダル | `app/components/ApplyModal.vue` |
| `creator.jsx` | 03 クリエイターモード | `app/pages/creator.vue` |
| `screens.jsx` | 04 公式インデックス | `app/pages/marketplace.vue` |
| `general-settings.jsx` | 05 一般設定（8セクション） | `app/pages/settings.vue` |
| `settings.jsx` | 06 外観 / ダークモード連動ペアリング | `app/pages/appearance.vue` |
| `panic.jsx` | 07 パニック復旧フロー | `app/components/PanicFlow.vue` |
| `design-canvas.jsx` | デザインプレビュー用キャンバス | 移植不要（開発ツール用） |

### 5-1: デザイントークン移行 ✅ 完了
- [x] `design/styles.css` (954 行) を `app/assets/css/global.css` に完全移植
- [x] カラー / ボーダー / テキスト / シャドウ / 角丸 / フォントトークン
- [x] Win11 風ウィンドウクロム（`.win`, `.titlebar`, ambient gradient bloom）
- [x] ガラスモーフィズム `backdrop-filter: blur()` の統一
- [x] アニメーション（`@keyframes pulse`, `fade-in`, `slide-in-right`, `spin`）

### 5-2: 共通コンポーネント変換（JSX → Vue SFC） ✅ 完了
- [x] `AppTitlebar.vue` — Tauri ウィンドウ API 連携（最小化/最大化/閉じる）
- [x] `AppSidebar.vue` — Workspace/System ナビ + パニックボタン + セッション表示
- [x] `AppStatusbar.vue` — items 配列駆動の動的ステータスバー
- [x] `CursorMatrix.vue` — 17役割の 6×3 グリッド（filled/empty）
- [x] `ThemeCard.vue` — プレビュー + メタ + カバレッジ + アクション、`apply`/`toggleFavorite`/`showDetails` emit
- [x] `UiIcon.vue` / `CursorIcon.vue` — render function 方式（v-html 禁止に対応）
- [x] アイコンセット — `UI_ICONS` (25 種) + `CURSOR_ICONS` (17 種) を `.ts` 化
- [x] `app/types/theme.ts` — `ThemeCardData` 型を分離
- [x] `nuxt.config.ts` — `pathPrefix: false` でファイル名そのまま自動インポート

### 5-3: 画面 01 — テーマライブラリ（リデザイン） ✅ 完了 + IPC 接続
- [x] `design/library.jsx` 準拠 UI (ツールバー / ヘッダー / chips / ソート / グリッド / オーバーレイ / 空状態 / スケルトン)
- [x] `app/layouts/default.vue` — Win11 シェル + ルート連動ナビ
- [x] **`get_themes` IPC バインド** — Tauri 接続時は実テーマ、未接続時はデモ
- [x] **Tauri v2 ウィンドウ drag-drop イベント購読** — ブラウザ DragEvent ではなく `getCurrentWindow().onDragDropEvent` から実ファイルパスを取得
- [x] **「インポート」ボタン → `@tauri-apps/plugin-dialog` でファイル選択** → `import_cursorpack` IPC
- [x] **「新規作成」→ `/creator` への NuxtLink 遷移**
- [x] インポート失敗時のエラーバナー

### 5-4: 画面 02 — 適用確認モーダル ✅ 完了
- [x] `ApplyModal.vue` 作成 (KV リスト / 17 ミニカーソル行 / カバレッジバーペア / 署名フッター)
- [x] バックドロップクリックでキャンセル、busy 時はキャンセル抑止
- [x] 未署名テーマは赤字警告メッセージに切替
- [x] `useTauri` composable — `invokeTauri` IPC ラッパー (Web 開発時はフォールバック)
- [x] テーマ適用 E2E フロー: カードの「適用」→ ApplyModal → `invoke('apply_theme')` → Rust → レジストリ書き込み
- [x] `ThemeManager::apply_theme(id: Uuid)` — テーマディレクトリ走査 + cursor_paths 構築 + RegistryManager::apply_cursors
- [x] `RegistryManager::set_cursor_shadow` — `SPI_SETCURSORSHADOW` で `requires_os_shadow` を反映
- [x] エラー時はトースト風バナーで通知
- [x] Tauri コマンド `apply_theme(theme_id: String)` を `commands.rs` に登録
- [x] `cargo check` 成功

### 5-5: 画面 03 — クリエイターモード ✅ `.cursorpack` 出力まで貫通
- [x] `app/pages/creator.vue` — 3 カラムレイアウト + タブバー (割り当て/メタデータ/プレビュー/公開)
- [x] `RoleListItem.vue` / `SizeStrip.vue`
- [x] ビッグプレビュー — インポート画像があれば `<img>`、なければ役割アイコン
- [x] リサンプル切替 (Lanczos/Nearest/Auto) 含む btn-group
- [x] 右プロパティ: Hotspot 座標入力 / per-size トグル / アセット情報 / Validation パネル (実値反映)
- [x] Arrow 必須バリデーション表示
- [x] 画像インポート (PNG / SVG / Magic Byte / sanitize / 10MB 上限)
- [x] SVG → Canvas → PNG ラスタライズ
- [x] 単体ビルド (`build_cursor_file` IPC + `cursor::build_cur_from_png`)
- [x] **メタデータタブ** — 名前(ja/en) / 作者 / バージョン / 説明 / OS 影トグル + 割り当て状況サマリー
- [x] **`.cursorpack` 出力フロー** — 全ロールの一時 `.cur` を作成 → `export_cursorpack` IPC → 保存ダイアログ
- [x] **Ed25519 署名 & エクスポート** — keystore 状態に応じて「署名 & エクスポート」ボタンを切替表示
- [x] keystore 未登録時は「未署名」赤タグ + 通常エクスポートのみ可能
- [x] エクスポート結果バナー (バイト数 + 署名状況 + 保存先パス)

### 5-6: 画面 04 — 公式インデックス（マーケットプレイス） ✅ 完了
- [x] `app/pages/marketplace.vue` — `design/screens.jsx::CFMarketplace` を Vue 化
- [x] `FeaturedCard.vue` — 横並びレイアウト + ハイライトラベル + ダウンロードボタン
- [x] `MarketplaceCard.vue` — グリッドカード (署名済バッジ常時表示, downloadCount, インポートボタン)
- [x] chips フィルタ (All / Pixel / Minimal / Animated / Dark) + 検索 + GitHub リンク
- [x] `app/types/marketplace.ts` — `MarketplaceEntry` / `MarketplaceTag` 型契約
- [x] Rust 側 `src-tauri/src/marketplace.rs` 新設:
  - `MarketplaceIndex` / `MarketplaceEntry` / `MarketplaceInstallRequest` 型
  - `MarketplaceClient::fetch_index` / `install` (Phase 9 までスタブ)
- [x] Tauri コマンド: `marketplace_fetch_index` / `marketplace_install` を `commands.rs` に登録
- [x] `cargo check` 成功
- [x] **実 HTTP 取得 (rustls-tls) + SHA-256 + Ed25519 署名検証** — Phase 9 一部前倒し
- [x] 著者公開鍵レコード (`authors/{github}.json`) 取得 + key_id 一致確認 + ローテーション対応
- [x] サイズ上限 (50MB) 付きダウンロード (Zip 爆弾対策の入口)
- [ ] ZIP 展開を `~/.custom_cursors/<UUID>/` へ統合 (Phase 9 残タスク)

### 5-7: 画面 05 — 一般設定（8セクション） ✅ 完了 + IPC 接続
- [x] `app/pages/settings.vue` — 8 セクション切替 (一般 / 起動 / ライブラリ / セキュリティ / 鍵 / ログ / 更新 / About)
- [x] `SettingsRow.vue` / `SettingsToggle.vue` — 汎用行 + トグル v-model 互換
- [x] 各セクションにトグル / select / 数値入力 / ボタンの組み合わせを実装
- [x] `.cursorprofile` エクスポート / インポートボタン (UI のみ)
- [x] 鍵ペア管理セクション — 生成 / インポート / エクスポート / key_id 表示
- [x] ログセクション — レベル / 保持日数 / 上限 MB / フォルダー開く
- [x] About — ホームページ / Issues / OSS ライセンス
- [x] **`get_config` で初期化、`update_config` で永続化** (`useAppConfig` composable で全画面共有)
- [x] dirty フラグ + 「変更を破棄 / 保存」ボタンの活性制御 + 保存中スピナー
- [x] `app/types/config.ts` — Rust `AppConfig` と対応する型定義

### 5-8: 画面 06 — 外観 / ダークモード連動ペアリング ✅ 完了 + IPC 接続
- [x] `app/pages/appearance.vue` — OS 状態インスペクター + ペアリング + Detection
- [x] `ModeIndicator.vue` / `PairingSlot.vue`
- [x] Auto Switch 中央セパレーター + Detection 3 トグル
- [x] 起動時 `get_dark_mode_status` IPC で初期状態取得
- [x] **`useAppConfig` で `dark_mode.{enabled, light_theme_id, dark_theme_id}` を永続化**
- [x] **`ThemePickerModal.vue`** — change ボタン → モーダルでテーマ選択 (検索 + クリア)
- [x] dirty / 保存 / 変更を破棄 ボタン制御
- [x] `useThemes` composable — テーマ一覧の共有リアクティブシングルトン

### 5-9: 画面 07 — パニック復旧フロー ✅ 完了
- [x] `PanicFlow.vue` — 全画面オーバーレイ実装 (idle → running → done/error)
- [x] 2 段階ステージ選択カード (Stage 1: Windows 既定 / Stage 2: 初回スナップショット)
- [x] ライブログ表示 + プログレスバー + 17 ロールグリッド (done/running/pending 色分け)
- [x] 残時間推定 + auto-rollback armed 表示
- [x] グローバルホットキー `Ctrl+Alt+Shift+R` を `default.vue` で購読
- [x] サイドバーのパニックボタン押下でも起動
- [x] 実 IPC: `reset_to_default` / `reset_to_initial` を呼び出し

### 5-10: IPC コマンド定義 ✅
- [x] 全 21 エンドポイント定義済み:
  - `get_cursor_roles` / `get_current_cursors` / `get_themes` / `apply_theme`
  - `inspect_cursorpack` / `import_cursorpack` / `build_cursor_file` / **`export_cursorpack`**
  - `export_profile` / `import_profile`
  - **`keystore_info` / `keystore_generate` / `keystore_delete`**
  - `marketplace_fetch_index` / `marketplace_install`
  - `reset_to_default` / `reset_to_initial`
  - `get_dark_mode_status` / `get_config` / `update_config` / `get_app_info`
- [x] `apply_theme` は config の `active_theme_id` を成功時に永続化
- [x] `get_themes` は `is_active` を `active_theme_id` から判定して返却
- [x] `import_cursorpack` / `marketplace_install` は展開後の UUID を文字列で返却
- [x] `import_profile` は `ProfileEnvelope` を返却
- [x] `inspect_cursorpack` は theme.json のみ読んで既存テーマとの衝突情報を返却
- [x] `build_cursor_file` は PNG → 6 サイズ .cur をビルドして指定パスへ書き出し、書込みバイト数を返却

### 5-11: アクセシビリティ
- [ ] WCAG AA 準拠（コントラスト比 4.5:1 以上）
- [ ] キーボードナビゲーション対応
- [ ] スクリーンリーダー対応（ARIA ラベル）

---

## Phase 6: テーマパッケージ & セキュリティ 📦 🔄 解凍 + 多層防御 完了

### 6-1: `.cursorpack` 形式 ✅ 完了
- [x] `theme.json` スキーマ定義（多言語対応含む）
- [x] **Zip 解凍 (`ThemeManager::import_cursorpack_bytes` / `import_cursorpack_file`)**
- [x] バリデーション（スキーマパース、ID/cursors マップ）
- [x] **同名テーマ再インポート時のバージョン比較ダイアログ** — `inspect_cursorpack` IPC + `ImportConflictDialog.vue` で newer/older/same を判定
- [x] **パッケージ「作成」** (`ThemeManager::export_cursorpack` + `export_cursorpack` IPC)
- [x] 自動 UUID 採番 + theme.json 自動構築 (cursors[role].file = `cursors/<role>.cur` に正規化)
- [x] 署名オプション (`sign: true` で id|version|sorted_roles の SHA-256 hex を Ed25519 署名)

### 6-2: セキュリティ多層防御 ✅ 完了
- [x] **Magic Byte 検証** (theme.json 先頭 `{` チェック + クリエイターで PNG ヘッダー検証)
- [x] **パストラバーサル対策** (`sanitize_archive_path` ヘルパー + 単体テスト 4 件)
- [x] **Zip 爆弾対策** (圧縮 50MB / 展開後 200MB / 個別 10MB の三段階逐次チェック)
- [x] **シンボリックリンク攻撃拒否** (`unix_mode` の `S_IFLNK` で弾く)
- [x] レジストリ・インジェクション防止（テーマ名サニタイズ）
- [x] **SVG サニタイズ** (`composables/sanitizeSvg.ts` 自前実装) — `<script>`/`<foreignObject>`/`href`/`on*`/`javascript:`/`data:` を除去 + 削除ログ表示

### 6-2a: セキュリティ閾値（具体数値） ⬅️ NEW

> [!IMPORTANT]
> 仕様書で定義済みの閾値を計画に明記

| 対象 | 上限値 |
|---|---|
| `.cursorpack` ファイル | 50 MB |
| 展開後合計 | 200 MB |
| 個別画像ファイル | 10 MB |
| 全テーマ合計（`~/.custom_cursors/`） | 1 GB（警告ダイアログ） |

### 6-3: Ed25519 署名 ✅ コア完成
- [x] **鍵ペア生成** (`Keystore::generate`) — OsRng で 32 バイト乱数、SigningKey/VerifyingKey 構築
- [x] **DPAPI 暗号化保存** (`CryptProtectData` / `CryptUnprotectData`) — 秘密鍵はユーザーアカウント紐付き
- [x] **テーマ署名 / 検証** — `Keystore::sign` / `Keystore::verify` + `export_cursorpack` で `signature` 埋め込み
- [x] **`key_id` 仕様** — 公開鍵 SHA-256 の先頭 16 文字 (`compute_key_id`)
- [x] 設定画面の鍵管理 UI (`useKeystore` composable + 生成/再生成/削除/key_id 表示)
- [x] **未署名テーマの警告ダイアログ** — `ApplyModal` で赤字メッセージ
- [x] **秘密鍵のエクスポート / インポート (.cfkey 形式)** — XChaCha20-Poly1305 + Argon2id (m=64MiB, t=3, p=1)
  - フォーマット: `CFKEY1\n\0` magic + 16 byte salt + 24 byte nonce + ciphertext
  - `keystore_export(passphrase, path)` / `keystore_import(passphrase, path)` IPC
  - `PassphrasePrompt.vue` (8 文字以上の弱バリデーション + 確認入力)
- [ ] 鍵ローテーション（公開鍵差し替え PR、過去 `key_id` 検証維持）

### 6-4: `.cursorprofile` フルバックアップ ✅ 完了
- [x] `.cursorprofile` 形式設計 — Zip 構造 (`profile.json` + `cursors/<UUID>/`)
- [x] `.cursorpack` との使い分け明確化（テーマ単体 vs 環境まるごと）
- [x] `BackupManager::export` / `import(path, merge)` 実装 (`src-tauri/src/backup.rs`)
- [x] `ProfileEnvelope` スキーマ (schema_version + exported_at + app_version + config スナップショット)
- [x] インポート時の path traversal / シンボリックリンク / サイズ防御 (theme.rs と共通の `sanitize_archive_path_pub` 再利用)
- [x] Tauri コマンド `export_profile(path)` / `import_profile(path, merge)` を IPC 公開
- [x] `settings.vue` の「.cursorprofile バックアップ」セクションに `@tauri-apps/plugin-dialog` 経由のファイル保存/選択を配線
- [x] インポート時 `ask` ダイアログで「上書き or マージ」を確認

---

## Phase 7: 通知・ロギング・国際化 📋 🔲 未着手

### 7-1: ロギング ✅ PII フィルタ適用完了
- [x] `tracing` + `tracing-subscriber` 導入
- [x] **`tracing-appender` 日次ローテーション** (`%LOCALAPPDATA%\CursorForge\logs\app-YYYY-MM-DD.log`)
- [x] **14 日経過ログの自動削除** + **合計 100MB 上限の古い順削除** (`logging::cleanup_old_logs`)
- [x] PII 除外ヘルパー: `logging::redact_path` (ホーム下を `~/...` に置換) + `logging::short_hash` (SHA-256 12 文字短縮)
- [x] `WorkerGuard` を main で保持して非同期書き出しの flush を保証
- [x] デバッグビルドは標準出力にも色付きで出力
- [x] **既存 `tracing!` のパス出力箇所に `redact_path` 適用** (theme/backup/commands/config)
- [x] **Marketplace の URL ログを `short_hash` で短縮**してフィッシング先追跡を防ぐ
- [ ] クラッシュレポート オプトイン送信 (Phase 7-2 と統合)

### 7-2: 通知システム ✅ 実装完了 (層 1/2/3 すべて)
- [x] 3 層通知の設計と振り分け:
  - **層 1 無通知**: バックグラウンドの自動適用成功 (ダークモード自動切替も無通知)
  - **層 2 Toast**: テーマ適用完了 / インポート成功 — `tauri-plugin-notification` 経由
  - **層 3 モーダル**: パニックリセット確認 (`PanicFlow`) / 署名検証失敗 / 衝突検出 (`ImportConflictDialog`)
- [x] `tauri-plugin-notification` を Cargo.toml + main.rs プラグインに追加
- [x] `@tauri-apps/plugin-notification` フロント JS パッケージ追加
- [x] `useNotify` composable — 起動時に `requestPermission` を一度だけ実行してキャッシュ
- [x] capabilities/default.json に notification 権限追加
- [x] ライブラリのテーマ適用成功 / インポート成功で Toast 表示
- [x] **`AppUserModelID` の明示的登録** (`src-tauri/src/appusermodel.rs`)
  - `SetCurrentProcessExplicitAppUserModelID("dev.cursorforge.app")` を main 起動時に呼出
  - tauri.conf.json の identifier と整合 (Vendor.Product 形式)
  - 通知センターで送信元アプリ名が CursorForge と表示される

### 7-3: 国際化 🔄 基盤完了
- [x] **i18n キー管理 (`app/locales/ja.ts` + `en.ts`)** — `as const` で型導出
- [x] **`useI18n` composable** — `t(key, params)` + プレースホルダ展開 + フォールバック
- [x] OS ロケール自動判定 (`navigator.language` 経由) + `general.language` 設定での上書き
- [x] `default.vue` で起動時に config 同期 + watch で動的更新
- [x] サイドバーをサンプルとして i18n 配線
- [x] `theme.json` の多言語フィールド対応（`LocalizedString`）
- [x] **全画面 + 全モーダルの文字列を `t()` 化** (177 キー / ja-en parity)
- [x] 通知メッセージ (`library.notifyApplied` / `notifyImported`) も多言語化
- [x] CI で `scripts/check-i18n.mjs` がキー差分を検出 (現在 ja=en=177)
- [x] PanicFlow / ImportConflictDialog も完了
- [ ] 残: 設定画面の各セクション本文 (現状は dynamic な短い説明のみ)

---

## Phase 8: リリース準備 & 配布 🚀 🔲 未着手

### 8-1: パフォーマンス 🔄 雛形整備
- [x] **`.github/workflows/performance.yml`** — release ビルドのバイナリサイズ計測 + 30MB 警告
- [x] **`.github/workflows/ci.yml`** — `cargo fmt --check` + `clippy -D warnings` + `cargo test` + `vue-tsc --noEmit` + i18n キー差分
- [x] **`.github/workflows/marketplace-validate.yml`** — Phase 9 連携用雛形

> [!IMPORTANT]
> CI 自動測定の目標値と回帰検出

| 指標 | 目標 |
|---|---|
| 常駐メモリ | ≤ 15 MB |
| 起動時間 | ≤ 1.5 秒 |
| テーマ適用 | ≤ 3 秒 |
| `.msi` パッケージサイズ | ≤ 30 MB |

- [x] **criterion マイクロベンチ導入** (`src-tauri/benches/cursor_build.rs`)
  - 64x64 / 256x256 の Lanczos リサイズ + 6 サイズ .cur ビルドを計測
  - 32x32 の Nearest 経路もベンチ
  - performance.yml が `cargo bench --bench cursor_build` を実行し、bencher 形式で出力 → artifact 保存
- [ ] 起動時間 / メモリ使用量の actual 測定
- [x] **リサイズ結果のキャッシュ** (`cursor.rs::RESIZE_CACHE`)
  - キー: (元画像 SHA-256 12 文字, target_size, ResizeMethod) / 値: RgbaImage
  - 容量 64 エントリの単純 FIFO 削除
  - `clear_resize_cache()` + `clear_cursor_cache` IPC
  - ベンチに cold/warm パスを追加して効果計測可能に

### 8-2: インストーラー & 署名
- [ ] `.msi` インストーラー生成（x64）
- [ ] ARM64 ビルドの独立した扱い
- [ ] WebView2 Evergreen Bootstrapper の `.msi` 同梱戦略
- [x] **NSIS バンドルターゲット追加** (perUser インストール、ja/en 言語セレクタ)
- [x] **MSI バンドル設定強化** (publisher / homepage / wix language)
- [ ] WebView2 Evergreen Bootstrapper の `.msi` 同梱戦略
- [ ] EV/OV コードサイニング調達方針（SignPath.io 等の OSS 無償署名）
- [ ] SmartScreen レピュテーション獲得の検証

### 8-3: MSIX / Microsoft Store 対応 🔄 雛形完成
- [x] **`runFullTrust` capability の設定** ([distribution/msix/AppxManifest.xml](distribution/msix/AppxManifest.xml))
- [x] **Packaged Win32 App 構成** (`Windows.FullTrustApplication` EntryPoint)
- [x] **`<Extension Category="windows.startupTask">`** による自動起動宣言 (Enabled=false 初期値)
- [x] [docs/distribution.md](docs/distribution.md) に変換手順を整備
  - MSIX Packaging Tool (GUI) 経由 / `makeappx` + `signtool` (CLI) 経由
  - SignPath.io / Microsoft Trusted Signing 比較
  - SmartScreen 緩和策
- [ ] Microsoft Store 申請審査 (`runFullTrust` は Restricted Capability)

### 8-4: 自動アップデート 🔄 基盤完了
- [x] **Tauri Updater 設定** (`tauri-plugin-updater` + `tauri-plugin-process`)
  - tauri.conf.json: GitHub Releases の `latest.json` をエンドポイント指定
  - capability に updater:* / process:allow-restart 追加
  - dialog: false で UI 側に進捗表示を委譲
- [x] `useUpdater` composable — check / downloadAndInstall / relaunch を提供
- [x] 設定画面のアップデートセクションに実機能配線
  - 「更新を確認」ボタン → 利用可能バージョンを表示
  - 「ダウンロード & インストール」ボタン → 進捗 % 表示 → 完了後 ask ダイアログで再起動
- [ ] 公開鍵 (`pubkey`) の発行 — `tauri signer generate` でリリース署名鍵を生成
- [ ] メジャーバージョン跨ぎ (v1 → v2) は自動更新しない方針 (現状はチェック側で判定なし)
- [x] **3 回連続起動失敗の検出機構** — `src-tauri/src/health.rs`
  - `%LOCALAPPDATA%\CursorForge\state\startup.json` に `pending_failures` を保存
  - main 起動時に `StartupCheck::begin` でインクリメント
  - setup 完了後に `mark_healthy()` で 0 リセット
  - panic/クラッシュで mark_healthy に到達しなければカウンタが残る
  - バージョン変更を検知したらカウンタを自動リセット
  - 単体テスト 2 件 PASS
- [ ] 検出時の Tauri Updater 旧バージョン取得 + 自動置換 (現状はカウンタ管理 + 警告ログのみ)

### 8-5: CI/CD & ドキュメント
- [ ] CI/CD パイプライン構築
- [x] README / LICENSE 整備
- [x] v1.0 の既知制約を README に明記 (README.md + README.ja.md):
  - `.ani` 新規生成不可
  - ライブプレビューなし
  - Undo なし
  - 自動切替はダークモード連動のみ
  - UAC Secure Desktop / ロック画面 / マルチユーザーの制限
  - RDP 環境は動作対象外

---

## Phase 9: 公式インデックス連携 🌐 🔄 着手中（クライアント実装の半分完了）

### 9-1: GitHub メタデータインデックス ✅ 設計完了
- [x] テーマメタデータスキーマ Rust 側型定義 (`MarketplaceIndex` / `MarketplaceEntry` / `AuthorRecord`)
- [x] `index.json` URL 定義 (`raw.githubusercontent.com/cursorforge/index/main/index.json`)
- [ ] 公式リポジトリの実体構築 (`cursorforge/index` リポジトリ作成)

### 9-2: テーマ提出フロー
- [ ] ブラウザ経由の PR 提出フロー（事前 URL パラメータでテンプレ埋め）
- [ ] アプリ内の「公式インデックスに提出」ボタン

### 9-3: クライアント側検証 ✅ 実装完了
- [x] `reqwest` (rustls-tls) ベースの HTTPS 取得
- [x] SHA-256 整合性チェック (`sha2`)
- [x] Ed25519 署名検証 (`ed25519-dalek`)
- [x] Base64 デコード (`base64`)
- [x] サイズ上限 (50MB) 付きダウンロード = Zip 爆弾対策の入口
- [x] 著者公開鍵レコード取得 + key_id 一致確認
- [x] 鍵ローテーション対応 (`historical_keys` マップで過去 key_id を検証可能)
- [x] **ZIP 展開を `~/.custom_cursors/<UUID>/` へ統合** (`ThemeManager::import_cursorpack_bytes`)
- [x] パストラバーサル / Zip 爆弾 / シンボリックリンク防御をローカル経路と共有

### 9-4: CI 自動検証（GitHub Actions） ✅ 主要検証実装完了
- [x] **`.github/workflows/marketplace-validate.yml`** — `cursorforge/index` 用ワークフロー雛形
- [x] **`scripts/marketplace/validate.mjs`** 本体実装:
  - JSON スキーマ検証 (必須フィールド + UUID + SHA-256 hex 形式 + Arrow ロール必須)
  - **SHA-256 整合性照合** — `themes/<id>.cursorpack` ファイルと entry.sha256 を比較
  - **Ed25519 署名検証** — Node.js `crypto.verify` + Ed25519 SPKI DER 構築 + 公開鍵レコード参照
  - **key_id 一致確認** + ローテーション対応 (`historical_keys` から過去鍵参照)
  - **サイズ閾値チェック** (50MB)
  - **マルウェアハッシュ DB 照合** (`scripts/marketplace/malware-hashes.txt` の SHA-256 リスト)
- [ ] VirusTotal API 経由のリアルタイムマルウェアスキャン (将来)

### 9-5: 公開鍵管理
- [x] Rust 側で `authors/{github_username}.json` の取得 / パース実装
- [ ] PR ベースでの公開鍵登録ガイドの整備

---

## 既知の問題 / 制限事項

> [!WARNING]
> - Nuxt 4.4.4 では `ssr: false` が IPC エラーを引き起こすため `routeRules` で回避
> - `npm run dev` 実行時に `Set-Location` が必要なケースあり
> - `zip` クレート v2.6.x は yanked（v2 で範囲指定）

---

## 次回セッションでの優先タスク

> [!NOTE]
> 直近 6 セッションで主要機能 (UI 7 画面 / Marketplace / 鍵管理 / .cursorpack 入出力 /
> ダーク自動切替 / 多重起動防止 / 通知 / 環境検出) はすべて完了。
> ここからは仕上げと「v1.0 リリース DOD」に向けた残タスク。

1. ~~**📦 MSIX / `runFullTrust` capability**~~ ✅ 完了 (Phase 8-3 — AppxManifest.xml + distribution.md)
2. **✍️ EV/OV コードサイニング調達** — 調達ガイド作成済 ([docs/code_signing.md](docs/code_signing.md))。実申請は外部待ち
3. ~~**🪪 Tauri Updater 公開鍵発行**~~ ✅ 完了 (`tauri signer generate` + tauri.conf.json 更新 + release.yml + docs/signing.md)
4. ~~**♿ WCAG AA 検証**~~ ✅ 完了 (ARIA / aria-current / aria-labelledby / aria-pressed / skip-to-content / prefers-reduced-motion)
5. ~~**🛡️ VirusTotal API 統合**~~ ✅ 完了 (validate.mjs を VT API v3 対応、429/ネットワーク障害は fail-open)
6. ~~**🧪 起動時間 / メモリ / 適用時間の実測ベンチ**~~ ✅ 完了 (benches/startup.rs + performance.yml に闾値検証)
7. ~~**🪝 鍵ローテーション PR テンプレ**~~ ✅ 完了 (docs/key_rotation.md + .github/PULL_REQUEST_TEMPLATE/key_rotation.md)
8. ~~**🚀 v1→v2 メジャー跨ぎ判定 + 3 回連続失敗ロールバック**~~ ✅ 完了 (health.rs + main.rs Win32 ダイアログ + settings.vue 追加確認)
9. ~~**🦠 SVG 以外の画像メタデータパージ**~~ ✅ 完了 (eXIf / iTXt / zTXt チャンク除去テスト追加 16 pass)
10. ~~**🌐 設定セクション本文の i18n 残置換**~~ ✅ 完了 (288 キー / 8 セクション全文 + ダイアログ + ステータス)
11. ~~**🆘 GUI 復旧フロー**~~ ✅ 完了 (`list_config_backups` / `restore_config_backup` + ConfigRecoveryPanel.vue)

---

## 優先度の高いギャップ（要注意領域）

> [!CAUTION]
> 以下の領域は仕様書に記載があるが、まだ実装未着手 / 部分的のため重点的に扱う。

1. **Phase 8-2/8-3: 配布基盤** — `.msi` / `.msix` インストーラー生成、WebView2 Bootstrapper 同梱、SmartScreen レピュテーション獲得
2. ~~**Phase 8-4: 自動アップデート**~~ ✅ 完了 — メジャー跨ぎ警告 + 3 回連続失敗ロールバック誘導 (GitHub Releases ブラウザ誘導)
3. ~~**Phase 9-2: テーマ提出フロー**~~ ✅ 完了 — SubmitThemeDialog.vue + open_url IPC + GitHub ファイルエディタ自動 URL 生成
4. ~~**Phase 9-3: マルウェアハッシュ実 DB**~~ ✅ 完了 — VirusTotal API v3 統合 + fail-open
5. **Phase 7-2: AppUserModelID 登録** — トースト通知発信元の明示
6. ~~**Phase 4-7 残: アクセシビリティ競合検出**~~ ✅ 完了 (accessibility.rs + ApplyModal.vue 警告バナー)
7. **Phase 5-11: WCAG AA 準拠** — コントラスト 4.5:1 検証 / キーボードナビ / ARIA ラベル
8. ~~**Phase 2-1 残: ユーザー向けの復旧 UI**~~ ✅ 完了 — `ConfigRecoveryPanel.vue` が設定 General セクションにバックアップ一覧 + 復旧ボタンを提供
9. **Phase 6-1 残: `.cursorpack` 内画像メタデータパージ強化** — Exif / トラッキングデータの除去
10. ~~**README ja/en 整備**~~ ✅ 完了 — README.md (英語) + README.ja.md (日本語)、v1.0 既知制約・RDP 警告・セキュリティモデル・サブミットフロー明記

### 完了済み主要マイルストーン (記録)

| Phase | 状態 |
|---|---|
| Phase 1-4 | ✅ プロジェクト基盤 / Rust コア / 画像 + .cur / トレイ + ダーク / 多重起動 / 環境検出 |
| Phase 5 (UI) | ✅ 7 画面 + i18n 主要 4 画面 + 17 コンポーネント |
| Phase 6-1/6-2 | ✅ .cursorpack 入出力 / 多層セキュリティ防御 (path traversal / Zip 爆弾 / symlink / Magic Byte / SVG sanitize) |
| Phase 6-3 | ✅ Ed25519 鍵管理 (DPAPI + パスフレーズ export/import) |
| Phase 6-4 | ✅ .cursorprofile フルバックアップ |
| Phase 7-1 | ✅ ロギング (日次ローテ / 14 日保持 / 100MB 上限 / PII redaction + URL short_hash) |
| Phase 7-2 | ✅ 通知 3 層配線 + AppUserModelID 登録 |
| Phase 7-3 | ✅ i18n 基盤 + 全画面配線 (275 キー / ja-en parity / 設定 8 セクション全文配線) |
| Phase 8-1 | 🔄 CI 雛形 + criterion ベンチ + リサイズキャッシュ (実測値の数値目標検証は残) |
| Phase 8-4 | 🔄 Tauri Updater 配線完了 (公開鍵発行 + ロールバック検出は残) |
| Phase 9-1/9-3 | ✅ クライアント側検証 (HTTPS / SHA-256 / Ed25519 / ZIP 展開) |
| Phase 9-4 | ✅ CI ワークフロー + scripts/marketplace/validate.mjs |
