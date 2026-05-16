# コードサイニング調達ガイド

EasyCursorSwap の `.msi` / `.exe` インストーラーに付与する Authenticode コードサイニングの
取得方針 (Phase 8-2 残)。

> [!IMPORTANT]
> Tauri Updater の **minisign 署名** とは別物。
> minisign 署名はアップデート差分の検証用 ([updater_signing.md](updater_signing.md))、
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

## SignPath Foundation 申請手順

1. https://signpath.org/apply から OSS プロジェクト登録 (2025 末にドメインが
   `about.signpath.io` から `signpath.org` に移行している)
2. 必要情報:
   - GitHub リポジトリ URL: https://github.com/nishiuriraku/easy-cursor-swap
   - ライセンス: MIT
   - メンテナの GitHub アカウント / 連絡先メール
   - 公開済み Code Signing Policy URL ([docs/code_signing_policy.md](code_signing_policy.md))
   - 少なくとも 1 つのリリース (Draft も可) が存在すること
3. 適格要件は https://signpath.org/terms.html の "Conditions for SignPath
   Foundation certificates" を満たす必要がある (本 repo は
   [docs/code_signing_policy.md](code_signing_policy.md) でカバー済み)
4. 1〜2 週間の審査後、SignPath プロジェクトページが発行される
5. CI で `signpath/github-action-submit-signing-request@v1` を使ってリリース
   成果物を送り、署名済みファイルを取得する流れ
   (既に [.github/workflows/release.yml](../.github/workflows/release.yml) に
   配線済み。Foundation 承認後は GitHub Variable `SIGNPATH_SIGNING_POLICY_SLUG`
   を `release-signing` に切り替えるだけで本番化される)

### SignPath Artifact Configuration

`tauri-installers` 構成は以下の XML を SignPath UI に登録 (MSI は `<msi-file>`、
NSIS/EXE は `<pe-file>` で **要素が分離** されている点に注意):

```xml
<?xml version="1.0" encoding="utf-8" ?>
<artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
  <zip-file>
    <pe-file path="**/*.exe">
      <authenticode-sign />
    </pe-file>
    <msi-file path="**/*.msi">
      <authenticode-sign />
    </msi-file>
  </zip-file>
</artifact-configuration>
```

### GitHub Secrets / Variables

| 種別     | 名前                            | 用途                                              |
| -------- | ------------------------------- | ------------------------------------------------- |
| Secret   | `SIGNPATH_API_TOKEN`            | SignPath User Settings → API Tokens で発行        |
| Secret   | `SIGNPATH_ORGANIZATION_ID`      | Organization Settings の UUID                     |
| Variable | `SIGNPATH_SIGNING_POLICY_SLUG`  | `test-signing` (テスト) / `release-signing` (本番) |

