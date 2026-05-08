# EasyCursorSwap - 配布手順

仕様書 Phase 8-2/8-3 に対応する配布フロー雛形。

## 配布形態

| 形式             | 用途                                            | 状態                             |
| ---------------- | ----------------------------------------------- | -------------------------------- |
| `.msi`           | デフォルト配布 (GitHub Releases / 自社サイト)   | ✅ Tauri ビルダーで自動生成      |
| `.nsis` (`.exe`) | より柔軟なインストーラー (perUser インストール) | ✅ Tauri ビルダーで自動生成      |
| `.msix`          | Microsoft Store 配布 / 高度なサンドボックス     | 🔄 手動変換 (本ドキュメント参照) |

## ビルド手順

### 1. `.msi` + `.nsis` の生成

```pwsh
# 開発時
npm run tauri:dev

# リリースビルド (release プロファイル)
npm run tauri:build
```

出力先:

- `src-tauri/target/release/bundle/msi/EasyCursorSwap_*.msi`
- `src-tauri/target/release/bundle/nsis/EasyCursorSwap_*-setup.exe`

### 2. `.msix` への変換 (Microsoft Store 向け)

Tauri 自体は `.msix` を直接出力しないため、以下のいずれかで変換する。

#### 2a. MSIX Packaging Tool 経由 (GUI)

1. Microsoft Store から **MSIX Packaging Tool** をインストール
2. 「Create package from existing installer」を選択
3. 上記 `.msi` を入力に指定
4. **Identity** タブで `dev.easycursorswap.app` を確認
5. **AppxManifest.xml** を [`distribution/msix/AppxManifest.xml`](../distribution/msix/AppxManifest.xml) で上書き
6. 出力: `EasyCursorSwap.msix`

#### 2b. `makeappx` + signtool (CLI)

```pwsh
# 1. payload directory を準備
mkdir msix-payload
xcopy src-tauri\target\release\* msix-payload\ /E /Y
copy distribution\msix\AppxManifest.xml msix-payload\AppxManifest.xml
xcopy src-tauri\icons\Square*.png msix-payload\Assets\ /Y

# 2. パッケージ生成
makeappx pack /d msix-payload /p EasyCursorSwap.msix /v

# 3. 署名 (テスト用 self-signed certificate)
signtool sign /a /v /fd SHA256 /f cert.pfx /p "<password>" EasyCursorSwap.msix
```

## コードサイニング

仕様書「§5 コードサイニング」要件:

- 配布物は EV/OV 証明書で署名 (SmartScreen レピュテーション獲得)
- OSS 向けの無償署名サービスを第一候補

### 候補

| サービス                            | 種類          | 条件                 |
| ----------------------------------- | ------------- | -------------------- |
| [SignPath.io](https://signpath.io/) | OV (組織検証) | OSS プロジェクト無償 |
| Microsoft Trusted Signing           | EV            | $9.99/月、Azure 経由 |
| SSL.com Code Signing                | EV/OV         | 商用、有料           |

### SignPath.io 申請手順 (推奨)

1. https://signpath.io/ でアカウント作成
2. プロジェクト作成 → GitHub Actions 連携
3. `.github/workflows/release.yml` に SignPath SignAction 追加
4. リリース時に `.msi` / `.exe` / `.msix` を自動署名

## SmartScreen レピュテーション獲得

新規発行の証明書は SmartScreen の警告を受ける。緩和策:

1. **EV 証明書を使う** → 即時レピュテーション
2. **OV 証明書を使う** → 数週間〜数か月のダウンロード実績で警告解消
3. **未署名で配布** → 不可 (常駐 + 自動起動アプリのため)

## アップデートチャネル

`tauri.conf.json` の `plugins.updater.endpoints` で指定:

```json
{
  "endpoints": [
    "https://github.com/nishiuriraku/easy-cursor-swap/releases/latest/download/latest.json"
  ]
}
```

GitHub Releases の `latest.json` フォーマット:

```json
{
  "version": "1.0.1",
  "notes": "リリースノート",
  "pub_date": "2026-05-20T10:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "...Tauri-signer 署名...",
      "url": "https://github.com/nishiuriraku/easy-cursor-swap/releases/download/v1.0.1/EasyCursorSwap_1.0.1_x64-setup.nsis.zip"
    }
  }
}
```

公開鍵は `tauri signer generate` で発行し、`tauri.conf.json` の `plugins.updater.pubkey` に投入。

## v1.0 既知制約 (README 明記)

- Windows 10 22H2 以降 / Windows 11 のみサポート (Win10 21H2 以前は非対象)
- RDP / Citrix / RemoteApp は動作対象外 (起動時バナーで警告)
- Windows Server エディションは動作対象外
- `.ani` の新規生成は未対応 (インポートのみ)
- ライブプレビューなし / Undo なし / 自動切替はダークモード連動のみ
- UAC Secure Desktop / ロック画面 / サインイン画面では Windows 既定カーソルが表示される
