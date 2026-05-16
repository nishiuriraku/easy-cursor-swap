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

`docs/` consists of two kinds of documents: **Living state docs (descriptions of the current code)** and **Operational runbooks**. Living docs are authoritative — if they disagree with the code, fix the docs.

| File | Role | Update policy |
|---|---|---|
| `docs/architecture.md` | **Living architecture map** — Rust/Vue module responsibilities, IPC inventory, startup sequence, Page → IPC routing, Security invariants + code references. Read this first when refactoring or onboarding. | Update when the codebase structure changes (module splits, new IPC, new security invariants, etc.). |
| `docs/file_inventory.md` | **Living file index** — full file-by-file table for `src-tauri/src/` and `app/` with direct source links. Covers "which file holds what" at a finer grain than `architecture.md`. | Update on file creation/removal or responsibility moves. |
| `docs/updater_signing.md` / `docs/authenticode_signing.md` / `docs/distribution.md` / `docs/key_rotation.md` / `docs/author_registration.md` / `docs/code_signing_policy.md` | **Operational runbooks** — Tauri Updater minisign key management / Authenticode certificate procurement / MSIX distribution / key-rotation PR / new-author registration / code signing policy. | Update only when the procedure itself changes. |

When documents disagree, **`docs/architecture.md` + `docs/file_inventory.md` are authoritative** — both are updated alongside the code.

> 内部設計記録 (旧 `docs/legacy/` の original plan 群、`docs/superpowers/` の per-feature work log) は v0.1.0 公開前に history から除去済み。引き続きローカル参照のみで運用する。

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

**Always run `scripts/verify-gate.sh` before committing.** This script is the canonical verification gate, composed of:

```bash
bash scripts/verify-gate.sh
# Breakdown:
#   cargo fmt --check / cargo clippy -D warnings / cargo test --lib
#   prettier --check / vue-tsc --noEmit
#   node scripts/check-i18n.mjs / npm test (vitest)
```

If you want to change the gate steps, edit `scripts/verify-gate.sh` directly (not CLAUDE.md) to keep CI behaviour in sync. To validate installer builds as well, additionally run `npm run tauri:build`.

## Architecture

```
Vue (UI) ──invoke()──▶ Tauri command (commands/) ──▶ Rust module ──▶ Windows registry / FS
```

**Rust is the single source of truth.** Frontend state must be synced via IPC; never persist app state only on the Vue side.

### Frontend layout (`app/`)

- `pages/` — `index.vue` (Library), `creator.vue`, `marketplace.vue`, `settings.vue`, `appearance.vue`
- `components/{shell,library,creator,marketplace,settings,preview,panic,icons,ui}/` — domain-grouped SFCs. `nuxt.config.ts` sets `pathPrefix: false`, so reference components by file name (`<ThemeCard>`, not `<LibraryThemeCard>`).
- `composables/` — 21 in total. IPC wrapper (`useTauri`), domain state (`useThemes`, `useAppSettings`, `useKeystore`, `useUpdater`, `useNotify`, `useUiTheme`), Creator helpers (`useCreatorAssets`, `useCreatorPickers`, `useCreatorImport`, `useCreatorBulkImportFlow`, `useCreatorExport`, `useHotspotDefaults`, `useHotspotInteraction`, `useAniPlayer`, `useBulkImport`, `useRoleMatcher`, `useThemePreviews`), and UI utilities (`useCursorpackOpener`, `useI18n`, `sanitizeSvg`). Vitest specs live in `app/composables/__tests__/` (10 files). See section 2-3 of `docs/file_inventory.md` for details.
- `locales/{ja,en}.ts` — keys typed `as const`; **must stay in parity** (CI gate via `scripts/check-i18n.mjs`).
- `types/` — IPC payload types (`config.ts`, `theme.ts`, `marketplace.ts`).
- `assets/css/tailwind.css` — Tailwind v4 entry + `@theme` block (exposes design tokens as utilities) + cross-cutting shared utilities (`.btn` / `.card` / `.chip` / `.input` / `.tag` / `.toolbar` / `.tabs` / `.prop-section` / `.lib-table` / `.lib-row` / `.lt-*` / `.modal*` / `.content` / `.page-head` / `.grid`, etc.).
- `assets/css/global.css` — `:root` + `html.light` design tokens, CSS reset, scrollbar customisation, `:focus-visible`, `prefers-reduced-motion`, and shared `@keyframes` (pulse / fade-in / slide-in-right / spin) only. Do **not** add component-specific styles here.

### Backend layout (`src-tauri/src/`)

21 modules registered in `lib.rs`. Grouped by responsibility:

