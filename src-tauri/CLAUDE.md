# src-tauri/ — Backend (Rust 1.82+, Tauri v2)

This file is loaded automatically when working under `src-tauri/`. Root `../CLAUDE.md` is also loaded — **read it first** for cross-cutting invariants (HKCU only / transactional apply / PII redaction / archive sanitization), the verification gate, and the documentation update policy.

Crates: `windows`, `winreg`, `image`, `tracing`, `ed25519-dalek`, `tauri` v2.

## Architecture

```
Vue (UI) ──invoke()──▶ Tauri command (commands/) ──▶ Rust module ──▶ Windows registry / FS
```

**Rust is the single source of truth.** All persistent app state lives here; the frontend reflects it via IPC.

## Layout

`src/` modules are registered in `lib.rs`. Grouped by responsibility:

| Concern         | Modules                                                                                                                                                                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| IPC surface     | `commands/` (sub-modules: `theme` / `cursor_build/` / `cursor_io` / `keystore` / `marketplace` / `marketplace_submit` / `profile` / `system` / `windows_scheme`)                                                                                                                   |
| Config / state  | `config.rs` (RwLock + schema_version v1 + `config.corrupt.*.json` quarantine, Source of Truth), `errors.rs`, `cancel_registry.rs` (shared cursorpack-build / bulk-import cancellation registry App state)                                                                          |
| Cursor pipeline | `cursor/` (`image` / `cur_build` / `ico_cur` / `ani` / `ani_write`), `cursor_watcher.rs`                                                                                                                                                                                           |
| Registry        | `registry/` (`mod` / `scheme` / `roles` / `env`)                                                                                                                                                                                                                                   |
| Theme packages  | `theme/`, `bulk_import/`, `backup.rs` (`.cursorprofile`)                                                                                                                                                                                                                           |
| Marketplace     | `marketplace.rs` (HTTP index fetch, SHA-256 + Ed25519 verify), `keystore.rs` (Ed25519 + DPAPI + `.cfkey` XChaCha20-Poly1305 + Argon2id)                                                                                                                                            |
| Reliability     | `health.rs` (startup-failure counter + rollback), `crash.rs`                                                                                                                                                                                                                       |
| OS integration  | `tray.rs`, `hotkey.rs`, `autostart.rs`, `appusermodel.rs`, `accessibility.rs`, `environment.rs` (RDP/Citrix detection). Multi-instance lock via `tauri_plugin_single_instance` (no custom module). Dark mode is handled on the frontend (`useUiTheme` composable).                 |
| Observability   | `logging.rs` (`redact_path` / `short_hash` PII helpers)                                                                                                                                                                                                                            |

**Full file-by-file detail and exact module / IPC counts: `docs/architecture.json` → `backend.modules[]` and `backend.ipc_commands[]`, or `docs/file_inventory.md` section 1.**

## Conventions

- **Comments and doc strings: Japanese.**
- Use `tracing::{info,warn,error,debug,trace}!` for logs; never `println!`. Always pass paths through `logging::redact_path` and hashes through `logging::short_hash` (12 chars).
- Errors propagate as `AppError`; IPC commands return `Result<T, AppError>`.
- Prefer `RwLock` over `Mutex` when read-heavy (e.g. `config.rs`).
- Use `tokio::task::spawn_blocking` for blocking I/O in async contexts (ZIP extraction, large file scans).

## Commands

Operate inside `src-tauri/` (or via `--manifest-path` from repo root).

```bash
cargo check
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib                                  # all unit tests
cargo test --lib cursor::ani_write::tests::name   # single test
cargo bench                                       # criterion benches in benches/
```

## Adding a `#[tauri::command]`

1. Implement the function in `src/commands/<sub-module>.rs`. Use snake_case in Rust; the mirroring TS payload type in `app/types/` uses camelCase (serde rename if needed).
2. Register it in the `invoke_handler` list in `lib.rs`.
3. Add the matching payload type in `app/types/`.
4. Add a `tracing::info!` log on entry; redact any PII.
5. Update `docs/architecture.md` IPC inventory + `docs/file_inventory.md` (numbers must stay in sync) + `docs/architecture.json`, and re-embed via `node scripts/embed-arch-json.mjs`.

## Hard rules (backend-side)

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional.** Snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating; delete on success. Leftover snapshot on startup triggers auto-rollback.
- **PII redaction is mandatory.** Raw registry values and full SHA-256 must never appear in logs.
- **Archive sanitisation.** Any unzip path must go through `theme::sanitize_archive_path` with the documented size limits (50 MB compressed / 200 MB expanded / 10 MB per image / 1 GB total).

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- `cargo test --lib` is the canonical test runner for the verification gate; integration tests in `tests/` are not run by `verify-gate.sh`.
