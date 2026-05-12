# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

EasyCursorSwap (`package.json` name: `easy-cursor-swap`) — a Windows-only desktop app for managing custom mouse cursor themes. Tauri v2 + Nuxt 4 + Rust hybrid. The project lives at the repo root (no `easy-cursor-swap/` subdirectory).

- **Target:** Windows 10 22H2+ / Windows 11, x64 (ARM64 planned)
- **Frontend:** Nuxt 4 / Vue 3 (SPA, Composition API)
- **Backend:** Rust 1.82+ (`windows`, `winreg`, `image`, `tracing`, `ed25519-dalek`)
- **Distribution:** NSIS / MSI installers signed via SignPath; Tauri Updater with Ed25519-signed releases

See `README.md` and the documentation map below for full context.

## Documentation map

`docs/` holds three different kinds of documents. Don't confuse their roles or update policies.

| File | Role | Update policy |
|---|---|---|
| `docs/first_plan.md` | **Frozen requirements (planning-phase source)** — the original brain-dump that fixed everything: licensing model, supported OS matrix, the 17 cursor roles, required `theme.json` fields, security thresholds, performance targets, v1.0 MVP scope. Captures the *original intent* in full. | **Do not rewrite.** When the spec changes, log the change in `implementation_plan.md` instead. Treat `first_plan.md` as the source-of-truth for "what was originally decided." |
| `docs/implementation_plan.md` | **Living implementation plan** — Phase 1–9 task tables + progress checklists derived from `first_plan.md`, updated commit by commit. The "次回セッションでの優先タスク" and "ユーザー手動が必要な残作業" sections are the de-facto roadmap. | **Updated every commit** under the one-feature = one-commit rule. Tick boxes when a phase finishes; append new findings to "既知の問題" or "次回セッションでの優先タスク". |
| `docs/01_specification.md` – `04_implementation_guide.md` | **Tidied chapter version** — `first_plan.md` re-organised into 4 chapters (spec / architecture / security / implementation guide) for easier reading. | Sync only on large spec changes. Day-to-day, `implementation_plan.md` is enough. |
| `docs/code_signing.md` / `signing.md` / `distribution.md` / `key_rotation.md` / `author_registration.md` | **Operational runbooks** — code-signing procurement, Updater key issuance, MSIX distribution, key-rotation PR procedure, new-author registration. | Update only when the procedure itself changes. |
| `docs/superpowers/` | **Per-feature work log** — design docs, plans, and follow-up issues for individual features, accumulated chronologically (e.g. `2026-05-07-creator-bulk-import-design.md`). | Add a new dated file per feature; keep old ones as history. |

When documents disagree, **treat `implementation_plan.md` as authoritative** — it travels with the code. Where it diverges from `first_plan.md`, the divergence is intentional (a deliberate change from the original plan); don't silently roll it back to match `first_plan.md`.

## Commands

Run all commands from the repo root.

```bash
# Frontend (Nuxt) only
npm run dev                  # Nuxt dev server on :3000
npm run build                # Nuxt build (static SPA, output to .output/public/)

# Tauri full app
npm run tauri:dev            # Tauri dev window + Nuxt HMR
npm run tauri:build          # Production build → src-tauri/target/release/bundle/

# Frontend tests (Vitest + happy-dom; tests live in app/**/__tests__/*.test.ts)
npm test                     # vitest run
npm run test:watch           # vitest watch
npx vitest run path/to/file.test.ts   # single file

# Type check (CI uses this; no separate npm script)
npx vue-tsc --noEmit

# i18n parity (CI fails if ja.ts / en.ts keys diverge)
node scripts/check-i18n.mjs

# Rust (operate inside src-tauri/ or via --manifest-path)
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --lib xp_logic::tests::name_of_test  # single test
cargo bench --manifest-path src-tauri/Cargo.toml                                     # criterion benches
```

### Verification gate (run before committing — see auto-memory note)

**コミット前は必ず `scripts/verify-gate.sh` を実行する。** このスクリプトが正準の検証ゲートで、内訳は以下:

```bash
bash scripts/verify-gate.sh
# 内訳:
#   cargo fmt --check / cargo clippy -D warnings / cargo test --lib
#   prettier --check / vue-tsc --noEmit
#   node scripts/check-i18n.mjs / npm test (vitest)
```

