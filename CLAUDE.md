# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Domain-specific guidance lives in nested files (auto-loaded when working under those dirs):

- `app/CLAUDE.md` — Frontend (Nuxt 4 / Vue 3 / Tailwind v4).
- `src-tauri/CLAUDE.md` — Backend (Rust 1.82+, Tauri v2).

## Project

EasyCursorSwap (`package.json` name: `easy-cursor-swap`) — a Windows-only desktop app for managing custom mouse cursor themes. Tauri v2 + Nuxt 4 + Rust hybrid. The project lives at the repo root (no `easy-cursor-swap/` subdirectory).

- **Target:** Windows 10 22H2+ / Windows 11, x64 (ARM64 planned)
- **Distribution:** NSIS / MSI installers signed via SignPath; Tauri Updater with Ed25519-signed releases

## Documentation map

`docs/` consists of **Living state docs** (descriptions of the current code) and **Operational runbooks**. Living docs are authoritative — if they disagree with the code, fix the docs.

| Tier                                          | Files                                             | Who reads it                                                                                                                                                                                               |
| --------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Required for AI (canonical for agents)** | `docs/architecture.json` + `docs/ui_map.json`     | AI/agents only need these two files for full coverage of structure / IPC / UI interactions / security invariants (~27k tokens total). **Read these first** — module / IPC / composable counts live inside. |
| **2. Optional for AI / preferred for humans** | `docs/architecture.md` + `docs/file_inventory.md` | AI reads only when narrative / refactor history / why-context is needed.                                                                                                                                   |
| **3. Humans only (visual viewer)**            | `docs/architecture.html` + `docs/ui_map.html`     | **AI must NOT Read these.** Their embedded JSON is identical to Tier 1; opening them wastes ~53k tokens for zero added information.                                                                        |

**Operational runbooks** (procedure-only): `docs/updater_signing.md` / `authenticode_signing.md` / `distribution.md` / `key_rotation.md` / `author_registration.md` / `code_signing_policy.md`.

When documents disagree, **Tier 2 prose is authoritative**; Tier 1 JSON is the structured mirror agents consume. Tier 3 HTML is regenerated from JSON via `node scripts/embed-arch-json.mjs` — never hand-edit.

> Internal design records (the original-plan documents formerly in `docs/legacy/` and the per-feature work logs in `docs/superpowers/`) were removed from history before the v0.1.0 release. They are maintained locally only.

## Critical invariants (cross-cutting)

These apply regardless of which side you're working on. Full list (including module-specific ones) is in `docs/architecture.json` → `critical_invariants[]`.

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

Edit `scripts/verify-gate.sh` directly to change gate steps (do **not** re-document them here or in `docs/architecture.md`). To validate installer builds as well, additionally run `npm run tauri:build`.

**Exception — docs-only commits skip the gate.** If a commit touches only `CLAUDE.md` (root or sub-dirs) / `README*.md` / anything under `docs/` / `CHANGELOG.md` / community markdown (`SUPPORT.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md` / `CONTRIBUTING.md`), and nothing under `app/`, `src-tauri/`, `scripts/`, `.github/`, `package.json`, `nuxt.config.ts`, the gate is not required. Mark with `docs:` Conventional Commit prefix.

## Implementation policy

When starting any new feature, refactor, or bug fix, always follow these steps in order:

1. **Invoke the relevant skill** via the `Skill` tool if there's even a 1% chance one applies (e.g. `superpowers:brainstorming`, `superpowers:test-driven-development`, `superpowers:systematic-debugging`, `rust-skills:m01-ownership`).
2. **Read Tier 1 docs before writing** (`docs/architecture.json` + `docs/ui_map.json`). Do NOT read Tier 3 `.html`. Read Tier 2 `.md` only when narrative is needed. Follow `file` pointers down to real sources and match existing conventions. Update `locales/{ja,en}.ts` in parity. Prefer extending an existing composable / module over duplication.
3. **Run `bash scripts/verify-gate.sh`** right before committing and confirm green.
4. **Update docs in the same commit** (see policy below). Code-only commits that move source-of-truth without touching living docs are the main cause of doc rot.

## Documentation update policy

Living docs must move with the code. Triggers and required updates:

- **New / renamed / removed Rust file** → `docs/file_inventory.md` section 1 (+ `docs/architecture.md` "Backend layout" if module boundary changed).
- **Added / removed `#[tauri::command]`** → `docs/architecture.md` IPC inventory + `docs/file_inventory.md`. Numbers must stay in sync.
- **Module split / merge** → `docs/architecture.md` responsibility map + Backend layout + refactor tracking, plus `docs/file_inventory.md`.
- **Startup sequence change** (`main.rs`) → `docs/architecture.md` Startup sequence list.
- **New security invariant** → `docs/architecture.md` Security table + README "Security Model" if user-visible.
- **New / changed Vue page, composable, or component sub-directory** → `docs/architecture.md` Frontend layout + Page→Composable→IPC table, and `docs/file_inventory.md`.
- **Tailwind / global CSS pattern change** → `app/CLAUDE.md` CSS subsection.
- **Verification gate change** → `scripts/verify-gate.sh` only.
- **Operational procedure change** → the corresponding runbook in `docs/`.
- **User-visible behaviour / install flow / supported OS / security model change** → both `README.md` and `README.ja.md` in parity, plus `CHANGELOG.md` under `## [Unreleased]` with the right Keep-a-Changelog section.
- **Any numeric / enumerated claim drift in the living-doc ring** — re-measure against the actual source (`grep -c` / `glob`) and update **every file in the ring that mentions the changed number in the same commit** (`README*.md` / `docs/architecture.md` / `docs/file_inventory.md` / `docs/architecture.json`). The CLAUDE.md files no longer hard-code module / IPC / composable counts — they reference `docs/architecture.json`, so doc rot from numeric drift is contained to the documentation ring.
- **Any change touching `docs/architecture.md` or `docs/file_inventory.md`** → also update `docs/architecture.json` (bump `meta.generated_at`, sync `meta.measured_counts`, update `meta.doc_drift_warnings` to `["No known drift as of YYYY-MM-DD (re-measured)."]` or list remaining drifts) and re-embed into `docs/architecture.html`:
  ```bash
  node scripts/embed-arch-json.mjs
  ```

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
