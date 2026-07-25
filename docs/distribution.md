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

- 配布物は EV/OV 証明書で署名 (SmartScreen レピュテーション獲得) — **将来目標**
- OSS 向けの無償署名サービスを第一候補

> **現状 (2026-07-25):** `release.yml` から SignPath 関連 step は撤去済
> (Wave 0A)。Microsoft Store / MSIX 自動署名を正準経路とする方針に変更。
> GitHub Releases 配布の NSIS / MSI は当面 **無署名** で継続し、SmartScreen 警告は
> README / リリースノート側で `Run anyway` を案内。詳細と方針変更理由は
> [`authenticode_signing.md`](authenticode_signing.md) を参照。

### 候補

候補の全体像と推奨パスは [`authenticode_signing.md`](authenticode_signing.md) を
正本とする。本ファイルは概要のみ:

| サービス                                       | 種類          | 条件                                              | 状態 (2026-07-25)              |
| ---------------------------------------------- | ------------- | ------------------------------------------------- | ------------------------------ |
| **Microsoft Store (MSIX 自動署名)**            | OV (相当)     | Partner Center 登録 + Identity Validation          | **採用方針** (Wave 4A で CI 化) |
| Microsoft Trusted Signing                      | OV (相当)     | $9.99/月、Azure 経由                              | 個人 onboarding 一時停止中     |
| Certum Open Source (SimplySign)                | OV            | €29〜€69/年、個人 OSS 開発者向け、クラウド HSM    | Store で代替可能なため優先度低  |
| SSL.com Code Signing                           | EV/OV         | 商用、有料                                        | 参考のみ                       |
| [SignPath.io](https://signpath.org/) Foundation | OV            | OSS プロジェクト無償、外部認知シグナル要件あり    | **再申請しない** (2026-07-25)   |

### Microsoft Store / MSIX 移行手順 (方針)

詳細は [`authenticode_signing.md`](authenticode_signing.md) を参照。サマリ:

1. Microsoft Partner Center で individual developer 登録 (Identity Validation)
2. `build-msix-artifacts.yml` (別 workflow、Wave 4A) で `makeappx` + test 自己署名
   (`CN=EasyCursorSwap-Dev`) で MSIX をビルドし、CI runner で `Add-AppxPackage`
   → sentinel HKCU write → cleanup のスモーク
3. AppxManifest は `distribution/msix/AppxManifest.xml` を参照 (`rescap:unvirtualizedResources`
   + `rescap6:RegistryWriteVirtualization=disabled` 設定済)
4. Partner Center 経由で本番 MSIX を提出 (Microsoft が自動署名)
5. **2026-07-25 時点では方針確定のみ**。実装は Wave 4A〜4C の範囲

## SmartScreen レピュテーション獲得

新規発行の証明書 (および無署名配布) は SmartScreen の警告を受ける。緩和策:

1. **EV 証明書を使う** → 即時レピュテーション (個人開発では運用コスト見合わず除外)
2. **OV 証明書を使う** → 数週間〜数か月のダウンロード実績で警告解消
3. **無署名で配布 (現状の運用)** → README / Wiki FAQ で警告案内、`More info → Run anyway`
   で続行可能。Tauri Updater minisign 署名で改ざん防止は別途維持

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
