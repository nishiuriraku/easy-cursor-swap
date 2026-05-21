# Changelog

All notable changes to EasyCursorSwap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- マウスポインターのサイズを本アプリから変更できるように。設定 → 一般 → 「カーソルサイズ」セクションに 1〜15 のスライダーを追加し、Windows 設定アプリ「アクセシビリティ → マウスポインターとタッチ → サイズ」と等価な書き換え (`HKCU\Control Panel\Cursors\CursorBaseSize` の DWORD 32〜256) を行う。スライダー位置 → DWORD の変換は Rust 側 (`registry::slider_position_to_base_size`) が single source of truth で、UI からは `set_cursor_base_size` IPC 経由で即時反映 (テーマ適用とは独立した OS 全体設定として扱う)。書き込み後は `SystemParametersInfoW(SPI_SETCURSORS)` でシェルに通知し、`set_cursor_shadow` と同じ broadcast 偽陽性ハンドリングで `CursorBaseSize` 拡大時のシェルキャッシュ再構築由来の `ERROR_INVALID_HANDLE` を吸収する。

### Changed

- 共通 UI コンポーネント `app/components/ui/` を整備し、フロントエンド全体のモーダル / アラート / ボタン経路を統一: `UiModal` (Teleport + focus trap + body scroll lock を `useModalLifecycle` + `useFocusTrap` で内包), `UiConfirmDialog` (cancel/confirm 専用ラッパ), `UiAlert` (info/success/warn/danger インラインバナー), `UiButton` (`.btn` の Vue ラッパ + loading/icon ハンドリング) の 4 SFC + composable `useFocusTrap.ts` を追加。これに伴い `ApplyModal` / `ImportConflictDialog` / `DiscardEditDialog` / `NewThemeStartModal` / `SaveDestinationModal` / `BulkImportPreviewModal` / `ThemeDetailModal` / `ThemePickerModal` / `MarketplaceDetailModal` / `SubmitThemeDialog` / `SubmitDeviceFlowModal` / `OssLicenseModal` / `PassphrasePrompt` の 11 モーダルが `UiModal` shell に統一され、`.a11y-banner` (ApplyModal) / `ConfigRecoveryPanel` / `KeysSection` / `UpdatesSection` / `SubmitThemeAutoForm` / `SubmitThemeManualForm` / `SubmitDeviceFlowModal` の警告ボックスが `UiAlert` 経由になった。これにより 11 モーダルすべてに focus trap が初導入され、Esc / backdrop / Tab 循環の挙動が完全に一貫。さらに smoke test で判明した `ThemeDetailModal` の二重フッター UX 問題 (body 内アクション行 + .modal-foot の Close ボタンが重複) を解消し、`ThemeDetailDrawerFooter.vue` を削除して footer アクションを UiModal の `#leftNote` / `#actions` slot に直接配置。`components_total` 57 → 60 (`ui` 1 → 5、`library` 15 → 14)、`composables` 35 → 36。
- `accessibility::AccessibilityConflicts::has_conflicts` の判定から `CursorBaseSize > 32` を除外。本アプリの正規機能になったため「競合」ではなく「現状値」として扱う。値自体は UI スライダーの初期値反映のために `cursor_base_size` フィールドで引き続き返す。これに連動して `ApplyModal` の `conflictCursorBaseSize` 警告も非表示化し、未使用の `apply.conflictCursorBaseSize` i18n キーは両言語から削除した (ja/en 各 1 件減)。

### Fixed

