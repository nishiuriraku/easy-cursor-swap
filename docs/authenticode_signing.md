# コードサイニング調達ガイド

EasyCursorSwap の `.msi` / `.exe` (NSIS) / `.msix` インストーラーに付与する
Authenticode コードサイニングの取得方針。

> [!IMPORTANT]
> Tauri Updater の **minisign 署名** とは別物。
> minisign 署名はアップデート差分の検証用 ([updater_signing.md](updater_signing.md))、
> Authenticode 署名は Windows SmartScreen / UAC の警告軽減用。
> 両方が必要だが、minisign は tauri-action が CI で自動付与するため本ドキュメントの
> スコープ外。

> [!WARNING]
> **方針変更 (2026-07-25):** **Microsoft Store 経由の MSIX 自動署名** に移行する。
> 旧 SignPath Foundation OSS 一次申請 (2026-05-21) は外部認知不足のため保留となり、
> 再申請はしない方針に変更。NSIS / MSI 配布 (GitHub Releases) は当面 **無署名** の
> まま継続し、Authenticode が必要なユーザーは Microsoft Store 版を案内する。MSIX
> ビルド経路は別 workflow (`build-msix-artifacts.yml`) で扱い、Partner Center 提出
> は別フェーズ。`release.yml` から SignPath 関連の step は撤去済
> (commit history を参照)。本ファイル末尾の「SignPath Foundation 一次申請の経緯」
> は方針変更前 (〜2026-07-24) の履歴として残す (読み物・後年の判断材料)。

---

## 配布経路と署名ポリシー (2026-07-25 現在)

| 配布経路                          | 形式          | Authenticode 署名              | SmartScreen           | リリース手段             |
| --------------------------------- | ------------- | ------------------------------- | --------------------- | ------------------------ |
| **Microsoft Store (正準・目標)**  | `.msix`       | Microsoft 自動署名 (Store 経由) | 即時信用              | Store 申請 (パートナー登録必要) |
| **GitHub Releases (当面)**        | `.msi` `.exe` | 無署名 (minisign は引き続き有効) | 警告出る (Run anyway) | `release.yml` 経由の tag push |

**正準の Authenticode 取得経路 = Store 経由の MSIX 自動署名**。これにより
SignPath / Certum / Trusted Signing 等の第三者 CA を個人が調達する必要はなくなる
(Microsoft Store のパートナー登録 + Identity Validation は必要だが、Trusted Signing
の Azure サブスクリプションや SignPath のプロジェクト審査よりは低コスト)。

GitHub Releases の NSIS / MSI 配布は当面 **無署名** を継続する (minisign 署名は
Tauri Updater 経由で引き続き付与され、アップデート改ざん防止は維持される)。SmartScreen
警告は README / リリースノート側で「More info → Run anyway」の案内文を提示する。

---

## Microsoft Store / MSIX 戦略

### なぜ MSIX か

- **自動署名**: ストア提出時に Microsoft が Authenticode 署名 + Publisher 証明書を
  自動付与し、SmartScreen 警告が即時消える。個人 CA 調達が不要。
- **現在確認済の前提** (`task.md` OPS3 / 2026-06-09 実機スパイク):
  - `runFullTrust` のみ = 仮想化され HKCU 書込が no-op になる (NO-GO)。
  - `unvirtualizedResources` + `RegistryWriteVirtualization=disabled` = 実 HKCU 到達確認。
  - 制限付き capability のためダブルクリック sideload 不可、PowerShell `Add-AppxPackage`
    または Store 経由のみ。
  - 結論 = 条件付き GO。Store 申請が前提の経路なら問題なし。
- **既存の土台**: `appusermodel.rs::is_msix_packaged()` 検出、AUMID 設定、Store 用
  アイコン、AppxManifest テンプレートが既に整備済 (OPS3 経緯)。

### パートナー登録要件 (概要)

