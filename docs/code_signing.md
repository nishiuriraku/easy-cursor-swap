# コードサイニング調達ガイド

EasyCursorSwap の `.msi` / `.exe` インストーラーに付与する Authenticode コードサイニングの
取得方針 (Phase 8-2 残)。

> [!IMPORTANT]
> Tauri Updater の **minisign 署名** とは別物。
> minisign 署名はアップデート差分の検証用 ([signing.md](signing.md))、
> Authenticode 署名は Windows SmartScreen / UAC の警告軽減用。
> 両方が必要。

---

## 選択肢の比較

| 方式                          | 年間費用     | OSS 無償 | EV / OV   | SmartScreen 即時信用          | 取得期間         | 備考                                   |
| ----------------------------- | ------------ | -------- | --------- | ----------------------------- | ---------------- | -------------------------------------- |
| **SignPath.io (Foundation)**  | 無償         | ✅       | OV        | ❌ (レピュテーション蓄積必要) | 1〜2 週間 (審査) | OSS プロジェクト向け。年間更新審査あり |
| **Microsoft Trusted Signing** | $9.99/月〜   | ❌       | OV (相当) | ✅ (即時)                     | 1 日〜           | Azure サブスクリプション必須           |
| **DigiCert / Sectigo EV**     | 約 $400〜500 | ❌       | EV        | ✅ (即時)                     | 1〜3 営業日      | HSM/USB トークン必須                   |
| **Sectigo OV (個人)**         | 約 $200〜350 | ❌       | OV        | ❌ (レピュテーション蓄積必要) | 1 週間程度       | 個人開発者でも取得可                   |
| **無署名**                    | 0            | —        | —         | ❌                            | —                | SmartScreen で警告ダイアログが必ず出る |

---

## 推奨パス

**v1.0 リリースまで**: SignPath.io Foundation で OV 署名取得。

- OSS なので無償
- リリース毎に SignPath の Web UI から署名要求 → 自動承認
- SmartScreen のレピュテーションは数十〜数百ダウンロードで通り始める

**ユーザーが増えてきた段階**: Microsoft Trusted Signing への移行を検討。

- 月額 $9.99 + Azure サブスクリプション
- Microsoft 自身のルート CA から発行されるため SmartScreen 即時信用
- HSM 不要、Azure Key Vault に署名鍵が格納される

EV 証明書 (HSM 必須) は個人開発では運用コストが見合わないため除外。

---

## SignPath.io 申請手順

1. https://about.signpath.io/foundation 申請フォームから OSS プロジェクト登録
2. 必要情報:
   - GitHub リポジトリ URL: https://github.com/nishiuriraku/easy-cursor-swap
   - ライセンス: MIT
   - メンテナの GitHub アカウント / 連絡先メール
3. 1〜2 週間の審査後、SignPath プロジェクトページが発行される
4. CI で `signpath-org-action@v1` を使ってリリース成果物を送り、
   署名済みファイルを取得する流れ

`.github/workflows/release.yml` への追記例:

```yaml
- name: Sign with SignPath
  uses: signpath/github-action-submit-signing-request@v1
  with:
    api-token: ${{ secrets.SIGNPATH_API_TOKEN }}
    organization-id: '<signpath-org-id>'
    project-slug: 'easy-cursor-swap'
    signing-policy-slug: 'release-signing'
    artifact-configuration-slug: 'tauri-installers'
    github-artifact-id: ${{ steps.upload.outputs.artifact-id }}
    wait-for-completion: true
    output-artifact-directory: 'signed'
```

---

## Microsoft Trusted Signing 申請手順 (将来移行用)

参照: https://learn.microsoft.com/en-us/azure/trusted-signing/

1. Azure サブスクリプション作成
2. Trusted Signing アカウント作成 (`Microsoft.CodeSigning` リソース プロバイダー)
3. Identity Validation 申請 (個人 or 組織) — 数営業日で承認
4. Certificate Profile 作成
5. CI で `azure/trusted-signing-action@v0` を使う

```yaml
- name: Sign with Trusted Signing
  uses: azure/trusted-signing-action@v0
  with:
    azure-tenant-id: ${{ secrets.AZURE_TENANT_ID }}
    azure-client-id: ${{ secrets.AZURE_CLIENT_ID }}
    azure-client-secret: ${{ secrets.AZURE_CLIENT_SECRET }}
    endpoint: 'https://eus.codesigning.azure.net/'
    trusted-signing-account-name: 'easycursorswap-signing'
    certificate-profile-name: 'easy-cursor-swap-release'
    files-folder: 'src-tauri/target/release/bundle'
    files-folder-filter: 'msi,exe'
    file-digest: 'SHA256'
```

---

## SmartScreen レピュテーション

OV 署名でリリースした場合、初期は SmartScreen が「不明な発行元」警告を出す。
レピュテーションを蓄積するには:

1. **同一証明書で継続的にリリースする** (毎リリース別証明書だとリセット)
2. **VirusTotal で検査して陰性であることを確認** ([validate.mjs の VT 統合](../scripts/marketplace/validate.mjs) は別物だが類似ツール)
3. **ユーザーに「実行」をクリックしてもらう** (累積数が SmartScreen 信用判定の母数)
4. 数百〜数千ダウンロード規模で警告が外れることが多い (Microsoft 非公開アルゴリズム)

警告が出ている期間中は README / リリースノートで:

> ⚠️ SmartScreen が「不明な発行元」の警告を出すことがあります。
> 「詳細情報」→「実行」で続行してください。本アプリは SignPath.io の OV 署名で
> 検証可能な発行元から配布されています。

と注記しておくとサポートコストが下がる。

---

## チェックリスト

- [ ] SignPath.io 申請フォーム送信
- [ ] 審査承認 (1〜2 週間)
- [ ] SIGNPATH_API_TOKEN を GitHub Secrets に登録
- [ ] release.yml に署名ステップ追加
- [ ] 署名済み `.msi` で Smartscreen 動作確認
- [ ] README に SmartScreen 警告対応の説明を追記