- カーソルサイズスライダーを動かしても、現在表示中のカーソルが視覚的にリサイズされない不具合を修正。原因は `SystemParametersInfoW(SPI_SETCURSORS)` が `HKCU\Control Panel\Cursors\CursorBaseSize` の DWORD 変更を実行時に再評価しないため、現在キャッシュされているカーソルがそのまま残ること。修正として Windows 設定アプリの「マウスポインターとタッチ」スライダーが内部で行っているのと同じ経路 — `LoadImageW(file, IMAGE_CURSOR, target_size, target_size, LR_LOADFROMFILE)` で明示サイズのカーソルを生成 → `SetSystemCursor(hcursor, OCR_*)` で kernel の cursor table を直接差し替え — を `registry::RegistryManager::apply_system_cursors_at_size` として実装し、`set_cursor_base_size` から DWORD 書込・`SPI_SETCURSORS` の後に呼ぶようにした。これにより全アプリ・全 HDC で即時にカーソルが指定サイズに切り替わる。`OCR_*` 定数が存在する 14 種の標準役割 (Arrow / Help / AppStarting / Wait / Crosshair / IBeam / No / Size×6 / UpArrow / Hand) が対象で、`NWPen` / `Pin` / `Person` の 3 種は `OCR_*` がないため即時反映対象外 (DWORD 永続化のみ、次回テーマ適用 / ログオン時に反映)。レジストリ書込は HKCU 限定のまま、UAC 不要。
- 上記カーソルサイズ反映機構が、新規実装後も依然として「DWORD 書込は成功するが `read_back=None` / `SetSystemCursor 適用: 0/14`」となり視覚反映しない不具合を再修正。真の根本原因は `set_cursor_base_size` 内で `HKCU\Control Panel\Cursors` を `KEY_WRITE` 単独で開いていたこと — Win32 レジストリ API のアクセス権は読/書独立で、`set_value` は通るが同じハンドルでの `get_value` / `get_raw_value` が `PermissionDenied` で静かに失敗し、続く read-back 検証と `apply_system_cursors_at_size` 内の役割パス取得がすべて空振りしていた。`open_subkey_with_flags("Control Panel\\Cursors", KEY_READ | KEY_WRITE)` に変更して両方の操作を1つのハンドルで通せるようにし、これで DWORD 書込・read-back・14 役割の `LoadImageW`+`SetSystemCursor` 一括差し替えが全て成立する。HKCU 限定 / UAC 不要 / トランザクション独立は維持。
- Windows のアクセシビリティ設定でカーソルサイズを変更してから本アプリを起動したとき、設定 → 一般 → カーソルサイズスライダーが Windows 側の現在値を反映せず常に最小 (slider 1) を初期表示してしまう不具合を修正。原因は `accessibility::read_cursor_base_size` が `HKCU\Control Panel\Cursors\CursorBaseSize` のみを読んでいたこと — Windows 11 Settings の「マウスポインターとタッチ」スライダーは `HKCU\SOFTWARE\Microsoft\Accessibility\CursorSize` (slider 1-15) を canonical な書込先として扱い、`CursorBaseSize` は Windows ビルドによっては未書込のままになるため、Settings UI と本アプリの読込先がズレていた。修正として `accessibility::resolve_cursor_base_size` 純粋関数を切り出し、(1) `Accessibility\CursorSize` を slider→DWORD 変換、(2) `CursorBaseSize` を fallback、(3) 両方未設定なら 32 という優先順位で解決する形にした。合わせてフロントエンド (`pages/settings.vue`) にウィンドウフォーカス / `visibilitychange` イベントの購読を追加し、Windows 側でスライダーを動かしたあと本アプリへ戻ってきたタイミングでもスライダーが自動同期するようにした。

## [0.0.3] - 2026-05-20 (pre-release)

v0.0.1 / v0.0.2 と同じく仮リリース系列 (provisional, SemVer 0.0.x で API 安定保証なし)。Creator / Marketplace 周りの UX 改善、Library のビジュアル不具合修正、開発体験向上、そして大規模な依存関係リフレッシュ (npm 8 件 + Rust 5 メジャー + ロックファイル 31 件) が中心。HKCU 限定 / 適用トランザクション性 / アーカイブ検閲 / PII レダクション / `v-html` 不採用 の 5 大不変条件はすべて維持。

### Added