- Microsoft Partner Center アカウント (個人開発者登録可、年会費なし)
- Identity Validation: 政府発行 ID による本人確認 (Certum / Trusted Signing と同種)
- Developer agreement / 税情報 / payout 口座登録
- 提出物の審査は 1〜数日、Rejection 時のフィードバック対応が必要

### CI 経路 (`build-msix-artifacts.yml` — 別 workflow)

`release.yml` とは独立した手動 trigger workflow。`makeappx` で NSIS / MSI 出力を
MSIX に詰め替え、`AppxManifest.xml` (`distribution/msix/`) を上書き、test 自己署名
証明書 (`CN=EasyCursorSwap-Dev`) で sign → `Add-AppxPackage` で CI runner に
install → sentinel HKCU write → cleanup のスモークを実施する。ARM64 実機 install
は runbook に「実機手順」として書き、ローカル完了条件から除外する。詳細手順は
別 commit (`build-msix-artifacts.yml` 新設) で実装予定 (Wave 4A)。

### MSIX 化での機能分岐

MSIX 環境 (`is_msix_packaged() == true`) では次の分岐が既存/予定されている:

- **Updater**: `tauri-plugin-updater` を無効化、Store 更新に委譲 (`AppConfig.general.auto_update` を MSIX では無視)
- **Health::attempt_rollback**: NSIS installer download 経路を無効化、release 通知のみ
- **Autostart**: `ShellExecuteW` の `tasks` URI またはレジストリ書込を禁止、Windows 設定 → スタートアップ アプリへの deep link を提示
- **AUMID / notification / single-instance / file association**: MSIX 環境で動作確認するスモークを runbook に追加

これらは Wave 4B (Store runtime 分岐) で実装予定。

---

## 候補比較 (方針変更後・参考)

| 方式                                | 年間費用      | OSS 無償 | EV / OV   | SmartScreen 即時信用          | 備考                                                                                                                          |
| ----------------------------------- | ------------- | -------- | --------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **Microsoft Store (MSIX 自動署名)** | 無償          | ✅       | OV (相当) | ✅ (即時)                     | **2026-07-25 採用方針**。パートナー登録 + Identity Validation のみ。                                                            |
| Microsoft Trusted Signing           | $9.99/月〜    | ❌       | OV (相当) | ✅ (即時)                     | 2026 年現在、個人開発者の新規 onboarding は一時停止                                                                              |
| Certum Open Source (SimplySign)     | 約 €29〜€69   | —        | OV        | ❌ (レピュテーション蓄積必要) | 暫定有料案として残すが、Store 戦略で代替可能なら優先度低                                                                          |
| DigiCert / Sectigo EV               | 約 $400〜500  | ❌       | EV        | ✅ (即時)                     | HSM/USB トークン必須、個人開発では運用コスト見合わず除外                                                                          |
| SignPath.io Foundation              | 無償          | ✅       | OV        | ❌                            | **再申請しない** (2026-07-25 方針)                                                                                              |
| 無署名 (GitHub Releases 当面の運用) | 0             | —        | —         | ❌                            | SmartScreen で「不明な発行元」警告。README / リリースノートで `Run anyway` 案内。minisign 署名は引き続き有効                      |

EV 証明書 (HSM 必須) は個人開発では運用コストが見合わないため除外。

---

## Microsoft Store / Partner Center セットアップ手順

1. <https://partner.microsoft.com/> で **Partner Center** アカウント作成 (Microsoft Account 必須)
2. Account settings → **Account type** で **Individual developer** を選択 (Organization ではない)
3. **Identity Validation** を完了 (政府発行 ID による本人確認、Certum / Trusted Signing と同等の審査)
   - パスポート / 運転免許証 / マイナンバーカード 等の提出、数営業日で承認
