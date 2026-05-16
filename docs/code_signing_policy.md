# Code Signing Policy

EasyCursorSwap (cursor-forge) は [SignPath Foundation](https://signpath.org/) が
提供する OSS 向け Authenticode コードサイニング証明書で、Windows インストーラー
(`.exe` / `.msi`) およびその中に含まれる主要 PE ファイルを署名しています。

本ポリシーは SignPath Foundation の OSS 適格要件
([signpath.org/terms.html](https://signpath.org/terms.html)) を満たすための
公開ドキュメントです。

---

## Project Identity

| 項目                  | 値                                                          |
| --------------------- | ----------------------------------------------------------- |
| Project name          | EasyCursorSwap (repository: `easy-cursor-swap`)             |
| Source code           | https://github.com/nishiuriraku/easy-cursor-swap            |
| License               | MIT ([LICENSE](../LICENSE))                                 |
| Distribution channel  | GitHub Releases (signed installers) + Tauri Updater         |
| Marketplace index     | https://github.com/nishiuriraku/easy-cursor-swap-index      |
| Authenticode provider | SignPath Foundation (OSS) — request via signpath.org/apply  |

---

## Team

cursor-forge は個人開発プロジェクトです。SignPath Foundation が定める
Author / Reviewer / Approver の 3 ロールを単一メンテナが兼務します。

| Role                       | GitHub handle                                          | Contact                  |
| -------------------------- | ------------------------------------------------------ | ------------------------ |
| Author / Reviewer / Approver | [@nishiuriraku](https://github.com/nishiuriraku)     | (GitHub プロフィール参照) |

### Multi-Factor Authentication

- **GitHub**: 全アカウントで 2FA 必須
  ([Settings → Security](https://github.com/settings/security) で有効化)
- **SignPath.io**: アカウントログインで MFA 必須
- **GitHub Secrets**: SignPath API Token はリポジトリ Settings の Secrets として
  暗号化保存。閲覧不可な書き込み専用フローで Actions に渡す

### Access Control

- SignPath API Token のスコープは **`easy-cursor-swap` プロジェクト 1 つのみ**。
- `release-signing` ポリシーは **`refs/tags/v[0-9]+.[0-9]+.[0-9]+`** にマッチする
  タグ push でのみ発動。フォーク PR からの署名要求は SignPath 側 GitHub Actions
  メタデータ検証で自動拒否される。
- 鍵交換 / 鍵失効手順は [docs/key_rotation.md](key_rotation.md) を参照。

---

## Privacy

EasyCursorSwap はユーザーのプライバシーを尊重し、収集する情報は最小限です。

### 収集しない情報

- カーソル画像・テーマ・設定の内容
- ユーザーのマシン ID / メールアドレス / 利用統計
- レジストリのスナップショット内容 (ローカル `~/.custom_cursors/` のみ)

### オプトインで送信する情報

クラッシュレポート機能 (デフォルト **OFF**) を明示的に有効化した場合のみ:

- アプリのバージョン / OS バージョン (例 `Windows 11 23H2 26100.1`)
- パニック (panic) のスタックトレース (パスは `redact_path` でマスク、SHA-256 は
  先頭 12 文字に短縮、`logging::short_hash` で正規化)

送信先は cursor-forge の所有する Cloudflare Worker のみ。第三者には開示しません。
**無効化方法**: `~/.custom_cursors/_config.json` の `crash_reporting: false` に
変更、または GUI の「設定 → クラッシュレポート」トグルを OFF。

### システム変更の通知

EasyCursorSwap は Windows レジストリの **`HKCU\Control Panel\Cursors` 以下のみ**
を書き換えます。HKLM や UAC 必須領域は一切操作しません。テーマ適用の前に
`~/.custom_cursors/_pending_apply.snapshot` にロールバック用スナップショットを
書き出し、成功時に削除します。起動時に残存スナップショットがあれば自動復元します。

### アンインストール

- NSIS / MSI どちらの installer も **per-user (現在のユーザーのみ)** で動作し、
  Programs and Features (`appwiz.cpl`) からアンインストールできます。
- カーソルファイルは `~/.custom_cursors/` に保存され、**アンインストール時も
  既定では削除されません** (ユーザーの作品を保護するため)。完全削除はユーザーが
  手動で当該ディレクトリを削除する必要があります。
- Panic button (`Ctrl+Alt+Shift+R`) は Windows 既定または初期スナップショットへ
  即座にカーソルを戻します。

---

## Build Reproducibility

すべての公式リリース成果物は GitHub Actions の公開ワークフローでビルドされます。

| 項目                  | 値                                                       |
| --------------------- | -------------------------------------------------------- |
| Build environment     | `windows-latest` (GitHub-hosted runner)                  |
| Build trigger         | Tag push: `v[0-9]+.[0-9]+.[0-9]+`                        |
| Workflow source       | [.github/workflows/release.yml](../.github/workflows/release.yml) |
| Toolchain             | Rust stable (dtolnay/rust-toolchain@stable), Node.js 22  |
| Targets               | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`      |
| Output bundles        | NSIS (`.exe`) + MSI (`.msi`), per-user installer         |
| Updater signing       | Ed25519 (minisign) — see [updater_signing.md](updater_signing.md) |
| Authenticode signing  | SignPath Foundation OSS — submission via API token       |

ソースコードは MIT ライセンスで完全公開されており、第三者がローカルで同一バイナリを
再現できることを目標としています。再現性に影響する変更は CHANGELOG に明記します。

---

## File Metadata Attributes

すべての署名対象 PE ファイルには Tauri バンドラーによって以下のメタデータ属性が
埋め込まれます。

| Attribute       | Value                                                              |
| --------------- | ------------------------------------------------------------------ |
| ProductName     | EasyCursorSwap                                                     |
| FileDescription | EasyCursorSwap — Mouse Cursor Theme Manager                        |
| CompanyName     | nishiuriraku                                                       |
| LegalCopyright  | © nishiuriraku. MIT License.                                       |
| FileVersion     | semver と一致 (例 `0.1.0.0`)                                       |
| ProductVersion  | semver (例 `0.1.0`)                                                |
| OriginalFilename | `EasyCursorSwap.exe` または `EasyCursorSwap_<ver>_<arch>_<locale>.msi` |

これらは [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) の
`bundle.windows` 設定で一元管理されます。

---

## Reporting Issues

セキュリティ上の懸念や署名済みバイナリへの疑義は GitHub Issues ではなく
[`SECURITY.md`](../SECURITY.md) の手順に従って報告してください。

最終更新: 2026-05-16