- Creator の編集破棄ガード: 編集中に「クリア」ボタンを押したときや、未保存の状態で別ページ・別テーマへ遷移しようとしたとき、`DiscardEditDialog` を出して破棄を確認するように変更。直前まで作りかけていた cursor 配置が誤操作で吹き飛ぶ事故を防止。
- Creator の保存先選択モーダル (`SaveDestinationModal`) で、デフォルトの保存先を「Library に保存」に変更し、keystore が登録済みのときは「署名する」チェックを自動 ON に。新規ユーザーが特に何も意識しなくても、署名済みテーマがそのまま Library に取り込まれる導線になった。
- 開発者体験: `npm run tauri:dev` で WebView2 DevTools が自動的に有効化されるようになり、`devtools()` の `cfg!(debug_assertions)` ゲート付きで開発時のみ起動。`Cargo.toml` 側で `tauri` の `devtools` feature をデバッグビルド限定に切り出し、リリースビルドのバイナリサイズと攻撃面には影響しないよう構成。

### Changed

- マーケットプレイス「公式インデックスに提出」モーダルのタグ入力を、自由入力 chip 方式から allow-list の toggle chip 方式に変更。`easy-cursor-swap-index/schemas/index-entry.json` の enum (`pixel`, `minimal`, `animated`, `dark`, `light`, `anime`, `retro`, `neon`) に固定し、Auto / Manual 両タブから 8 個の chip をクリックでトグルする UI に。タイポによる Ajv バリデーション失敗 PR を未然に防ぐ。`app/types/marketplace.ts` に `ALLOWED_MARKETPLACE_TAGS` 定数を追加し、専用 composable だった `useTagChipInput.ts` は呼出元が 1 箇所のため削除して `SubmitThemeDialog.vue` に inline 化 (composables 36 → 35)。
- Creator のリサンプリング選択肢から「auto」を削除。実装上は常に Lanczos3 にフォールバックしていた dead option で、UI に出すことでユーザーに誤った期待を与えていたため整理。残る選択肢は `nearest` / `lanczos3` のみとなり、cursor 解像度に応じた手動選択ロジックに一本化。
- マーケットプレイスからダウンロード済みのテーマを、再びマーケットプレイスへ提出することを Creator 側で抑止。`source/kind === "marketplace"` のテーマを「複製」「提出」両ダイアログのドロップダウンから除外し、原作者の credit を保ったまま二次配布される事故を防ぐ。
- マーケットプレイスのテーマ詳細・カードからダウンロード回数の表示を撤去。実カウントを集計する配信側 API がまだ存在せず、フロントが常に `0` を表示してしまう状態だったための整理 (`FeaturedCard.vue` / `MarketplaceDetailModal.vue`)。

### Fixed

- Library の空状態 (まだテーマが 1 つも無いとき) で、グラデーション背景が card 領域だけに収まり、その下の `LibraryToolbar` 周辺が地の `bg-surface` で抜けて視覚的に不連続だった問題を修正。`LibraryEmptyState` のラッパーを `min-h-full grid place-items-center` 構成に変更し、空状態時のメインコンテンツ高さをツールバー下端まで満たすようにした。`LibraryEmptyState.test.ts` / `LibraryToolbar.test.ts` のスナップショット / DOM 構造期待値も追従。
- Library のテーマカード上で、テーマに紐づく外部リンク (作者プロフィール / 元配布元など) にカーソルを乗せると、Tauri WebView の左下に URL がポップアップ表示されてしまう問題を修正。内部用リンク (`href="#"` ベースで `openExternalUrl` 経由で起動するもの) には `href="javascript:void(0)"` を割り当てず、`a` 要素そのものを `button[role=link]` 代替に置き換えることで preview を抑止。マウスホバー時のスクリーンスペースが本来のテーマ名やタグの可読性で埋まるようにした。
- リポジトリ内に残っていた旧プロジェクト名 `cursor-forge` を `easy-cursor-swap` に統一。実害は限定的だったが、再 brand 後の README / ドキュメント / コメント中に紛れていたものを一括置換し、外部ユーザーから見える文字列の整合を確保。
- ドキュメント生成系の補助で wiki-staging ディレクトリが誤ってコミットされる可能性があったため `.gitignore` に追加。

