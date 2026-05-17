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

### Consumption tiers (important)

| Tier | Files | Who reads it |
|---|---|---|
| **1. Required for AI (canonical for agents)** | `docs/architecture.json` + `docs/ui_map.json` | AI/agents only need **these two files** to get full coverage of structure / IPC / UI interactions / security invariants. ~27k tokens total gives the whole picture. |
| **2. Optional for AI / preferred for humans (supplementary prose)** | `docs/architecture.md` + `docs/file_inventory.md` | AI reads these only when narrative / refactor history / why-context / design-decision background is needed. Not required for routine structural understanding. Humans (PR review / onboarding) primarily read these. |
| **3. Humans only (visual viewer)** | `docs/architecture.html` + `docs/ui_map.html` | **AI must NOT Read these** (opening them via the Read tool wastes ~24k + ~29k = ~53k tokens for zero added information — the embedded JSON is identical to Tier 1). HTML exists solely for the dark-mode styling, accordion tree, and live drift check. |

### File-by-file detail

| File | Role | Update policy |
|---|---|---|
| `docs/architecture.json` (Tier 1) | **Living architecture map (canonical for agents)** — Structured JSON of all 21 Rust modules, 53 IPC commands, 27 composables, 50 components, 15 security invariants, startup sequence, and Page → Composable → IPC routing, all with `file` pointers to real sources. **The first file an AI should read** — ~95% of the architecture is reachable from here. | Update whenever `architecture.md` or `file_inventory.md` is updated. Re-measure numbers against the source (`grep -c` / `glob`) before bumping. |
| `docs/ui_map.json` (Tier 1) | **Living UI map (frontend pair of `architecture.json`)** — every user-facing interaction in the Nuxt SPA (4 pages, 50 components, 197 interactions, 50 distinct IPCs) catalogued as Page → Component → Action → IPC → Backend module. Answers "what does clicking X do?" and "which UI calls IPC Y?" in either direction. The redundant `ipc_index` field has been removed — agents that need the reverse lookup should walk `interactions[].ipc` directly (fewer tokens than reading a separate index). | Update in the same commit as the corresponding `app/` change: new `@click` / `@change` / `@input` / keyboard shortcut / drag-drop / modal action requires a new entry. Re-measure `interactions_total` / `ipc_unique_targets` — the `runLiveCheck` script at the bottom of `ui_map.html` is the canonical counter. |
| `docs/architecture.md` (Tier 2) | **Prose architecture map** — Rust/Vue module responsibilities, IPC inventory grouped by category, startup sequence, Page → IPC routing, Security invariants + **why explanations**, ASCII tree diagram, refactor tracking history. Holds the narrative content (why / refactor history / design-decision background) that the JSON does not mirror. | Update when the codebase structure changes (module splits, new IPC, new security invariants, etc.). Update the JSON in the same commit. |
| `docs/file_inventory.md` (Tier 2) | **Prose file index** — full file-by-file table for `src-tauri/src/` and `app/` with direct source links. An expanded prose version of the JSON `role` / `purpose` fields, plus supplementary information for locales / types / CSS / layouts. | Update on file creation/removal or responsibility moves. Update the JSON in the same commit. |
| `docs/architecture.html` (Tier 3) | **Human-only viewer for `architecture.json`** — dark theme + accordion + search. AI does NOT read this (the embedded JSON is identical to the Tier 1 file). | When `architecture.json` is updated, re-embed the JSON into the HTML in the same commit. |
| `docs/ui_map.html` (Tier 3) | **Human-only viewer for `ui_map.json`** with a **live drift self-check** that runs on page load: it compares `meta.measured_counts` against the actual data and shows a warning banner if they disagree. AI does NOT read this. | When `ui_map.json` is updated, re-embed the JSON into the HTML in the same commit. |
| `docs/updater_signing.md` / `docs/authenticode_signing.md` / `docs/distribution.md` / `docs/key_rotation.md` / `docs/author_registration.md` / `docs/code_signing_policy.md` | **Operational runbooks** — Tauri Updater minisign key management / Authenticode certificate procurement / MSIX distribution / key-rotation PR / new-author registration / code signing policy. | Update only when the procedure itself changes. |

When documents disagree, **`docs/architecture.md` + `docs/file_inventory.md` (Tier 2 prose) are authoritative** (they are written prose with code references); `docs/architecture.json` / `docs/ui_map.json` (Tier 1) are the structured mirrors agents consume — keep all of them in sync, and let prose disagreements be resolved by reading the actual source. **Tier 3 `.html` follows automatically once the embedded JSON is updated**, so HTML files are never edited directly.

> Internal design records (the original-plan documents formerly in `docs/legacy/` and the per-feature work logs in `docs/superpowers/`) were removed from history before the v0.1.0 release. They continue to be maintained locally only and are not part of the public repository.

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