4. **Developer agreement** / 税情報 / payout 口座 を登録
5. **Apps and games → New app** で `EasyCursorSwap` を作成 (予約名は確保のみで OK)
6. 提出時に必要なもの:
   - MSIX パッケージ (`build-msix-artifacts.yml` 出力)
   - Store アイコン (44x44 / 150x150 / 310x150 等)
   - スクリーンショット (1366x768 以上、複数)
   - プライバシー ポリシー URL ([docs/code_signing_policy.md](code_signing_policy.md) の Privacy セクション)
   - アプリ説明 / カテゴリ / 価格設定 (無料)
7. 提出審査は 1〜数日。Rejection 時はフィードバック対応 → 再提出

---

## Microsoft Trusted Signing (将来検討・参考)

Microsoft Trusted Signing は Azure 経由の OV 相当サービス。**2026 年現在、個人開発者の
新規 onboarding は一時停止中**のため、Store 自動署名が解放されるまでの暫定選択肢として
残す。再開した場合のセットアップ概要 (再評価用メモ):

- Azure サブスクリプション作成
- Trusted Signing アカウント作成 (`Microsoft.CodeSigning` リソース プロバイダー)
- Identity Validation 申請 (個人 or 組織) — 数営業日で承認
- Certificate Profile 作成
- CI で `azure/trusted-signing-action@v0` を使う (Store 自動署名で代替可能なら優先度低)

参照: <https://learn.microsoft.com/en-us/azure/trusted-signing/>

---

## SmartScreen レピュテーション

OV / EV 署名でリリースした場合、初期は SmartScreen が「不明な発行元」警告を出す。
レピュテーションを蓄積するには (Store 経由の MSIX 自動署名では不要):