### Internal

- **依存関係の包括的リフレッシュ** (PR #3 で実施)。npm 8 件 + Rust 5 メジャー + ロックファイル 31 件:
  - **npm** (8 件, vulnerability 0):
    - `@tauri-apps/api` 2.6 → 2.11
    - `@tauri-apps/cli` 2.6 → 2.11
    - `nuxt` 4.4.4 → 4.4.6
    - `vue` 3.5.33 → 3.5.34
    - `vue-router` 5.0.6 → 5.0.7
    - `@vitejs/plugin-vue` 6.0.6 → 6.0.7
    - `vitest` / `@vitest/ui` 4.1.5 → 4.1.6
    - `ws` 推移的脆弱性を `npm audit fix` で解消
  - **Rust メジャー** (5 件):
    - `criterion` 0.5 → 0.7 (`black_box` の import を `criterion::` → `std::hint::` に移行。`-D warnings` 下の deprecation 警告を解消)
    - `reqwest` 0.12 → 0.13 (`rustls-tls` feature を `rustls` にリネーム / `form` / `query` を opt-in 化したため `github/device_flow.rs` + `github/client.rs` の呼び出し元に合わせて features を再宣言)
    - `zip` 2 → 5 (CLAUDE.md の pitfall に書かれていた yanked 2.6.x レンジを完全に回避。API は呼び出し元と source-compatible で、`ZipWriter` / `ZipArchive` / `SimpleFileOptions` / `CompressionMethod::Deflated` はそのまま動作)
    - `windows` 0.61 → 0.62 (`WNDCLASSW.hbrBackground: HBRUSH` が `Win32_Graphics_Gdi` feature 必須となったため、`cursor_watcher.rs` / `hotkey.rs` の HWND_MESSAGE window 用に追加)
    - `winreg` 0.55 → 0.56 (`RegValue.bytes` が `Vec<u8>` → `Cow<'_, [u8]>` に変わったため `registry::register_scheme` に 4 行の `.into()` アダプタを追加。HKCU 限定 / snapshot ordering / PII レダクション / `AppError` 伝播 はすべて保たれていることをレビューで確認)
  - **その他**: `tauri-build` 2.6.0 → 2.6.2、`tauri` 2.11.0 → 2.11.2、`mockito` 1.5 → 1.7 を `cargo upgrade` で minor / patch 更新。
- `docs/` の分割: 巨大化していた root `CLAUDE.md` を `app/CLAUDE.md` (Frontend) と `src-tauri/CLAUDE.md` (Backend) に分け、root には cross-cutting なドキュメント運用ルールと invariant のみを残す構成に整理。Claude Code セッションのコンテキスト消費を 1/3 以下に圧縮しつつ、ディレクトリ単位で auto-load される設計を活かす。
- `app/locales/{ja,en}.ts` の i18n key parity は 546 keys / 546 keys (差分 0) を維持。`composables/useTagChipInput.ts` 削除に伴う composable 数の再測定 (36 → 35) を `docs/architecture.json` / `docs/file_inventory.md` に反映済み。

## [0.0.2] - 2026-05-19 (pre-release)

v0.0.1 と同じく仮リリース系列 (provisional, SemVer 0.0.x で API 安定保証なし)。Marketplace 提出フローのバグ修正と、Windows カーソルサイズ拡大時に発生していたレジストリ書き込み偽陽性の修正が中心。

### Fixed

- マーケットプレイス『公式インデックスに提出』モーダル (Auto / Manual 両タブ) のテーマ選択ドロップダウンで、`name` が `LocalizedString` マップ形式のテーマ (マーケットプレイス由来テーマなど) に対して `[object Object]` と表示される問題を修正。`MarketplaceName` 型と `pickLocalizedName(name, locale)` composable を通じて現在ロケールのラベルに解決するよう変更し、`themeOptions` を script-side computed として宣言することで unimport の auto-import を確実に発火させた。`entryJson` 側は `th.name` を raw のまま保持し、公式インデックス entry の `LocalizedString` 形式 name と整合 (`FeaturedCard.vue` の `displayName` パターンと一致)。`SubmitThemeDialog.test.ts` に `name: { ja, en }` でマウントして `[object Object]` が出ないことを保証するリグレッションテストを追加。
- Creator の『複製するテーマを選択』モーダル、および マーケットプレイスの『公式インデックスに提出』モーダル (Auto / Manual 両タブ) で、`source/kind` が `marketplace` のテーマを選択肢から除外。マーケットプレイス由来テーマからの派生作品で出所が曖昧になる問題と、公式由来テーマの二重提出を防止する。
- Windows の「マウス ポインターとタッチ」でカーソルサイズを拡大した状態で Library からテーマを「適用」すると、カーソル画像自体は正しく差し替わっているのに `適用に失敗しました: レジストリエラー: SystemParametersInfoW の呼び出しに失敗: ハンドルが無効です。 (0x80070006)` というエラートーストが出る問題を修正。`SPI_SETCURSORS` に同梱される `SPIF_SENDCHANGE` の `WM_SETTINGCHANGE` ブロードキャスト経路で受信側 (シェル / アクセシビリティサービス) のカーネルハンドルがライフサイクル境界で `ERROR_INVALID_HANDLE (6)` を返した場合の偽陽性。レジストリ書き込みは既に完了しているため、既存の `ERROR_INVALID_WINDOW_HANDLE (1400)` と同じく `is_broadcast_false_positive` のホワイトリストに `HRESULT_FROM_WIN32(6) = 0x80070006` を追加し、debug ログ扱いで握りつぶす。ACCESS_DENIED など本物の Win32 エラーは引き続き伝播する。
- マーケットプレイス自動提出 (`submit_theme_auto`) が公式インデックス entry の `author` フィールドに提出者の GitHub username を上書きしていたバグを修正。`theme.json` の `author` フィールドが優先採用されるようになり、第三者が代理提出しても本来の作者クレジットが保持される (Manual タブの `th.author ?? githubUsername.value` と挙動を一致)。`theme.json` に `author` が無い場合のみ提出者 GitHub username にフォールバックする。あわせて公式インデックスリポ ([`easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index)) の `scripts/marketplace/validate.mjs` に `.cursorpack` 内 `theme.json` と `entry.author` の一致チェックを追加し、CI で drift を検出するようにした。既存 entry `f2d3825c` (Hamuchi Mouse Cursor) の `author` も `"nishiuriraku"` → `"無ナ"` に修正。

## [0.0.1] - 2026-05-18

仮リリース (provisional first release)。配布チャネル / 署名 / Updater 配線の試走を含む早期版で、機能セットは v0.1.0 計画相当だが、SemVer 上は安定 API を未保証として `0.0.x` にとどめている。

### Added

- 初回パブリックリリース (仮)。Windows 10 22H2+ / Windows 11 (x64) 対応。ARM64 ターゲットは CI でビルド検証済 (配布は次マイルストーン)。
- ローカルテーマ管理 (Library / Creator)、Marketplace 連携、Tauri Updater + Ed25519 署名 による自動アップデート、パニックリセット (`Ctrl+Alt+Shift+R`)、設定スナップショット / 復元、`.cursorpack` / `.cursorprofile` のインポート / エクスポートなどを含む。
- HKCU 限定の安全な適用フロー (適用前スナップショット → 失敗時自動ロールバック → 起動時不整合検知)。
- アーカイブ検閲 (path traversal 対策、サイズ上限 50/200/10/1024 MB、image metadata 剥離)。
- Creator 用 Ed25519 鍵管理 (DPAPI 暗号化保存、`.cfkey` import/export は XChaCha20-Poly1305 + Argon2id)。
- マーケットプレース提出フロー (GitHub Device Flow + 自動 PR 作成、署名 / SHA-256 検証は配信側で実施)。

### Fixed

- `get_app_info` IPC が常に `os_version: "Windows 0.0"` を返していたバグを修正 (`OSVERSIONINFOW::default()` のフィールドゼロのままだった)。`ntdll!RtlGetVersion` 経由でクランプされない真の OS バージョンを返す。
- `export_cursorpack_streamed` の sign / package 段階でビルドを中断したとき `build-progress` イベントに `stage: "cancelled"` が発火されず、Creator の進捗バーが固まる問題を修正。
- メインウィンドウの閉じるボタン押下時に Tauri ランタイムがそのままプロセス終了してしまい、トレイ常駐が機能していなかった問題を修正。`tauri::Builder::build()` + `App::run()` に切り替え、`RunEvent::ExitRequested { code: None, .. }` (ユーザー操作起点) のみ `api.prevent_exit()` で抑止することで、tray メニュー「終了」(`AppHandle::exit(0)` 経由 = `code: Some(0)`) と `AppHandle::restart` の終了経路は素通ししつつ、close ボタンでは WebView 破棄 + トレイ常駐 (Phase 4-1 メモリ最適化の設計意図) を回復。グローバルパニックホットキー (`Ctrl+Alt+Shift+R`) と `cursor_watcher`、`auto_start` のサイレント常駐モードが GUI クローズ後も継続して機能する。あわせて設定画面の自動起動説明文に残っていた dark-mode 連動の文言 (`(ダークモード自動切替を有効化)` / `(enables dark-mode auto-switch)`) を削除。
- トレイ常駐 (close ボタンによる `window.destroy()`) から「EasyCursorSwap を開く」「ダブルクリック」「`Ctrl+Alt+Shift+R`」「2 重起動」で WebView を再生成した際、`tray::show_or_recreate_main_window` の `WebviewWindowBuilder` が `tauri.conf.json` の `app.windows[0]` と乖離しており、ネイティブの Windows タイトルバー (`decorations(true)`) が `AppTitlebar.vue` の上に重なって描画され、ウィンドウサイズも 1100×750 (本来 1280×820) に縮んでいた問題を修正。再生成時のプロパティを conf.json と一致させ、`decorations(false)` のフレームレス前提を回復した。

### Changed

- パニックリセットフロー (`Ctrl+Alt+Shift+R`) が 17 ロール × 40ms の擬似アニメと「snapshot saved」「writing X → Y」「recovery completed in Xms」のハードコード英文ログを擬似的に表示していた挙動を廃止。実 IPC (`reset_to_default` / `reset_to_initial`) を即時呼び出し、完了/失敗のみを正直に表示するシンプルな UI に改修。
- サイドバーのバージョン表記がハードコード `v1.0` だったのを `useAppInfo` 経由で `Cargo.toml` の実バージョンを表示するよう変更。Creator 画面のヒーロー表示 (`CREATOR · v…`) も同様。

### Removed

- サイドバーの「トレイ常駐 · 11.4 MB」ステータス表示を削除 (`trayMemoryMb` props は親から渡されておらず、常にデフォルト値 11.4 の偽データだった)。関連する i18n キー `common.trayResident` も削除。
- 旧パニックフローで使用していた i18n キー `panic.writingProgress` / `panic.targetWindowsDefault` / `panic.targetSnapshot` を削除。代わりに `panic.recoveryStarted` / `panic.recoveryDone` / `panic.recoveryFailed` を追加。

### Internal

- マーケットプレースのフィルタチップ (`All` / `Pixel` / `Minimal` / `Animated` / `Dark`) とレイアウトのスキップナビゲーション (「メインコンテンツへスキップ」) を i18n 化。Creator のデフォルトテーマ名 `'Untitled Theme'` も `creator.untitledThemeName` 経由に。
- `docs/architecture.json` の `useUiTheme` 役割記述、および `docs/ui_map.json` の `titlebar-theme-cycle` インタラクションが「`update_config` IPC を呼ぶ / config に永続化する」と宣言していたが、実際には localStorage のみ操作する設計であるため記述を修正。生成済み HTML (`docs/architecture.html` / `docs/ui_map.html`) も再埋め込み。
- 構造的負債リファクタ (UI 軸監査 Phase 2 + 3 由来) — 6 つの新規 composable に共通パターンを集約:
  - `usePngBlobCache<K, V>` — Map + in-flight Promise + dispose の汎用パターン。`useThemePreviews` / `useMarketplacePreviews` の重複機構を統合 (C20-DUP-001)。
  - `useModalLifecycle` — Teleport modal の body scroll lock (重ね合わせ対応 counter 方式) + Esc 購読 + cleanup を統合。`ThemeDetailModal` / `MarketplaceDetailModal` / `OssLicenseModal` で重複していた ~30 行を解消 (D28-DUP-001)。
  - `useListbox<V>` — `UiSelect.vue` の listbox 状態機械 + キーボードナビ + Teleport 位置計算を分離 (F43-SIZE-001: 528 → 263 行)。
  - `useThemeCardState` — `ThemeCard.vue` / `ThemeRow.vue` の 5 ブロック並行重複を解消 (B9-DUP-001)。
  - `useExternalUrl` (`openExternalUrl`) — `open_url` IPC + `window.open` フォールバックの 6 callsite 重複を 1 行 await に圧縮 (AboutSection / OssLicenseModal / SubmitDeviceFlowModal / marketplace.vue / SubmitThemeDialog ×2 / ThemeDetailDrawer)。
  - `useTagChipInput` — `SubmitThemeDialog` Auto/Manual タブ間の tag chip 入力ロジック重複を解消 (D29 部分)。
- `useThemes` に theme mutation IPC 7 件 (`apply_theme` / `delete_theme` / `duplicate_theme` / `repackage_theme` / `set_theme_favorite` / `inspect_cursorpack` / `import_cursorpack`) のラッパーメソッドを追加。`pages/index.vue` から直接 `invokeTauri` していた経路を composable 経由に集約し、`docs/architecture.json` の `useThemes.ipc_calls` 宣言と実態を一致させた (B8-SIZE-001)。
- composable 総数: 28 → 36 (新規 8 件)。`docs/architecture.json` の `meta.measured_counts.composables` と composable リストを再測定。
- 監査 🔴 のうち `B10-SIZE-001` / `D29-SIZE-001` / `C20-SIZE-001` の純粋な file split を完遂:
  - `SubmitThemeDialog.vue` (576 行) を `SubmitThemeAutoForm.vue` / `SubmitThemeManualForm.vue` の 2 子コンポーネントに分離 (D29-SIZE-001)。`useMarketplaceSubmit` が singleton ではないため submitter は親で保持し、reactive な値を props で子に渡す設計。
  - `ThemeDetailDrawer.vue` (645 行) を `ThemeDetailDrawerHero.vue` / `Strip.vue` / `Footer.vue` の 3 子コンポーネントに分離 (B10-SIZE-001)。activeRole 内部状態は Hero に閉じ、emit はコンテナを通して props down 単方向。
  - `creator.vue` (1269 → 1056 行 / -17%) から `useCreatorMetaState` (メタデータ 6 ref + reset) を抽出、Assign タブ中央エディタを `CreatorEditorCanvas.vue` (~290 行) に切り出し (C20-SIZE-001 部分)。
  - `BulkImportPreviewModal.vue` (579 → 297 行 / -49%) から `useBulkImportPreviewState` を抽出。matches/unmatched の三方移動 state machine + props.open 連動の初期マッチ watch + Blob URL ライフサイクル + ApplyPayload 組立を composable に閉じ込め、SFC は presentation に専念 (audit C21-SIZE 部分)。`ApplyPayload` 型の output 場所も SFC から composable に移動 (`useCreatorBulkImportFlow` 側 import を更新)。
- component 総数: 50 → 56 (library +3 / marketplace +2 / creator +1)。`docs/architecture.json` / `docs/ui_map.json` の `measured_counts.components_total` を再測定し、HTML viewer に再埋め込み。

[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/nishiuriraku/easy-cursor-swap/releases/tag/v0.0.1
