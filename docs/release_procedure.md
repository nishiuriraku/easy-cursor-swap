# リリース手順 (version bump → tag push)

EasyCursorSwap の v0.0.X リリース運用 runbook。`develop` に蓄積した変更を `main` に
反映し、annotated tag を打って `release.yml` を起動するまでの一連の手順を定義する。

> **タグ発行後の手順** (Tauri Updater Ed25519 署名 / SignPath 経由の Authenticode 署名 —
> ただし 2026-05-21 時点で SignPath Foundation 一次審査は保留中につき SignPath
> ステップは SIGNPATH_* secret 未設定で自動 skip される) は
> [`authenticode_signing.md`](./authenticode_signing.md) と
> [`updater_signing.md`](./updater_signing.md) を参照。本 runbook はタグ push までの
> リポジトリ側作業に範囲を限定する。

---

## いつ実施するか

- `develop` に新機能 / バグ修正 / 依存更新が複数蓄積し、ユーザーに配布する区切りに達したとき
- セキュリティ修正で即時リリースが必要なとき (緊急)
- 仮リリース系列 (`0.0.x`) 中は 1〜2 週間程度のケイデンスを目安に

緊急リリース時も手順自体は変えず、`verify-gate.sh` だけは必ず通すこと
(invariant 退行を最も低コストで検知できるレイヤ)。

---

## バージョン更新箇所

`v0.0.X` の version 文字列は **3 マニフェスト + 2 ロックファイル** に分散している。
すべて同期させる:

| ファイル | 更新方法 | 備考 |
|---|---|---|
| `package.json` | 手動編集 | `"version": "0.0.X"` |
| `src-tauri/Cargo.toml` | 手動編集 | `[package] version = "0.0.X"` |
| `src-tauri/tauri.conf.json` | 手動編集 | `"version": "0.0.X"` (`productName` の直下) |
| `src-tauri/Cargo.lock` | `cargo check --manifest-path src-tauri/Cargo.toml` で再生成 | 直接編集禁止 |
| `package-lock.json` | `npm install --package-lock-only` で再生成 | 直接編集禁止 |

3 マニフェストどれか 1 つでも忘れると、サイドバーや Cargo 内部で表示される version が
drift する (`useAppInfo` 経由でフロントが Rust 側 version を読むため、Tauri 側だけ更新で
UI には反映されるが、npm パッケージとしての meta が古いままになる)。

---

## 手順

### 1. リリース対象コミットの確認

```bash
git fetch origin
git log --oneline --no-merges origin/main..origin/develop
```

ここで出てくる全コミットが CHANGELOG に反映される対象。

### 2. release ブランチを切る

> **通常ケース** (`main` が `develop` の最新まで進んでいる、または `develop` のみが
> 進んでいて release PR でまとめてマージしたい場合)

```bash
git fetch origin main
git checkout -b release/v0.0.X origin/main   # main 起点
# または
git checkout -b release/v0.0.X develop        # develop 起点 (どちらでも結果は同じ)
```

> **特殊ケース**: develop → main の別 PR が **オープン中** に release ブランチを
> 切ってしまった場合、PR マージ後にブランチ起点をやり直す必要がある。
> 未コミット変更がある状態では `git rebase` が走らないため、stash 経由で起点を
> 振り替える:
>
> ```bash
> git stash push -u -m "wip release bump"
> git fetch origin main
> git reset --hard origin/main
> git stash pop
> ```

### 3. バージョン同期

3 マニフェストを手動で `0.0.(X-1)` → `0.0.X` に書き換え、ロックファイルを再生成:

```bash
# (3 マニフェストを編集)

# Cargo.lock を再生成
cargo check --manifest-path src-tauri/Cargo.toml

# package-lock.json を再生成
npm install --package-lock-only

# diff を目視確認 — 5 ファイルだけが変更されているはず
git diff --stat
```

### 4. CHANGELOG.md 更新

`[Unreleased]` セクションの **直後** に新エントリを挿入する形で書く
(`[Unreleased]` のヘッダ自体は残し、中身を新セクションへ移動)。

