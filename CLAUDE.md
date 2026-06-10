# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Domain-specific guidance lives in nested files (auto-loaded when working under those dirs):

- `app/CLAUDE.md` — Frontend (Nuxt 4 / Vue 3 / Tailwind v4).
- `src-tauri/CLAUDE.md` — Backend (Rust 1.82+, Tauri v2).

## Project

EasyCursorSwap (`package.json` name: `easy-cursor-swap`) — a Windows-only desktop app for managing custom mouse cursor themes. Tauri v2 + Nuxt 4 + Rust hybrid. The project lives at the repo root (no `easy-cursor-swap/` subdirectory).

- **Target:** Windows 10 22H2+ / Windows 11, x64 (ARM64 planned)
- **Distribution:** NSIS / MSI installers (Authenticode signing pending — SignPath Foundation OSS application was deferred 2026-05-21 for insufficient external visibility; `release.yml` SignPath step is wired and skip-guarded so it activates automatically when SIGNPATH\_\* secrets are configured after reapproval); Tauri Updater with Ed25519-signed releases (active)

## Documentation map

**All living state docs live in the Obsidian vault** (`<USER_HOME>\Workspace\Obsidian`, under `develop/easy-cursor-swap/`). Repo `docs/` keeps **operational runbooks only**. 3-layer model (2026-05-29 redesign v2):

| Layer                                          | What                                                                                                                                                                                   | Who reads it                                                                                                     |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **Layer1 正準 (markdown, hand-written)**       | `specs/<NN-slug>/<NN-slug>.md` (14 feature specs, folder-note form) + `shared/*.md` (7 cross-cutting) + `reference/{backend-overview,frontend-overview,ipc-catalog,file_inventory}.md` | **Agents grep these first.** Each spec's frontmatter carries `ipc:` / `modules:` / `invariants:` ownership maps. |
| **Layer2 人間ナビ**                            | `bases/*.base` + `overview.canvas` + Obsidian Graph View + `index.md`                                                                                                                  | Humans. **AI must NOT read `.canvas` / `.base`** (frontmatter projections, redundant with Layer1 markdown).      |
| **Layer3 機械マニフェスト (生成・手書き禁止)** | `reference/index.json` (measured_counts + module/IPC/invariant → note maps) + `reference/ui-map.json` (197 UI interactions, grep 用)                                                   | Tools (SessionStart hook / verify-gate). `scripts/gen-architecture.mjs` が出力。                                 |

**Operational runbooks** (procedure-only, in repo `docs/`): `docs/release_procedure.md` / `updater_signing.md` / `authenticode_signing.md` / `distribution.md` / `key_rotation.md` / `author_registration.md` / `code_signing_policy.md`.

When documents disagree, the **Layer1 markdown** is canonical; `reference/index.json` is a generated projection (never hand-edit it). The Tier 1/2/3 system was abolished 2026-05-28; `architecture.json` and the narrative `architecture.md` + HTML viewers were retired (architecture.json on 2026-05-29 in redesign v2, replaced by Layer1 markdown + generated `index.json`). Human visual = `overview.canvas` + Graph + Bases.

> Design history — the original-plan documents (formerly `docs/legacy/`) and per-feature work logs (formerly `docs/superpowers/`) — was moved to the Obsidian vault on 2026-05-28: `develop/easy-cursor-swap/legacy/` and `…/superpowers/`. Both were always git-untracked / removed from history before v0.1.0.

## Critical invariants (cross-cutting)

These apply regardless of which side you're working on. Full list (including module-specific ones) is in the Obsidian vault `develop/easy-cursor-swap/shared/invariants.md` (Layer1 canonical home; `invariants:` frontmatter lists all ids).

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional (2 recovery paths).** `registry/mod.rs` writes a snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating, deletes it on success. (a) **In-process write failure** → `restore_from_snapshot` rolls the registry back to the **pre-apply values** (exact restore). (b) **Leftover snapshot on startup** → means a previous apply was interrupted (likely a crash); since the registry may be in a mixed/partial state, `reset_to_windows_default` resets to **Windows default — NOT the pre-apply values** (intentional safety choice to avoid a mixed state, not a bug). `_initial_snapshot.json` (first-run) is restored by the panic button (`Ctrl+Alt+Shift+R`).
- **Cursor files live in `~/.custom_cursors/`** so they survive uninstall.
- **PII redaction in logs.** Raw paths via `logging::redact_path`, hashes via `logging::short_hash` (12 chars). No raw registry values, no full SHA-256.
- **Archive sanitisation.** Any code unzipping `.cursorpack` / `.cursorprofile` must go through `theme::sanitize_archive_path` and the size limits (50 MB compressed / 200 MB expanded / 10 MB per image / 1 GB total user storage).
- **No `v-html`** anywhere in Vue. SVG icons go through render functions in `UiIcon.vue` / `CursorIcon.vue`.
- **Rust is the single source of truth.** Frontend state must be synced via IPC; never persist app state only on the Vue side.
- **IPC payload types** in `app/types/` must mirror `serde`-derived Rust structs in `src-tauri/src/commands/`.

