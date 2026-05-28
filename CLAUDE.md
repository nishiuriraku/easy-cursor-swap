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

**All living state docs live in the Obsidian vault** as of 2026-05-28 (`<USER_HOME>\Workspace\Obsidian`, under `develop/easy-cursor-swap/`). Repo `docs/` keeps **operational runbooks only**. Specs are single-sourced in the vault — if a doc disagrees with the code, fix the vault doc.

| Role                                               | Files (location)                                                                                  | Who reads it                                                                                                                                            |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Canonical structured facts** (agents read first) | Obsidian vault `develop/easy-cursor-swap/reference/architecture.json` + `…/reference/ui_map.json` | AI/agents: full coverage of structure / IPC / UI interactions / security invariants. **Read these first.** Distilled views: `…/specs/*` + `…/shared/*`. |
| **File index**                                     | `…/reference/file_inventory.md`                                                                   | When per-file detail is needed.                                                                                                                         |
| **Visual map (humans)**                            | Obsidian vault `develop/easy-cursor-swap/overview.canvas` + Obsidian Graph View                   | **AI must NOT read `.canvas`** — it's a navigation aid, redundant with `architecture.json`.                                                             |

**Operational runbooks** (procedure-only, in repo `docs/`): `docs/release_procedure.md` / `updater_signing.md` / `authenticode_signing.md` / `distribution.md` / `key_rotation.md` / `author_registration.md` / `code_signing_policy.md`.

When documents disagree, `reference/architecture.json` is canonical (the structured single source agents consume); narrative / why lives in the relevant `specs/*` 設計判断 block. The **Tier 1/2/3 system was abolished on 2026-05-28** (docs SSOT redesign): the duplicate narrative `architecture.md` and the HTML viewers (`architecture.html` / `ui_map.html`), plus the `scripts/embed-arch-json.mjs` embed step, were retired. Human visual = `overview.canvas` + Obsidian Graph View.

> Design history — the original-plan documents (formerly `docs/legacy/`) and per-feature work logs (formerly `docs/superpowers/`) — was moved to the Obsidian vault on 2026-05-28: `develop/easy-cursor-swap/legacy/` and `…/superpowers/`. Both were always git-untracked / removed from history before v0.1.0.

## Critical invariants (cross-cutting)

These apply regardless of which side you're working on. Full list (including module-specific ones) is in the Obsidian vault `develop/easy-cursor-swap/reference/architecture.json` → `critical_invariants[]` (link-rich distilled view: `…/shared/invariants.md`).

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional.** `registry/mod.rs` writes a snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating, deletes it on success. On startup, a leftover snapshot triggers auto-rollback. `_initial_snapshot.json` (first-run) is restored by the panic button (`Ctrl+Alt+Shift+R`).
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
2. **Read the canonical docs before writing** — Obsidian vault `develop/easy-cursor-swap/reference/architecture.json` + `…/reference/ui_map.json` (or the distilled `…/specs/*` + `…/shared/*`). Do NOT read `overview.canvas` (human visual aid, redundant with the JSON). Read `reference/file_inventory.md` only when per-file detail is needed. Follow `file` pointers down to real sources and match existing conventions. Update `locales/{ja,en}.ts` in parity. Prefer extending an existing composable / module over duplication.
3. **Run `bash scripts/verify-gate.sh`** right before committing and confirm green.
4. **Update docs in the same commit** (see policy below). Code-only commits that move source-of-truth without touching living docs are the main cause of doc rot.

## Documentation update policy

Living docs must move with the code. Triggers and required updates:

- **New / renamed / removed Rust file** → `reference/architecture.json` `backend.modules[]` + `reference/file_inventory.md`.
- **Added / removed `#[tauri::command]`** → `reference/architecture.json` `backend.ipc_commands[]` + `meta.measured_counts` + `reference/file_inventory.md`. Numbers must stay in sync.
- **Module split / merge** → `reference/architecture.json` `backend.modules[]` + `reference/file_inventory.md`.
- **Startup sequence change** (`main.rs`) → `reference/architecture.json` `backend.startup_sequence[]`.
- **New security invariant** → `reference/architecture.json` `critical_invariants[]` + `shared/invariants.md` + README "Security Model" if user-visible.
- **New / changed Vue page, composable, or component sub-directory** → `reference/architecture.json` `frontend.*` + `reference/ui_map.json` + `reference/file_inventory.md`.
- **Tailwind / global CSS pattern change** → `app/CLAUDE.md` CSS subsection.
- **Verification gate change** → `scripts/verify-gate.sh` only.
- **Operational procedure change** → the corresponding runbook in `docs/`.
- **User-visible behaviour / install flow / supported OS / security model change** → both `README.md` and `README.ja.md` in parity, plus `CHANGELOG.md` under `## [Unreleased]` with the right Keep-a-Changelog section.
- **Any numeric / enumerated claim drift** — re-measure against the actual source (`grep -c` / `glob`) and update **every file that mentions the changed number in the same commit**: `README*.md` / `reference/file_inventory.md` / vault `reference/architecture.json` (the sole numeric home — `meta.measured_counts`). The CLAUDE.md files no longer hard-code counts, and `index.md` no longer carries a count table, so numeric drift is contained to `architecture.json`.
- **Any change touching `reference/file_inventory.md`** → also bump `reference/architecture.json` `meta.generated_at` + sync `meta.measured_counts` + set `meta.doc_drift_warnings` accordingly. If the high-level structure changed, also hand-adjust `overview.canvas`. (Narrative `architecture.md`, the HTML viewers, and the `scripts/embed-arch-json.mjs` step were all retired 2026-05-28.)

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