> **v0.0.4 から** provisional (pre-release) 期を卒業し、ヘッダから
> `(pre-release)` を外す。`release.yml` の `prerelease: false` 切替と同じ判断。
> v0.0.3 以前は `## [0.0.X] - YYYY-MM-DD (pre-release)` 形式だったため、
> 履歴の整合は崩さない (過去エントリは触らない)。

```markdown
## [Unreleased]

## [0.0.X] - YYYY-MM-DD

(1〜2 行のサマリ。「何が中心の release か」を一文で。
release.yml が CHANGELOG.md の該当セクションを GitHub Release ページに
そのまま埋め込むため、サマリと各 ### Added/Changed/... 配下の本文は
ユーザー向けの読み物として書く。)

### Added
- ...

### Changed
- ...

### Fixed
- ...

### Internal
- (依存更新 / リファクタ / docs / 内部整理はここ)
```

ファイル末尾の comparison link も追加 / 更新:

```markdown
[Unreleased]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.X...HEAD
[0.0.X]: https://github.com/nishiuriraku/easy-cursor-swap/compare/v0.0.(X-1)...v0.0.X
```

エントリ作成のためのコミット一覧取得:

```bash
git log --oneline --no-merges v0.0.(X-1)..HEAD
```

抽出テスト (release.yml と同じ helper を手元で実行して中身を確認):

```bash
node scripts/release/extract-changelog-section.mjs v0.0.X
```

### 5. 検証ゲート

```bash
bash scripts/verify-gate.sh
```

ALL GREEN になるまで何度でも回す。`CHANGELOG.md` は docs-only commit のスキップ対象に
含まれているが、**release commit は invariant 退行検知のため必ず gate を通す**。

### 6. コミット → push → PR (target: main)

```bash
git add CHANGELOG.md package.json package-lock.json \
        src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json

git commit -m "chore(release): bump version to v0.0.X

(本文に主な変更点と verify-gate 結果を記載)
"

git push -u origin release/v0.0.X

gh pr create --base main --head release/v0.0.X \
  --title "chore(release): bump version to v0.0.X" \
  --body "..."
```

正しい状態の PR は **6 ファイル / +数十行 / -数行のクリーンな diff**。それ以外の
ファイルが diff に出ていたら起点ブランチが間違っているので 2 番からやり直す。

### 7. PR マージ確認

```bash
gh pr view <PR番号> --json state,mergedAt,mergeCommit
```

`state: "MERGED"` を確認してから次へ進む (mergeCommit の oid をメモしておく)。

### 8. annotated tag を打つ

`v0.0.1` / `v0.0.2` と同じ annotated tag (メッセージ付き) を、PR の merge commit に
対して作成する:

```bash
git fetch origin main
# ローカル develop も最新化しておく (releaseブランチが develop 起点だった場合)
git checkout develop && git pull --ff-only origin main

git tag -a v0.0.X <merge-commit-sha> -m "EasyCursorSwap v0.0.X

(主な変更を 2〜3 行で要約。SemVer 0.0.x の API 安定保証は
依然 best-effort だが、配布形態は stable release として扱う。)

詳細は CHANGELOG.md の [0.0.X] - YYYY-MM-DD セクションを参照。

GitHub Release は releaseDraft: true / prerelease: false でドラフト作成
される。release.yml が CHANGELOG.md の該当セクションを releaseBody に
注入するため、ドラフトのリリースノートは編集不要なケースが多い。
公開は手動レビュー後。"
```

タグが正しい commit を指していることを確認:

```bash
git show v0.0.X --no-patch
git cat-file -p v0.0.X | head -5   # 1 行目に object <merge-commit-sha> が出る
```

### 9. タグ push → release.yml 起動

```bash
git push origin v0.0.X
```

`release.yml` が tag push をトリガーに起動する。実行状況の確認:

```bash
gh run list --workflow=release.yml --limit 3
```

実績では完走まで ~13 分 (v0.0.2 計測)。

### 10. GitHub Release を手動 publish