## Commands

Run from the repo root.

```bash
npm run dev             # Nuxt-only dev server (IPC will fail — use tauri:dev to exercise IPC)
npm run tauri:dev       # Tauri dev window + Nuxt HMR
npm run tauri:build     # Production build → src-tauri/target/release/bundle/
npm test                # Vitest run (frontend)
npx vue-tsc --noEmit    # Frontend type check
node scripts/check-i18n.mjs    # i18n parity (ja.ts vs en.ts) — CI gate
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

### Verification gate (canonical — run before every commit)

```bash
bash scripts/verify-gate.sh
```

Edit `scripts/verify-gate.sh` directly to change gate steps (do **not** re-document them here or in the vault docs). To validate installer builds as well, additionally run `npm run tauri:build`.

**Exception — docs-only commits skip the gate.** If a commit touches only `CLAUDE.md` (root or sub-dirs) / `README*.md` / anything under `docs/` / `CHANGELOG.md` / community markdown (`SUPPORT.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md` / `CONTRIBUTING.md`), and nothing under `app/`, `src-tauri/`, `scripts/`, `.github/`, `package.json`, `nuxt.config.ts`, the gate is not required. Mark with `docs:` Conventional Commit prefix.

## Implementation policy

When starting any new feature, refactor, or bug fix, always follow these steps in order:

1. **Invoke the relevant skill** via the `Skill` tool if there's even a 1% chance one applies (e.g. `superpowers:brainstorming`, `superpowers:test-driven-development`, `superpowers:systematic-debugging`, `rust-skills:m01-ownership`).
2. **Read the canonical docs before writing** — the relevant Layer1 markdown in the Obsidian vault (`develop/easy-cursor-swap/specs/<NN-slug>/<NN-slug>.md` + `shared/*.md` + `reference/*.md`); the agent SessionStart snapshot already injects counts from `index.json`. Do NOT read `overview.canvas` (human visual aid). Read `reference/file_inventory.md` only when per-file detail is needed. Follow `file` pointers down to real sources and match existing conventions. Update `locales/{ja,en}.ts` in parity. Prefer extending an existing composable / module over duplication.
3. **Run `bash scripts/verify-gate.sh`** right before committing and confirm green.
4. **(UI を変更したとき) `tauri-visual-review` スキルで実機レビュー** — `npm run tauri:dev` を起動し Tauri MCP Bridge (port 9223) 経由で「動作確認 (起動 / コンソールエラー / 主要フロー / IPC 応答) + ビジュアルリグレッション (`git stash` + HMR で before/after を構造シグネチャ diff) + 視覚バグ批評 (スクショ目視)」を行い、`C:\tmp\ecs-visual-review\<run-id>\report.md` に出す (`/visual-review` でも起動可)。**助言であってゲートではない** — 実機・ディスプレイ・debug ビルドが要るので `verify-gate.sh`/CI には含めない。Rust のみ / 設定変更でフロントの見た目に影響しないなら省略可。スキル実体はローカル (`~/.claude/skills/tauri-visual-review/`、git 非追跡) なので、フローの正準記録はこの CLAUDE.md 側に置く。
5. **Update docs in the same commit** (see policy below). Code-only commits that move source-of-truth without touching living docs are the main cause of doc rot.

### Where new design work lands (development loop)

The `superpowers/` and `legacy/` vault dirs are **frozen design history** (now excluded from Graph View / search) — do **not** add new files there. Capture ongoing work like this instead:

- **While thinking / brainstorming** → append to the daily log `develop/easy-cursor-swap/log/YYYY-MM-DD.md` (one rolling file per day under the `log/` dir, via `obsidian create`/`append`). This replaces the old per-feature `superpowers/{plans,specs}/` 3-files-per-feature pattern.
- **Once a design is confirmed** → promote the conclusion into the relevant `specs/<feature>/spec.md` 設計判断 block (the spec is the living home; the log is the scratchpad).
- **Follow-ups / bugs / refactors discovered** → file in `develop/easy-cursor-swap/task.md` with a priority mark and a `[[specs/...]]` backlink.
- **History stays in git** — don't hand-write changelogs of what you did into the vault; the commit message is the record.

## Documentation update policy

Living docs must move with the code. Triggers and required updates:

- **New / renamed / removed Rust file** → `reference/backend-overview.md` (if infra: main/lib/errors/config/logging/cancel_registry) OR the owning `specs/<NN-slug>/<NN-slug>.md` `modules:` frontmatter + `reference/file_inventory.md`.
- **Added / removed `#[tauri::command]`** → update the owning `specs/<NN-slug>/<NN-slug>.md` `ipc:` frontmatter + `reference/ipc-catalog.md` table + `reference/file_inventory.md`. The **count** is owned by `node scripts/gen-architecture.mjs` (regenerates `reference/index.json` `measured_counts`). Drift is gated by `verify-gate.sh` (`gen-architecture.mjs --check`).
- **Module split / merge** → the owning `specs/<NN-slug>/<NN-slug>.md` `modules:` frontmatter + `reference/backend-overview.md` + `reference/file_inventory.md`.
- **Startup sequence change** (`main.rs`) → `reference/backend-overview.md` startup sequence section.
- **New security invariant** → `shared/invariants.md` (canonical home, `invariants:` frontmatter lists all ids) + the owning spec's `invariants:` frontmatter + README "Security Model" if user-visible.
- **New / changed Vue page, composable, or component sub-directory** → `reference/frontend-overview.md` + `reference/ui-map.json` + the relevant `frontend/<page>.md` spec + `reference/file_inventory.md`.
- **Tailwind / global CSS pattern change** → `app/CLAUDE.md` CSS subsection.
- **Verification gate change** → `scripts/verify-gate.sh` only.
- **Operational procedure change** → the corresponding runbook in `docs/`.
- **User-visible behaviour / install flow / supported OS / security model change** → both `README.md` and `README.ja.md` in parity, plus `CHANGELOG.md` under `## [Unreleased]` with the right Keep-a-Changelog section.
- **Any numeric / enumerated claim drift** — run `node scripts/gen-architecture.mjs`. It re-measures from source (`pub mod` in `lib.rs`, the `generate_handler![]` list in `commands/mod.rs`, composable/page/component globs, CI workflows) and regenerates `reference/index.json` `measured_counts` + `generated_at` from code + frontmatter ownership maps. It also reports name-level module/IPC drift (Layer1 markdown you must update by hand). `reference/index.json` `measured_counts` is the sole numeric home; `README*.md` only mentions counts in prose — fix those in the same commit if they reference a changed number.
- **Any change touching `reference/file_inventory.md`** → run `node scripts/gen-architecture.mjs` to refresh `reference/index.json` `measured_counts` + `generated_at`; update narrative drift warnings by hand if relevant. If the high-level structure changed, also hand-adjust `overview.canvas`. (Narrative `architecture.md`, the JSON `architecture.json`, the HTML viewers, and the `scripts/embed-arch-json.mjs` step were all retired — `architecture.md` and HTML viewers 2026-05-28, `architecture.json` on 2026-05-29 in redesign v2.)

**Doc-only commits** are fine as standalone — use the `docs:` Conventional Commit prefix. Exempt from `scripts/verify-gate.sh` (see the Exception above).

## Workflow rule (auto-memory)

One feature = one commit. Run `bash scripts/verify-gate.sh` and confirm green before committing. (Docs-only commits skip the gate.)

## CI workflows

- `.github/workflows/ci.yml` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --lib`, `vue-tsc --noEmit`, i18n parity.
- `.github/workflows/performance.yml` — Criterion benches (`benches/cursor_build.rs`, `benches/startup.rs`); regression detection on PRs.
- `.github/workflows/release.yml` — signed installer builds.

Marketplace submission validation lives in the separate [`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) repo (`scripts/marketplace/validate.mjs`, `marketplace-validate.yml`; Ajv-based version is canonical).

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- Do **not** scaffold new features under an `easy-cursor-swap/` subdirectory; the repo root is `easy-cursor-swap/` itself. The workspace `CLAUDE.md` (`<USER_HOME>\Workspace\CLAUDE.md`) and some legacy references mention `cursor-forge/` — ignore those.
- `npm run dev` (Nuxt only) has no Tauri runtime — IPC will fail. Use `npm run tauri:dev`.