| Concern | Modules |
|---|---|
| IPC surface | `commands/` (52 `#[tauri::command]` across 9 sub-modules: `theme` / `cursor_build/` (5 files) / `cursor_io` / `keystore` / `marketplace` / `marketplace_submit` / `profile` / `system` / `windows_scheme`) |
| Config / state | `config.rs` (RwLock + schema_version v1 + `config.corrupt.*.json` 退避, Source of Truth), `errors.rs` |
| Cursor pipeline | `cursor/` (5 files: `image` / `cur_build` / `ico_cur` / `ani` / `ani_write`), `cursor_watcher.rs` |
| Registry | `registry/` (4 files: `mod` / `scheme` / `roles` / `env`) |
| Theme packages | `theme/` (3 files: `mod` / `types` / `sanitize`), `bulk_import/` (3 files: `mod` / `assets` / `cursorpack`), `backup.rs` (`.cursorprofile`) |
| Marketplace | `marketplace.rs` (HTTP index fetch, SHA-256 + Ed25519 verify), `keystore.rs` (Ed25519 + DPAPI + `.cfkey` XChaCha20-Poly1305 + Argon2id) |
| Reliability | `health.rs` (startup-failure counter + rollback), `crash.rs` |
| OS integration | `tray.rs`, `darkmode.rs`, `hotkey.rs`, `autostart.rs`, `appusermodel.rs`, `accessibility.rs`, `environment.rs` (RDP/Citrix detection). Multi-instance lock is `tauri_plugin_single_instance` (no custom module). |
| Observability | `logging.rs` (with `redact_path` / `short_hash` PII helpers) |

### Critical invariants

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional.** `registry/mod.rs` writes a snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating, deletes it on success. On startup, a leftover snapshot triggers auto-rollback. There is also `_initial_snapshot.json` saved on first run; the panic button (`Ctrl+Alt+Shift+R`) restores either Windows defaults or this initial snapshot.
- **Cursor files live in `~/.custom_cursors/`** so they survive uninstall.
- **PII redaction in logs.** When using `tracing!`, route raw paths through `logging::redact_path` and hashes through `logging::short_hash` (12 chars). No raw registry values, no full SHA-256 in logs.
- **Archive sanitisation.** Any code unzipping `.cursorpack` / `.cursorprofile` must go through `theme::sanitize_archive_path` and the size limits (50 MB compressed / 200 MB expanded / 10 MB per image / 1 GB total user storage).
- **No `v-html`** anywhere in Vue — SVG icons go through render functions in `UiIcon.vue` / `CursorIcon.vue`.
- **`routeRules: { '/**': { ssr: false } }`** in `nuxt.config.ts` is intentional. Nuxt 4.4.4 has an IPC bug with `ssr: false` at the top level; do not change to `ssr: false` directly.

## Coding conventions

- Rust comments and doc strings: **Japanese**.
- Vue: SFC + `<script setup>` + Composition API + TypeScript.
- CSS: **Tailwind v4 utility classes** as the default styling mechanism.
  - Design tokens live in `app/assets/css/tailwind.css` (`@theme` block), aliasing legacy `--*` tokens.
  - **Cross-cutting shared utilities** (`.btn`, `.card`, `.chip`, `.input`, `.tag`, `.toolbar`, `.tabs`, `.prop-section`, `.lib-row`, `.lt-*`, `.modal*`, `.content`, `.page-head`, `.grid`, etc.) are defined at the top level (unlayered) of `app/assets/css/tailwind.css`. **Do not** wrap them in `@layer components` — Tailwind preflight (e.g. `button { color: inherit }`) is emitted unlayered, so rules inside `@layer` lose the cascade.
  - **Component-specific styles** belong in each `.vue` file's `<style scoped>`. Declare `@reference '~/assets/css/tailwind.css';` at the top, then use `@apply`.
  - `app/assets/css/global.css` is **strictly limited** to `:root` tokens, CSS reset, scrollbar customisation, `:focus-visible`, `prefers-reduced-motion`, shared `@keyframes`, and `html.light` token overrides. Do not add component-specific styles here (Phase 10-12 collapsed it from 3327 lines to 223).
- IPC payload types live in `app/types/` and must mirror `serde`-derived Rust structs in `commands/` sub-modules / domain module files.
- Filenames in `components/` are referenced without directory prefix — keep names globally unique.

## Implementation policy

When starting any new feature, refactor, or bug fix, **always** follow these steps in order:

1. **Invoke the relevant skill.** If there is even a 1% chance a skill applies to the task, launch it via the `Skill` tool (e.g. `superpowers:brainstorming` for ideation, `superpowers:test-driven-development` for TDD, `superpowers:systematic-debugging` for debugging, `rust-skills:m01-ownership` for Rust ownership/concurrency errors, and the matching skill for Cloudflare / Tauri / Nuxt-specific work). When in doubt, launch it and discard if it doesn't fit.
2. **Read existing code before writing.** Read the relevant areas of `app/` / `src-tauri/src/` / `composables/` / `commands/` first, then match the prior naming, types, and IPC conventions. Update both `locales/{ja,en}.ts` so `scripts/check-i18n.mjs` stays green. Prefer extending an existing composable / module over introducing duplicate code.
3. **Use `scripts/verify-gate.sh` as the verification gate.** Run `bash scripts/verify-gate.sh` right before committing and confirm it passes green. Do not use the inline sequence (`cargo fmt` … `npm run tauri:build`) — the script is canonical.
4. **Update docs in the same commit.** See "Documentation update policy" below. Code-only commits that move the source-of-truth without touching the living docs are the main cause of doc rot in this repo — do not create them.

## Documentation update policy

Living docs must move with the code. The mapping below is the **trigger → action** contract: when you make the change in the left column, update the file(s) in the right column **in the same commit**.

| Trigger (what you changed in code) | Update these |
| --- | --- |
| Added / removed / renamed a Rust file under `src-tauri/src/` | `docs/file_inventory.md` section 1 (file table). If it changes a module boundary, also update `docs/architecture.md` "Backend layout / responsibility map". |
| Added / removed a `#[tauri::command]` | `docs/architecture.md` IPC inventory (count + category table) **and** `docs/file_inventory.md` section 1-2. The two numbers must stay in sync. |
| Module split / merge (e.g. `foo.rs` → `foo/`) | `docs/architecture.md` "responsibility map" + "Backend layout" + "refactor tracking" entry, plus `docs/file_inventory.md`. |
| Startup sequence changed (`main.rs`) | `docs/architecture.md` "Startup sequence" numbered list. |
| New security invariant or threat-mitigation primitive | `docs/architecture.md` "Security" table (invariant + responsible module). Re-check whether the README "Security Model" table needs an entry too. |
| New / changed Vue page, composable, or component sub-directory | `docs/architecture.md` "Frontend layout" + "Page → Composable → IPC" table, and `docs/file_inventory.md` section 2. |
| Tailwind / global CSS pattern change | `CLAUDE.md` "Coding conventions" CSS subsection (this file). |
| Verification gate changed | Edit `scripts/verify-gate.sh` itself (canonical) — do **not** re-document the steps in CLAUDE.md or `docs/architecture.md`. |
| Operational procedure change (Updater key issuance, Authenticode procurement, MSIX distribution, key rotation, author onboarding, code signing policy) | The corresponding runbook in `docs/` (`updater_signing.md` / `authenticode_signing.md` / `distribution.md` / `key_rotation.md` / `author_registration.md` / `code_signing_policy.md`). |
| User-visible behaviour, install flow, supported OS, security model, or installation step changed | Both `README.md` and `README.ja.md` in parity, and `CHANGELOG.md` under `## [Unreleased]` with the right Keep-a-Changelog sub-section (`Added` / `Changed` / `Deprecated` / `Removed` / `Fixed` / `Security`). |
| Module count, IPC count, composable count, or any other numeric claim in any doc | Re-verify the number against the actual source (`grep -c` / `glob`) before changing it. **Numbers in docs are the most common drift signal** — don't trust them, count them. |

**Doc-only changes** (no code touched) are fine as standalone commits — for example, fixing a stale number, clarifying a pitfall, or adding a new runbook step. Use a `docs: ...` Conventional Commit prefix.

## Workflow rule (auto-memory)

One feature = one commit. Run `bash scripts/verify-gate.sh` and confirm it passes before committing.

## CI workflows

- `.github/workflows/ci.yml` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --lib`, `vue-tsc --noEmit`, i18n parity.
- `.github/workflows/performance.yml` — Criterion benches (`benches/cursor_build.rs`, `benches/startup.rs`); regression-detected on PRs.
- `.github/workflows/release.yml` — signed installer builds.

Marketplace submission validation (`.cursorpack` SHA-256 / Ed25519 / size / malware DB) lives in the separate [`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) repo under `scripts/marketplace/validate.mjs` and the `marketplace-validate.yml` workflow (the Ajv-based version is canonical).

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- Do **not** scaffold new features under an `easy-cursor-swap/` subdirectory; the workspace `CLAUDE.md` (`<USER_HOME>\Workspace\CLAUDE.md`) lists the project that way for historical reasons, but the actual repo root is `cursor-forge/`.
- `npm run dev` (Nuxt only) is fine in isolation, but Tauri commands will fail because there's no Tauri runtime. Use `npm run tauri:dev` to exercise IPC.
