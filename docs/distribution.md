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

> **現状 (2026-05-21):** Authenticode 署名は **未取得**。SignPath Foundation OSS
> 申請は外部認知不足で保留となり、再申請準備中。`release.yml` の SignPath ステップ
> は SIGNPATH_* secret 未設定時に自動 skip するため、CI は無影響。詳細と再申請
> ロードマップは [`authenticode_signing.md`](authenticode_signing.md) を参照。

### 候補

候補の全体像と推奨パスは [`authenticode_signing.md`](authenticode_signing.md) を
正本とする。本ファイルは概要のみ:

| サービス                            | 種類          | 条件                                                  | 状態 (2026-05-21)              |
| ----------------------------------- | ------------- | ----------------------------------------------------- | ------------------------------ |
| [SignPath.io](https://signpath.org/) Foundation | OV    | OSS プロジェクト無償、外部認知シグナル要件あり        | 一次審査保留、再申請準備中    |
| Certum Open Source (SimplySign)     | OV            | €29〜€69/年、個人 OSS 開発者向け、クラウド HSM        | 暫定有料案 (再申請長期化時)    |
| Microsoft Trusted Signing           | OV (相当)     | $9.99/月、Azure 経由                                  | 個人 onboarding 一時停止中     |
| SSL.com Code Signing                | EV/OV         | 商用、有料                                            | 参考のみ                       |

### SignPath.io 申請手順

詳細は [`authenticode_signing.md`](authenticode_signing.md) を参照。サマリ:

1. https://signpath.org/apply から OSS Foundation 申請
2. 必要書類: GitHub repo URL、ライセンス、メンテナ情報、公開済み Code Signing Policy URL
3. `.github/workflows/release.yml` の SignPath ステップは既に配線済 (skip-guard 付き)
4. 承認後に GitHub Secrets (`SIGNPATH_API_TOKEN` / `SIGNPATH_ORGANIZATION_ID`) を
   投入すれば自動的に有効化される
5. **2026-05-21 時点では一次審査保留**。再申請は外部認知シグナル蓄積後

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