**Exception — docs-only commits are out of scope.** If a commit touches **only** documentation files (`CLAUDE.md`, `README.md`, `README.ja.md`, anything under `docs/`, `CHANGELOG.md`, and community markdown like `SUPPORT.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md` / `CONTRIBUTING.md`) and **nothing under `app/`, `src-tauri/`, `scripts/`, `.github/`, `package.json`, `nuxt.config.ts`, or other config**, `scripts/verify-gate.sh` is **not required**. Every step in the gate (`cargo fmt --check` / `cargo clippy` / `cargo test --lib` / `prettier --check` / `vue-tsc --noEmit` / `node scripts/check-i18n.mjs` / `vitest run`) targets Rust or Vue code, not prose — running it on a docs-only diff wastes time without catching anything. Mark these with the `docs: ...` Conventional Commit prefix.

## Architecture

```
Vue (UI) ──invoke()──▶ Tauri command (commands/) ──▶ Rust module ──▶ Windows registry / FS
```

**Rust is the single source of truth.** Frontend state must be synced via IPC; never persist app state only on the Vue side.

### Frontend layout (`app/`)

- `pages/` — `index.vue` (Library), `creator.vue`, `marketplace.vue`, `settings.vue` (4 pages; helpers in `index.helpers.ts` / `marketplace.helpers.ts`).
- `components/{shell,library,creator,marketplace,settings,preview,panic,icons,ui}/` — domain-grouped SFCs. `nuxt.config.ts` sets `pathPrefix: false`, so reference components by file name (`<ThemeCard>`, not `<LibraryThemeCard>`).
- `composables/` — 27 in total. IPC wrapper (`useTauri`), domain state (`useThemes`, `useAppSettings`, `useAppInfo`, `useKeystore`, `useUpdater`, `useUpdaterBootstrap`, `useNotify`, `useUiTheme`), Creator helpers (`useCreatorAssets`, `useCreatorPickers`, `useCreatorImport`, `useCreatorBulkImportFlow`, `useCreatorExport`, `useHotspotDefaults`, `useHotspotInteraction`, `useAniPlayer`, `useBulkImport`, `useRoleMatcher`, `useThemePreviews`), marketplace (`useMarketplacePreviews`, `useGithubAuth`, `useMarketplaceSubmit`), and UI utilities (`useCursorpackOpener`, `useI18n`, `useSettingsSearch`, `sanitizeSvg`). Vitest specs live in `app/composables/__tests__/` (15 files). See section 2-3 of `docs/file_inventory.md` for details.
- `locales/{ja,en}.ts` — keys typed `as const`; **must stay in parity** (CI gate via `scripts/check-i18n.mjs`).
- `types/` — IPC payload types (`config.ts`, `theme.ts`, `marketplace.ts`, `githubAuth.ts`).
- `assets/css/tailwind.css` — Tailwind v4 entry + `@theme` block (exposes design tokens as utilities) + cross-cutting shared utilities (`.btn` / `.card` / `.chip` / `.input` / `.tag` / `.toolbar` / `.tabs` / `.prop-section` / `.lib-table` / `.lib-row` / `.lt-*` / `.modal*` / `.content` / `.page-head` / `.grid`, etc.).
- `assets/css/global.css` — `:root` + `html.light` design tokens, CSS reset, scrollbar customisation, `:focus-visible`, `prefers-reduced-motion`, and shared `@keyframes` (pulse / fade-in / slide-in-right / spin) only. Do **not** add component-specific styles here.

### Backend layout (`src-tauri/src/`)

21 modules registered in `lib.rs`. Grouped by responsibility:

| Concern | Modules |
|---|---|
| IPC surface | `commands/` (53 `#[tauri::command]` across 10 sub-modules: `theme` / `cursor_build/` (5 files) / `cursor_io` / `keystore` / `marketplace` / `marketplace_submit` / `profile` / `system` / `updater` / `windows_scheme`) |
| Config / state | `config.rs` (RwLock + schema_version v1 + `config.corrupt.*.json` quarantine on parse failure, Source of Truth), `errors.rs` |
| Cursor pipeline | `cursor/` (5 files: `image` / `cur_build` / `ico_cur` / `ani` / `ani_write`), `cursor_watcher.rs` |
| Registry | `registry/` (4 files: `mod` / `scheme` / `roles` / `env`) |
| Theme packages | `theme/` (3 files: `mod` / `types` / `sanitize`), `bulk_import/` (3 files: `mod` / `assets` / `cursorpack`), `backup.rs` (`.cursorprofile`) |
| Marketplace | `marketplace.rs` (HTTP index fetch, SHA-256 + Ed25519 verify), `keystore.rs` (Ed25519 + DPAPI + `.cfkey` XChaCha20-Poly1305 + Argon2id) |
| Reliability | `health.rs` (startup-failure counter + rollback), `crash.rs` |
| OS integration | `tray.rs`, `hotkey.rs`, `autostart.rs`, `appusermodel.rs`, `accessibility.rs`, `environment.rs` (RDP/Citrix detection). Multi-instance lock is `tauri_plugin_single_instance` (no custom module). Dark mode is handled on the frontend (`useUiTheme` composable). |
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
2. **Read existing code before writing.** Start with the **Tier 1 docs**: `docs/architecture.json` (21 Rust modules / 53 IPC commands / 27 composables / 50 components / 15 security invariants / startup sequence) and `docs/ui_map.json` (197 UI interactions / Page → Action → IPC routing). These two files (~27k tokens total) give you full coverage of "which module owns what" and "which UI interaction triggers which IPC". **Do NOT read Tier 3 `.html` files** — they are humans-only and their embedded JSON is identical to Tier 1, so reading them just wastes ~53k tokens. **Read Tier 2 `.md` files only when narrative / why / refactor history is needed** — they are not required for structural understanding. Follow the `file` pointers in Tier 1 down to the real sources (`app/` / `src-tauri/src/` / `composables/` / `commands/`) and match the existing naming, types, and IPC conventions. Update both `locales/{ja,en}.ts` so `scripts/check-i18n.mjs` stays green. Prefer extending an existing composable / module over introducing duplicate code.
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
| Module count, IPC count, composable count, page count, sub-module list, component sub-directory file list, or any other numeric / enumerated claim drifted in any of the **living-doc ring** — `CLAUDE.md`, `README.md`, `README.ja.md`, `docs/architecture.md`, `docs/file_inventory.md`, `docs/architecture.json`, `docs/architecture.html` | These seven files are **mirrors of each other**. Re-measure against the actual source (`grep -c` / `glob`) and **update every file in the ring that mentions the changed number in the same commit**. Touching only one of them is the single most common cause of doc rot in this repo. CLAUDE.md itself counts — it directly states "21 modules registered in `lib.rs`", "53 `#[tauri::command]` across 10 sub-modules", "27 composables", etc. If those numbers change, CLAUDE.md changes too. |
| Any trigger above that touched `docs/architecture.md` or `docs/file_inventory.md` | **Also update `docs/architecture.json` and re-embed it into `docs/architecture.html`.** The JSON is the agent-facing structured mirror — its `backend.modules[]` / `backend.ipc_commands[]` / `frontend.composables[]` / `frontend.pages[]` / `critical_invariants[]` / `meta.measured_counts` entries must match the prose docs, and bump `meta.generated_at`. The HTML carries a copy of the JSON inside `<script id="data" type="application/json">`; re-embed it with the one-liner below so the two copies never diverge by hand-edit:<br>`node -e "const fs=require('fs'),h=fs.readFileSync('docs/architecture.html','utf8'),j=fs.readFileSync('docs/architecture.json','utf8').trimEnd(),o='<script id=\"data\" type=\"application/json\">',c='</script>',i=h.indexOf(o),k=h.indexOf(c,i);fs.writeFileSync('docs/architecture.html',h.slice(0,i)+o+'\n'+j+'\n'+h.slice(k));JSON.parse(fs.readFileSync('docs/architecture.html','utf8').match(/<script id=\"data\" type=\"application\/json\">\n([\s\S]*?)\n<\/script>/)[1]);"`<br>The trailing `JSON.parse(...)` is the safety check — it errors loudly if the embed is malformed. After running, also clear or update `meta.doc_drift_warnings`: keep `["No known drift as of YYYY-MM-DD (re-measured)."]` if nothing remains, or list the remaining drifts in the `"docs/<file> says X but actual is Y"` format. |

**Doc-only changes** (no code touched) are fine as standalone commits — for example, fixing a stale number, clarifying a pitfall, or adding a new runbook step. Use a `docs: ...` Conventional Commit prefix. These commits are exempt from `scripts/verify-gate.sh` (see the "Exception — docs-only commits are out of scope" note in the Verification gate section).

## Workflow rule (auto-memory)

One feature = one commit. Run `bash scripts/verify-gate.sh` and confirm it passes before committing. (Docs-only commits skip the gate — see the Verification gate section for the precise file-scope definition.)

## CI workflows

- `.github/workflows/ci.yml` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --lib`, `vue-tsc --noEmit`, i18n parity.
- `.github/workflows/performance.yml` — Criterion benches (`benches/cursor_build.rs`, `benches/startup.rs`); regression-detected on PRs.
- `.github/workflows/release.yml` — signed installer builds.

Marketplace submission validation (`.cursorpack` SHA-256 / Ed25519 / size / malware DB) lives in the separate [`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) repo under `scripts/marketplace/validate.mjs` and the `marketplace-validate.yml` workflow (the Ajv-based version is canonical).

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- Do **not** scaffold new features under an `easy-cursor-swap/` subdirectory; the repo root is `easy-cursor-swap/` itself (matches the git remote name). The workspace `CLAUDE.md` (`<USER_HOME>\Workspace\CLAUDE.md`) and some legacy references mention `cursor-forge/` for historical reasons — ignore those.
- `npm run dev` (Nuxt only) is fine in isolation, but Tauri commands will fail because there's no Tauri runtime. Use `npm run tauri:dev` to exercise IPC.