Variable 未設定時は `release.yml` 側で `test-signing` がデフォルトとして使われる。

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
2. **VirusTotal で検査して陰性であることを確認** ([index repo の validate.mjs の VT 統合](https://github.com/nishiuriraku/easy-cursor-swap-index/blob/main/scripts/marketplace/validate.mjs) は別物だが類似ツール)
3. **ユーザーに「実行」をクリックしてもらう** (累積数が SmartScreen 信用判定の母数)
4. 数百〜数千ダウンロード規模で警告が外れることが多い (Microsoft 非公開アルゴリズム)

警告が出ている期間中は README / リリースノートで:

> ⚠️ SmartScreen が「不明な発行元」の警告を出すことがあります。
> 「詳細情報」→「実行」で続行してください。本アプリは SignPath.io の OV 署名で
> 検証可能な発行元から配布されています。

と注記しておくとサポートコストが下がる。

---

## チェックリスト (Foundation 承認まで)

- [x] SignPath UI で Project / Artifact Configuration / Test Certificate / Test Signing Policy を作成
- [x] テスト署名を SignPath UI から手動で実行し、`signtool verify` でローカル検証に成功
- [x] release.yml に署名ステップ追加 (Secret 未設定時 skip ガード付き)
- [x] [docs/code_signing_policy.md](code_signing_policy.md) を公開
- [x] [signpath.org/apply](https://signpath.org/apply) から OSS Foundation 申請 (2026-05-16 送信)
- [x] テスト試走で workflow_dispatch 経由のパイプラインが緑になることを確認
      (run `25959975470` / x64 + aarch64 + Draft Release 自動生成 + 6 ファイル添付)
- [x] SIGNPATH_API_TOKEN / SIGNPATH_ORGANIZATION_ID を一時的に未登録のまま CI を緑に維持
- [ ] 審査承認 (1〜2 週間)
- [ ] Foundation 承認後の作業 (下記セクション参照)
- [ ] 署名済み `.msi` / `.exe` で SmartScreen 動作確認
- [x] README に SmartScreen 警告対応の説明を追記済み

---

## Foundation 承認後の運用切替

審査結果の承認メールが届いたら、以下の順で本番運用に移行する。

### 1. SignPath UI の状態確認

- [ ] Organization が Foundation tier に切り替わっていることを確認
- [ ] Projects > `easy-cursor-swap` で **Trusted Build Systems** タブ
      (または Signing Policy の Origin Verification セクション) が選択可能になっている

### 2. 本番 Signing Policy `release-signing` の作成

- [ ] `Signing Policies` → **Add** で `release-signing` policy を新規作成
- [ ] Certificate: SignPath Foundation が発行する OV 証明書を選択 (テスト証明書ではない)
- [ ] Approval mode: 初期は `Manual approval` → 安定後 `Automatic` に切替
- [ ] Allowed CI: **GitHub Actions** を有効化
- [ ] Repository restriction: `nishiuriraku/easy-cursor-swap`
- [ ] Tag pattern restriction: `refs/tags/v[0-9]+.[0-9]+.[0-9]+`

### 3. API Token の発行と GitHub Secrets / Variables 登録

- [ ] `User Settings → API Tokens → Create`
   - Scope: `easy-cursor-swap` project のみ
   - Lifetime: 1 year
- [ ] GitHub Secrets:
   - `SIGNPATH_API_TOKEN` (新規発行したトークン)
   - `SIGNPATH_ORGANIZATION_ID` (Organization Settings の UUID)
- [ ] GitHub Repository Variables:
   - `SIGNPATH_SIGNING_POLICY_SLUG` = `release-signing`
     (未設定なら release.yml 側のデフォルト `test-signing` が使われるため、本番時は必ず設定)

### 4. 本番タグで CI 実行

- [ ] `git tag v0.1.0 <commit-sha>` で本番タグを作成
- [ ] `git push origin v0.1.0` で push
- [ ] `on.push.tags` トリガーで release.yml が走り、SignPath 本番 OV 署名済みの
      `.exe` / `.msi` が Draft Release に並ぶことを確認

### 5. ローカル検証

- [ ] Draft Release から `.exe` / `.msi` をダウンロード
- [ ] `signtool verify /pa /v <file>.exe` で Issuer に SignPath Test ではなく
      本番 CA (Certum など) が表示されることを確認
- [ ] Windows SmartScreen で「不明な発行元」警告が **減少** することを確認
      (即時消えるわけではない。レピュテーション蓄積に数百〜数千 DL 必要)

### 6. ドキュメント更新 (同コミットで)

- [ ] 本ファイルの「Foundation 承認まで」チェックリストの未完項目に `[x]`
- [ ] `CHANGELOG.md [Unreleased] → Security` に「Foundation 承認 → 本番 OV 署名運用開始」追記
- [ ] `docs/code_signing_policy.md` の "Authenticode provider" 欄を OV 本番証明書情報に更新

### 想定リスクと回避策

| リスク | 影響 | 回避策 |
|---|---|---|
| 審査却下 (規約違反) | 高 | 却下理由を確認して追加対応。`docs/code_signing_policy.md` は要件を満たしているはず |
| 審査長期化 (2 週間超) | 中 | Trial の手動署名で凌ぐか、Microsoft Trusted Signing への一時切替を検討 (本ドキュメント比較表) |
| 承認後の Certum 証明書失効 | 低 | SignPath が自動更新。年次の Foundation 再審査がある |
| API Token 漏洩 | 中 | Token は project 単位スコープ + tag pattern 制限あり。漏洩時は SignPath UI で即時 revoke |

### 申請送信時の入力内容 (記録)

| フィールド | 値 |
|---|---|
| 申請日 | 2026-05-16 |
| Project name | EasyCursorSwap |
| Repository URL | `https://github.com/nishiuriraku/easy-cursor-swap` |
| Homepage URL | `https://github.com/nishiuriraku/easy-cursor-swap` |
| Download URL | `https://github.com/nishiuriraku/easy-cursor-swap/releases/latest` |
| Privacy Policy URL | `https://github.com/nishiuriraku/easy-cursor-swap/blob/main/docs/code_signing_policy.md#privacy` |
| License | MIT |
| Maintainer Type | Individual |
| Build System | GitHub Actions |