1. **同一証明書で継続的にリリースする** (毎リリース別証明書だとリセット)
2. **VirusTotal で検査して陰性であることを確認** ([index repo の validate.mjs の VT 統合](https://github.com/nishiuriraku/easy-cursor-swap-index/blob/main/scripts/marketplace/validate.mjs) は別物だが類似ツール)
3. **ユーザーに「実行」をクリックしてもらう** (累積数が SmartScreen 信用判定の母数)
4. 数百〜数千ダウンロード規模で警告が外れることが多い (Microsoft 非公開アルゴリズム)

### Microsoft Store 版 (2026-07-25 目標)

Store 経由で配布される MSIX は Microsoft が Authenticode 署名 + Publisher 証明書を
自動付与するため、SmartScreen 警告は **即時解消される**。README / リリースノートに
次の文面を提示する:

> ✅ このパッケージは Microsoft Store 経由で配布されており、Microsoft が
> Authenticode 署名を自動付与しています。Windows SmartScreen 警告は表示されません。

### GitHub Releases 版 (当面 / 無署名) の README / リリースノート文面

GitHub Releases で配布する NSIS / MSI は当面 **無署名** のため、SmartScreen 警告が
出る可能性がある。README / README.ja で次の案内文を提示する:

> ⚠️ Authenticode (Windows code signing) is **not yet provisioned** for the GitHub
> Releases installer. The canonical Authenticode-signed distribution is via the
> Microsoft Store (MSIX), where Microsoft automatically signs the package and
> SmartScreen warning disappears. Until the Store track is live, this NSIS / MSI
> installer is unsigned; Windows SmartScreen may show an "Unknown publisher"
> warning — click **More info → Run anyway** to proceed. Releases remain
> verifiable via the Tauri Updater's Ed25519 (minisign) signature; the source is
> MIT-licensed and built reproducibly in public GitHub Actions.

---

## チェックリスト (方針変更後の状態)

- [x] Authenticode 取得経路を Microsoft Store / MSIX に確定 (2026-07-25)
- [x] `release.yml` から SignPath step 群を撤去 (Wave 0A)
- [x] README / README.ja / `docs/code_signing_policy.md` / `docs/distribution.md` / `docs/release_procedure.md` を「再申請しない・Store 署名へ移行」に書き換え
- [x] `.env.example` から SignPath 関連項目を削除 (元から存在せず、念のため確認)
- [ ] Partner Center アカウント作成 + Identity Validation
- [ ] `build-msix-artifacts.yml` 新設 (Wave 4A — `makeappx` + test self-sign + CI install smoke)
- [ ] MSIX での runtime 分岐実装 (Wave 4B — Updater / Health / Autostart / AUMID / notification)
- [ ] Store 提出 (パートナー登録完了後・別フェーズ)

---

## 申請送信時の入力内容 (記録)

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

## SignPath Foundation 一次申請の経緯 (履歴)

> **方針変更 (2026-07-25) により、SignPath Foundation への再申請は行わない**。
> 以下は判断材料として残す (読み物・後年の判断材料)。

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

### 旧再申請ロードマップ (方針変更により不再適用・参考記録)

SignPath が見ている指標を意識的に蓄積する。目安は以下 (公式数値ではなく経験値)。
**Microsoft Store / MSIX 戦略を採用したため、本ロードマップは適用しない**。

| シグナル                  | 目標目安                       | 具体策                                                                                  |
| ------------------------- | ------------------------------ | --------------------------------------------------------------------------------------- |
| GitHub stars / forks      | star ≥ ~100, fork / contributor 複数 | Hacker News (Show HN) / Reddit (`r/windows`, `r/cursors`, `r/rust`, `r/tauri`, `r/Windows11`) でデモ投稿 |
| 独立した第三者の言及      | 複数の独立ソース               | **Zenn / Qiita / note / dev.to** に開発記事 ("Tauri で Windows カーソル管理アプリを作った")              |
| 動画 / SNS                | YouTube 短いデモ動画 1 本以上、X (旧 Twitter) で開発ログ | 30 秒のデモ動画、開発進捗の継続発信                                                     |
| 外部記事 / メディア露出   | 紹介系サイト 1〜2 件           | **窓の杜 / ITmedia / Forest** など Windows 向け OSS 紹介系メディアに情報提供メール       |
| ダウンロード実績          | 月間数百〜                     | GitHub Releases の **download count バッジ** を README に表示                            |
| 持続的な活動              | 数か月以上の継続コミット       | 定期的にバグ修正 / 機能追加リリースを継続                                                |

### 方針変更の判断理由 (2026-07-25)

1. **Microsoft Store / MSIX 自動署名で代替可能**: パートナー登録 + Identity
   Validation のコストは SignPath 再申請時の外部認知シグナル蓄積 (3〜6 か月 +
   認知度要件) より低コストかつ確実。
2. **個人 CA 調達 (Certum / Trusted Signing) の代替として**: Trusted Signing は
   2026 年現在個人 onboarding 停止中、Certum は年間 €29〜€69。Store 自動署名が
   即時 SmartScreen 信用を得る経路として最も安価。
3. **MSIX の制限 (restricted capability) は Store 経由では問題なし**: 実機
   スパイク (2026-06-09) で `unvirtualizedResources` + `RegistryWriteVirtualization=
   disabled` で実 HKCU 書込が成立することを確認済。Store 審査の制限付き capability
   扱いは Store 配布ならば許容範囲。

### 想定リスクと回避策 (再申請フェーズ・参考)

| リスク | 影響 | 回避策 |
|---|---|---|
| 再申請も同じ理由で保留 | 中 | star / 言及シグナルが目に見えて伸びてから出す。間隔を空ける |
| 認知度が伸びない | 中 | 上記ロードマップを淡々と実行。半年スパンで見る |
| 個別の有料 OV 切替が必要になる | 中 | Certum OSS が最安 (€29〜€69/年)。Trusted Signing は個人 onboarding 再開待ち |
| 配布物への信頼質問が増える | 中 | README / Wiki FAQ で「未署名 + Updater minisign 署名は有効 + 再現可能ビルド」を明示済 |