1. <https://github.com/nishiuriraku/easy-cursor-swap/releases> で `v0.0.X` の draft を確認
2. **v0.0.4 以降は通常リリース** (`Pre-release` チェックは OFF のまま)。`release.yml` 側でも `prerelease: false` がデフォルト。v0.0.3 以前は ON にしていたが provisional 期を卒業。
3. リリースノートを確認 — `release.yml` の `Extract CHANGELOG section` step が `CHANGELOG.md` の該当 `[0.0.X]` セクション本文を自動で `releaseBody` 冒頭に挿入する。インストール / 動作要件 / 既知の制限 は `release.yml` の静的テンプレートとして末尾に続く。CHANGELOG 側で書いた内容がそのまま見えるので、編集が必要なのは追加スクショ / GIF を貼る場合だけ。
4. **Publish release** を押下
5. Tauri Updater が新版を自動配信開始

---

## ハマりポイント

| 罠 | 回避策 |
|---|---|
| version を `package.json` だけ更新して `Cargo.toml` / `tauri.conf.json` を忘れる | 3 マニフェストを必ずチェックリスト化。UI 表記が drift する |
| Lockfile を手動編集してしまう | 必ず `cargo check` / `npm install --package-lock-only` で再生成 |
| release ブランチを `develop` から切ったが `main` の方がまだ進んでなかった (PR 順序問題) | stash → reset → pop パターンで起点を `main` HEAD に揃え直す |
| lightweight tag を打ってしまう | `git tag -a` (annotated) を **必ず** 使う。`-a` を忘れると `git describe` の解釈や release.yml の挙動が安定しない |
| 間違ったコミットにタグを打つ | push 前に `git cat-file -p v0.0.X` で `object <sha>` を目視確認 |
| PR マージ前にタグを打ってしまう | `gh pr view <N> --json state` で `MERGED` を確認してから |
| CHANGELOG の comparison link を更新し忘れる | ファイル末尾の `[Unreleased]: .../v0.0.(X-1)...HEAD` を `v0.0.X...HEAD` に書き換え、新たに `[0.0.X]: .../compare/v0.0.(X-1)...v0.0.X` を追加 |
| docs-only commit と勘違いして gate をスキップ | `CHANGELOG.md` は規約上 docs-only に含まれるが、**release commit は安全側に gate を通す** のが慣例 |
| `npm run tauri:build` を回さずに PR を作る | ローカル installer build は CI で代替可能だが、Authenticode 署名前の sanity check として手元で 1 回回しておくと安全 (現状 SignPath 申請保留中につき CI でも Authenticode は付与されない) |

---

## 不変条件 audit (release 都度)

PR 本文に明記する。`v0.0.2` PR と `v0.0.3` PR の前例に倣う:

| Invariant | 確認方法 |
|---|---|
| HKCU only (no HKLM / UAC) | `git diff origin/main..HEAD -- src-tauri/src/registry/` に HKLM への参照が無いこと |
| Apply is transactional | `registry/mod.rs` の snapshot → mutate → delete 順が変わっていないこと |
| PII redaction in logs | `logging::redact_path` / `logging::short_hash` の使用箇所が減っていないこと |
| Archive sanitisation | `theme::sanitize_archive_path` 経由しない unzip が増えていないこと |
| No `v-html` | `grep -r "v-html" app/` で 0 件 |

---

## 関連 runbook

- [`authenticode_signing.md`](./authenticode_signing.md) — Authenticode 署名 (SignPath / Certum / EV / OV) のセットアップ、一次審査の結果と再申請ロードマップ
- [`updater_signing.md`](./updater_signing.md) — Tauri Updater の Ed25519 署名鍵管理
- [`distribution.md`](./distribution.md) — `.msi` / `.nsis` / `.msix` の生成と配布形態
- [`code_signing_policy.md`](./code_signing_policy.md) — 署名方針と OS 検証フロー
- [`key_rotation.md`](./key_rotation.md) — 公式インデックスの著者鍵ローテーション (リリース無関係だが鍵運用の同類)
