# コードサイニング調達ガイド

EasyCursorSwap の `.msi` / `.exe` インストーラーに付与する Authenticode コードサイニングの
取得方針 (Phase 8-2 残)。

> [!IMPORTANT]
> Tauri Updater の **minisign 署名** とは別物。
> minisign 署名はアップデート差分の検証用 ([updater_signing.md](updater_signing.md))、
> Authenticode 署名は Windows SmartScreen / UAC の警告軽減用。
> 両方が必要。

> [!WARNING]
> **現状 (2026-05-21):** SignPath Foundation OSS 一次申請は外部認知不足を理由に
> **保留** されました (詳細は本ファイル末尾の「一次審査の結果と再申請ロードマップ」)。
> ポリシーや技術要件には問題なしとの回答。当面は **無署名で配布** し、外部認知
> (GitHub stars / 紹介記事 / コミュニティ言及) が伸びた段階で再申請します。
> `release.yml` の SignPath ステップは SIGNPATH_* secret 未設定時に skip される
> 設計なので、再承認時に GitHub Secrets を投入するだけで自動的に有効化されます。

---

## 選択肢の比較

| 方式                              | 年間費用      | OSS 無償 | EV / OV   | SmartScreen 即時信用          | 取得期間         | 備考                                                                                                                          |
| --------------------------------- | ------------- | -------- | --------- | ----------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **SignPath.io (Foundation)**      | 無償          | ✅       | OV        | ❌ (レピュテーション蓄積必要) | 1〜2 週間 (審査) | OSS プロジェクト向け。**2026-05-21 一次審査で外部認知不足のため保留** — 認知度蓄積後に再申請                                  |
| **Certum Open Source (SimplySign)** | 約 €29〜€69  | —        | OV        | ❌ (レピュテーション蓄積必要) | 1〜2 週間        | 個人 OSS 開発者向け。クラウド HSM 版なら USB トークン不要で CI 連携可。**SignPath 再申請が長期化した場合の暫定有料案として最有力** |
| **Microsoft Trusted Signing**     | $9.99/月〜    | ❌       | OV (相当) | ✅ (即時)                     | 1 日〜           | Azure サブスクリプション必須。**2026 年現在、個人開発者の新規 onboarding は一時停止**                                         |
| **DigiCert / Sectigo EV**         | 約 $400〜500  | ❌       | EV        | ✅ (即時)                     | 1〜3 営業日      | HSM/USB トークン必須                                                                                                          |
| **Sectigo OV (個人)**             | 約 $200〜350  | ❌       | OV        | ❌ (レピュテーション蓄積必要) | 1 週間程度       | 個人開発者でも取得可                                                                                                          |
| **Microsoft Store (MSIX 自動署名)** | 無償         | ✅       | OV (相当) | ✅ (即時)                     | 数営業日 (審査)  | Store 公開時に Microsoft が自動署名。**Tauri は MSIX を直接生成できない** (tauri-apps/tauri#8548 / #4818) ため現状実用性低い |
| **無署名 (現状の運用)**           | 0             | —        | —         | ❌                            | —                | SmartScreen で「不明な発行元」警告が出る。README で案内、`More info → Run anyway` で続行可能                                  |

---

## 推奨パス (2026-05-21 改訂)

**短期 (〜次回 SignPath 再申請まで、数か月〜半年想定)**: **無署名のままリリース継続**。

- README / Wiki FAQ で SmartScreen 警告の案内文を提示済 (本リポジトリでは整備済)
- Tauri Updater の minisign 署名は引き続き有効 (アップデート改ざん防止は維持)
- 配布物は GitHub の公開ワークフローでビルドされ再現性があり、`docs/code_signing_policy.md`
  で署名ポリシーを公開済 → 再申請時にそのまま使える

**中期 (3〜6 か月後)**: 外部認知シグナルを蓄積した上で **SignPath Foundation に再申請**。

- 外部認知シグナルの作り方は本ファイル末尾の「再申請ロードマップ」を参照
- 再申請通過後は `release.yml` の SignPath ステップが Secret 投入で自動有効化

**並行オプション (再申請が長期化した場合の暫定切替)**: **Certum Open Source Code
Signing (SimplySign クラウド HSM 版)** — 個人 OSS 開発者向け、€29〜€69/年と最も安価。

- USB トークン不要、CI 連携可
- Identity Validation が必要 (数営業日)
- SmartScreen レピュテーションは SignPath 通過時と同様に蓄積待ちが必要

**ユーザーが大きく増えた段階 (将来)**: Microsoft Trusted Signing への移行を検討。

- 2026 年現在 **個人開発者の新規 onboarding は一時停止中**、再開待ち
- 開放後は月額 $9.99 + Azure サブスクリプション
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

### 現状 (無署名運用中) の README / リリースノート文面

SignPath Foundation 一次申請保留中 (2026-05-21) は **未署名のまま配布** している
ため、上記 "OV 署名済"前提の注記は使わない。代わりに以下のように案内する
(README / README.ja で実際に採用済):

> ⚠️ Authenticode (Windows code signing) is **not yet provisioned** — the
> SignPath Foundation OSS application was deferred on 2026-05-21 pending broader
> project visibility, and reapplication is planned. Until then, Windows
> SmartScreen may show an "Unknown publisher" warning; click **More info → Run
> anyway** to proceed. Releases remain verifiable via the Tauri Updater's
> Ed25519 (minisign) signature; the source is MIT-licensed and built reproducibly
> in public GitHub Actions.

### Foundation 承認後の README / リリースノート文面 (再申請通過時に切替)

承認後は以下のような OV 署名前提の注記に置き換える:

> ⚠️ SmartScreen が「不明な発行元」の警告を出すことがあります。
> 「詳細情報」→「実行」で続行してください。本アプリは SignPath.io の OV 署名で
> 検証可能な発行元から配布されています。

と注記しておくとサポートコストが下がる。

---

## チェックリスト (一次申請の経緯と再申請までの状態)

- [x] SignPath UI で Project / Artifact Configuration / Test Certificate / Test Signing Policy を作成
- [x] テスト署名を SignPath UI から手動で実行し、`signtool verify` でローカル検証に成功
- [x] release.yml に署名ステップ追加 (Secret 未設定時 skip ガード付き)
- [x] [docs/code_signing_policy.md](code_signing_policy.md) を公開
- [x] [signpath.org/apply](https://signpath.org/apply) から OSS Foundation 申請 (2026-05-16 送信)
- [x] テスト試走で workflow_dispatch 経由のパイプラインが緑になることを確認
      (run `25959975470` / x64 + aarch64 + Draft Release 自動生成 + 6 ファイル添付)
- [x] SIGNPATH_API_TOKEN / SIGNPATH_ORGANIZATION_ID を一時的に未登録のまま CI を緑に維持
- [x] **一次審査 (2026-05-21): 保留** — 外部認知シグナル不足 (詳細は本ファイル「一次審査の結果と再申請ロードマップ」)
- [x] README / README.ja / Wiki Usage-Guide-JA に「未署名運用」を反映 (虚偽の「signed via SignPath」表記を撤去)
- [ ] 外部認知シグナル蓄積期間 (3〜6 か月) — 再申請ロードマップに従う
- [ ] 再申請送信 (タイミング: stars / 紹介記事 / 月次 DL がロードマップの目安に達したら)
- [ ] 審査承認 (1〜2 週間)
- [ ] Foundation 承認後の作業 (下記「Foundation 承認後の運用切替」セクション参照)
- [ ] 署名済み `.msi` / `.exe` で SmartScreen 動作確認

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

### 想定リスクと回避策 (Foundation 承認後の運用フェーズ)

| リスク | 影響 | 回避策 |
|---|---|---|
| 承認後の Certum 証明書失効 | 低 | SignPath が自動更新。年次の Foundation 再審査がある |
| API Token 漏洩 | 中 | Token は project 単位スコープ + tag pattern 制限あり。漏洩時は SignPath UI で即時 revoke |
| 年次再審査で外部認知が再評価され保留 | 中 | リリース継続 / コミュニティ活動を維持。再保留時は本ファイル「再申請ロードマップ」と同じ運用 |

### 申請送信時の入力内容 (記録)

| フィールド | 値 |
|---|---|
| 申請日 | 2026-05-16 |
| 結果通知日 | 2026-05-21 (**保留** — 外部認知不足) |
| Project name | EasyCursorSwap |
| Repository URL | `https://github.com/nishiuriraku/easy-cursor-swap` |
| Homepage URL | `https://github.com/nishiuriraku/easy-cursor-swap` |
| Download URL | `https://github.com/nishiuriraku/easy-cursor-swap/releases/latest` |
| Privacy Policy URL | `https://github.com/nishiuriraku/easy-cursor-swap/blob/main/docs/code_signing_policy.md#privacy` |
| License | MIT |
| Maintainer Type | Individual |
| Build System | GitHub Actions |

---

## 一次審査の結果と再申請ロードマップ

### 2026-05-21 一次審査の結果 (保留)

SignPath GmbH (Phillip Deng 氏) より、以下の理由で Foundation 証明書発行は **保留** との
通知を受領しました。**ポリシー文書 / 技術要件 / プロジェクトの品質に問題はなく**、純粋に
"外部から見た公的信頼の蓄積" が不足しているという理由です。

> When evaluating projects for the SignPath Foundation program, we look at a
> combination of factors that help us verify a project's reputation and standing.
> These typically include signals such as:
>
> - Community adoption (e.g., GitHub stars, forks, contributors)
> - Independent references or discussions (Reddit, Stack Overflow, YouTube, etc.)
> - External articles, blog posts, or institutional backing
> - Evidence of sustained activity and user engagement
>
> At the moment, your project does not yet provide sufficient external verification
> signals for us to issue a Foundation certificate in our name. […]
> Once it has gained broader recognition, you are very welcome to reapply.

### 再申請ロードマップ

SignPath が見ている指標を意識的に蓄積する。目安は以下 (公式数値ではなく経験値):

| シグナル                  | 目標目安                       | 具体策                                                                                  |
| ------------------------- | ------------------------------ | --------------------------------------------------------------------------------------- |
| GitHub stars / forks      | star ≥ ~100, fork / contributor 複数 | Hacker News (Show HN) / Reddit (`r/windows`, `r/cursors`, `r/rust`, `r/tauri`, `r/Windows11`) でデモ投稿 |
| 独立した第三者の言及      | 複数の独立ソース               | **Zenn / Qiita / note / dev.to** に開発記事 ("Tauri で Windows カーソル管理アプリを作った")              |
| 動画 / SNS                | YouTube 短いデモ動画 1 本以上、X (旧 Twitter) で開発ログ | 30 秒のデモ動画、開発進捗の継続発信                                                     |
| 外部記事 / メディア露出   | 紹介系サイト 1〜2 件           | **窓の杜 / ITmedia / Forest** など Windows 向け OSS 紹介系メディアに情報提供メール       |
| ダウンロード実績          | 月間数百〜                     | GitHub Releases の **download count バッジ** を README に表示                            |
| 持続的な活動              | 数か月以上の継続コミット       | 定期的にバグ修正 / 機能追加リリースを継続                                                |

### 再申請のタイミング

- **最低でも 3〜6 か月後** — 即時再申請は逆効果になりかねない
- 上記指標のうち **GitHub stars / 第三者の言及 / 月次 DL** のいずれか 2 つ以上が
  目安に達したら再申請を検討する
- 再申請時は本ドキュメント / `docs/code_signing_policy.md` をそのまま提示できる

### 再申請が長期化した場合の暫定策

外部認知の伸びが想定より遅い場合、**Certum Open Source Code Signing (SimplySign)**
への暫定切替を検討する (€29〜€69/年)。本ドキュメント上部の比較表を参照。

### 想定リスクと回避策 (再申請フェーズ)

| リスク | 影響 | 回避策 |
|---|---|---|
| 再申請も同じ理由で保留 | 中 | star / 言及シグナルが目に見えて伸びてから出す。間隔を空ける |
| 認知度が伸びない | 中 | 上記ロードマップを淡々と実行。半年スパンで見る |
| 個別の有料 OV 切替が必要になる | 中 | Certum OSS が最安 (€29〜€69/年)。Trusted Signing は個人 onboarding 再開待ち |
| 配布物への信頼質問が増える | 中 | README / Wiki FAQ で「未署名 + Updater minisign 署名は有効 + 再現可能ビルド」を明示済 |