ゲート手順を変更したい場合は、CLAUDE.md ではなく `scripts/verify-gate.sh` を直接編集すること (CI と挙動を揃える)。インストーラまで含めて検証する場合は `npm run tauri:build` を追加で実行する。

## Architecture

```
Vue (UI) ──invoke()──▶ Tauri command (commands.rs) ──▶ Rust module ──▶ Windows registry / FS
```

**Rust is the single source of truth.** Frontend state must be synced via IPC; never persist app state only on the Vue side.

### Frontend layout (`app/`)

- `pages/` — `index.vue` (Library), `creator.vue`, `marketplace.vue`, `settings.vue`, `appearance.vue`
- `components/{shell,library,creator,marketplace,settings,panic,icons}/` — domain-grouped SFCs. `nuxt.config.ts` sets `pathPrefix: false`, so reference components by file name (`<ThemeCard>`, not `<LibraryThemeCard>`).
- `composables/` — `useThemes`, `useAppSettings`, `useI18n`, `useTauri` (IPC wrapper), `useKeystore`, `useUiTheme`, `useRoleMatcher`, `useThemePreviews`, `useBulkImport`, `useUpdater`, `useNotify`, `sanitizeSvg`. Vitest specs in `app/composables/__tests__/`.
- `locales/{ja,en}.ts` — keys typed `as const`; **must stay in parity** (CI gate via `scripts/check-i18n.mjs`).
- `types/` — IPC payload types (`config.ts`, `theme.ts`, `marketplace.ts`).
- `assets/css/tailwind.css` — Tailwind v4 entry + `@theme` ブロック (design tokens を utility に露出) + 横断 shared utility (`.btn` / `.card` / `.chip` / `.input` / `.tag` / `.toolbar` / `.tabs` / `.prop-section` / `.lib-table` / `.lib-row` / `.lt-*` / `.modal*` / `.content` / `.page-head` / `.grid` ほか)。
- `assets/css/global.css` — `:root` + `html.light` design tokens、CSS リセット、スクロールバーカスタマイズ、`:focus-visible`、`prefers-reduced-motion`、共有 `@keyframes` (pulse / fade-in / slide-in-right / spin) のみ。コンポーネント固有のスタイルは追加しない。

### Backend layout (`src-tauri/src/`)

22 modules registered in `lib.rs`. Grouped by responsibility:

| Concern | Modules |
|---|---|
| IPC surface | `commands.rs` (≈25 commands; the contract between Vue and Rust) |
| Config / state | `config.rs` (RwLock + schema migration + backups, Source of Truth), `errors.rs` |
| Cursor pipeline | `cursor.rs` (PNG/SVG → `.cur`, 6 sizes, Lanczos, hotspot), `cursor_watcher.rs` |
| Registry | `registry.rs` (`HKCU\Control Panel\Cursors`, `Schemes`, `SPI_SETCURSORS`) |
| Theme packages | `theme.rs` (`.cursorpack` import/export), `bulk_import.rs`, `backup.rs` (`.cursorprofile`) |
| Marketplace | `marketplace.rs` (HTTP index fetch, SHA-256 + Ed25519 verify), `keystore.rs` (Ed25519 + DPAPI) |
| Reliability | `health.rs` (startup-failure counter + rollback), `crash.rs` |
| OS integration | `tray.rs`, `darkmode.rs`, `hotkey.rs`, `autostart.rs`, `single_instance.rs` (Named Mutex), `appusermodel.rs`, `accessibility.rs`, `environment.rs` (RDP/Citrix detection) |
| Observability | `logging.rs` (with `redact_path` / `short_hash` PII helpers) |

### Critical invariants

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional.** `registry.rs` writes a snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating, deletes it on success. On startup, a leftover snapshot triggers auto-rollback. There is also `_initial_snapshot.json` saved on first run; the panic button (`Ctrl+Alt+Shift+R`) restores either Windows defaults or this initial snapshot.
- **Cursor files live in `~/.custom_cursors/`** so they survive uninstall.
- **PII redaction in logs.** When using `tracing!`, route raw paths through `logging::redact_path` and hashes through `logging::short_hash` (12 chars). No raw registry values, no full SHA-256 in logs.
- **Archive sanitisation.** Any code unzipping `.cursorpack` / `.cursorprofile` must go through `theme::sanitize_archive_path` and the size limits (50 MB compressed / 200 MB expanded / 10 MB per image / 1 GB total user storage).
- **No `v-html`** anywhere in Vue — SVG icons go through render functions in `UiIcon.vue` / `CursorIcon.vue`.
- **`routeRules: { '/**': { ssr: false } }`** in `nuxt.config.ts` is intentional. Nuxt 4.4.4 has an IPC bug with `ssr: false` at the top level; do not change to `ssr: false` directly.

## Coding conventions

- Rust comments and doc strings: **Japanese**.
- Vue: SFC + `<script setup>` + Composition API + TypeScript.
- CSS: **Tailwind v4 utility classes** as the default styling mechanism.
  - Design tokens live in `app/assets/css/tailwind.css` (`@theme` block) aliasing legacy `--*` tokens.
  - **横断 shared utility** (`.btn`, `.card`, `.chip`, `.input`, `.tag`, `.toolbar`, `.tabs`, `.prop-section`, `.lib-row`, `.lt-*`, `.modal*`, `.content`, `.page-head`, `.grid` ほか) は `app/assets/css/tailwind.css` の top-level (unlayered) に定義。`@layer components` には**入れない** — Tailwind preflight (`button { color: inherit }` 等) が unlayered で出力されるため、`@layer` 内のルールは cascade で負ける。
  - **コンポーネント固有のスタイル**は各 `.vue` ファイルの `<style scoped>` に置き、`@reference '~/assets/css/tailwind.css';` を冒頭で宣言してから `@apply` を使う。
  - `app/assets/css/global.css` は `:root` トークン、CSS リセット、スクロールバーカスタマイズ、`:focus-visible`、`prefers-reduced-motion`、共有 `@keyframes`、`html.light` トークン上書きの**みに限定**。コンポーネント固有スタイルをここへ追加しない (Phase 10-12 で 3327 行 → 223 行に収束済み)。
- IPC payload types live in `app/types/` and must mirror `serde`-derived Rust structs in `commands.rs` / module files.
- Filenames in `components/` are referenced without directory prefix — keep names globally unique.

## Implementation policy

新規機能・リファクタ・バグ修正に着手するときは、以下を**必ず**この順で実施する。

1. **必要な skill を使用する。** タスクに該当しそうな skill が 1% でもあれば `Skill` ツールで起動する (例: ブレストは `superpowers:brainstorming`、TDD は `superpowers:test-driven-development`、デバッグは `superpowers:systematic-debugging`、Rust の所有権/並行性エラーは `rust-skills:m01-ownership` 等、Cloudflare/Tauri/Nuxt 固有作業は対応 skill)。判断に迷ったら起動して、合わなければ捨てる。
2. **既存の実装を確認してから書く。** 触る領域の `app/` / `src-tauri/src/` / `composables/` / `commands.rs` を先に読み、命名・型・IPC の前例に揃える。`scripts/check-i18n.mjs` が落ちないよう `locales/{ja,en}.ts` 双方を更新。重複コードを新設せず、既存の composable / module を拡張する方を優先。
3. **検証ゲートは `scripts/verify-gate.sh` を使う。** コミット直前に `bash scripts/verify-gate.sh` を実行し、緑になることを確認する。インライン版 (`cargo fmt` 〜 `npm run tauri:build`) は使わない — スクリプトが正準。

## Workflow rule (auto-memory)

One feature = one commit. Run `bash scripts/verify-gate.sh` and confirm it passes before committing.

## CI workflows

- `.github/workflows/ci.yml` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --lib`, `vue-tsc --noEmit`, i18n parity.
- `.github/workflows/performance.yml` — Criterion benches (`benches/cursor_build.rs`, `benches/startup.rs`); regression-detected on PRs.
- `.github/workflows/release.yml` — signed installer builds.

Marketplace 投稿の検証 (`.cursorpack` の SHA-256 / Ed25519 / サイズ / マルウェア DB) は別リポジトリ [`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) 側の `scripts/marketplace/validate.mjs` と `marketplace-validate.yml` ワークフローで行う (Ajv 版が正準)。

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- Do **not** scaffold new features under an `easy-cursor-swap/` subdirectory; the workspace `CLAUDE.md` (`<USER_HOME>\Workspace\CLAUDE.md`) lists the project that way for historical reasons, but the actual repo root is `cursor-forge/`.
- `npm run dev` (Nuxt only) is fine in isolation, but Tauri commands will fail because there's no Tauri runtime. Use `npm run tauri:dev` to exercise IPC.
