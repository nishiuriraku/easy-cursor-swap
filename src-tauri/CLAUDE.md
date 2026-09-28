# src-tauri/ — Backend (Rust 1.82+, Tauri v2)

This file is loaded automatically when working under `src-tauri/`. Root `../CLAUDE.md` is also loaded — **read it first** for cross-cutting invariants (HKCU only / transactional apply / PII redaction / archive sanitization), the verification gate, and the documentation update policy.

Crates: `windows`, `winreg`, `image`, `tracing`, `ed25519-dalek`, `tauri` v2.

## Architecture

```
Vue (UI) ──invoke()──▶ Tauri command (commands/) ──▶ platform::CursorBackend ──▶ registry/ ──▶ Windows registry / FS
```

**Rust is the single source of truth.** All persistent app state lives here; the frontend reflects it via IPC.

## Layout

`src/` modules are declared (`pub mod`) in `lib.rs`; IPC commands are registered in `src/commands/mod.rs` (`get_command_handlers()` → `tauri::generate_handler![]`), which `main.rs` passes to `invoke_handler`. Grouped by responsibility:

| Concern         | Modules                                                                                                                                                                                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| IPC surface     | `commands/` (sub-modules: `theme` / `cursor_build/` / `cursor_io` / `keystore` / `marketplace` / `marketplace_submit` / `profile` / `system` / `windows_scheme`)                                                                                                                   |
| Config / state  | `config/` (`schema` 型 / `store` RwLock + atomic_write + `config.corrupt.*.json` quarantine / `migrate` schema_version / `v1` / `patch`, Source of Truth), `errors.rs`, `cancel_registry.rs` (shared cursorpack-build / bulk-import cancellation registry App state)                                                                          |
| Cursor pipeline | `cursor/` (`image` / `cur_build` / `ico_cur` / `ani` / `ani_write`), `cursor_watcher.rs`                                                                                                                                                                                           |
| Registry        | `registry/` (`mod` / `scheme` / `roles` / `env` / `snapshot` / `transaction`) + `platform/` (`CursorBackend` trait, `windows` / `noop` / `memory` 実装) |
| Theme packages  | `theme/`, `bulk_import/`, `backup.rs` (`.cursorprofile`)                                                                                                                                                                                                                           |
| Marketplace     | `marketplace.rs` (HTTP index fetch, SHA-256 + Ed25519 verify), `keystore.rs` (Ed25519 + DPAPI + `.cfkey` XChaCha20-Poly1305 + Argon2id)                                                                                                                                            |
| Reliability     | `health.rs` (startup-failure counter + rollback), `crash.rs`                                                                                                                                                                                                                       |
| OS integration  | `tray.rs`, `hotkey.rs`, `autostart.rs`, `appusermodel.rs`, `accessibility.rs`, `environment.rs` (RDP/Citrix detection). Multi-instance lock via `tauri_plugin_single_instance` (no custom module). Dark mode is handled on the frontend (`useUiTheme` composable).                 |
| Observability   | `logging.rs` (`redact_path` / `short_hash` PII helpers)                                                                                                                                                                                                                            |

**Full file-by-file detail: Obsidian vault `develop/easy-cursor-swap/reference/file_inventory.md` section 1 + `reference/backend-overview.md` (infra) + each `specs/<NN-slug>/<NN-slug>.md` `modules:` frontmatter. IPC catalog: `reference/ipc-catalog.md`. Counts: `reference/index.json` `measured_counts`.**

## Conventions

- **Comments and doc strings: Japanese.**
- Use `tracing::{info,warn,error,debug,trace}!` for logs; never `println!`. Always pass paths through `logging::redact_path` and hashes through `logging::short_hash` (12 chars).
- Errors propagate as `AppError`; IPC commands return `Result<T, AppError>`.
- Prefer `RwLock` over `Mutex` when read-heavy (e.g. `config/store.rs`).
- Use `tokio::task::spawn_blocking` for blocking I/O in async contexts (ZIP extraction, large file scans).
- OS カーソル機構へは `State<'_, SharedBackend>` (`platform::CursorBackend`) 経由でのみ到達する。`RegistryManager` を commands / theme / main から直接呼ばない。

## Commands

Operate inside `src-tauri/` (or via `--manifest-path` from repo root).

```bash
cargo check # Linux でも通る (lib + bins)。テスト実行・カバレッジは Windows のみ
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib                                  # all unit tests
cargo test --lib cursor::ani_write::tests::name   # single test
cargo bench                                       # criterion benches in benches/
```

## Adding a `#[tauri::command]`

1. Implement the function in `src/commands/<sub-module>.rs`. Use snake_case in Rust; the mirroring TS payload type in `app/types/` uses camelCase (serde rename if needed).
2. Register it in the `tauri::generate_handler![]` list inside `get_command_handlers()` in `src/commands/mod.rs` (the only place `main.rs` reads it from; `lib.rs` holds no handler list).
3. Add the matching payload type in `app/types/`.
4. Add a `tracing::info!` log on entry; redact any PII.
5. Update the owning `specs/<NN-slug>/<NN-slug>.md` `ipc:` frontmatter + `reference/ipc-catalog.md` table + `reference/file_inventory.md`, then run `node scripts/gen-architecture.mjs` to regenerate `reference/index.json`. `verify-gate.sh` runs `gen-architecture.mjs --check` and goes red if registered IPC (from `generate_handler![]`) drifts from the frontmatter ownership maps.

## Hard rules (backend-side)

- **HKCU only.** Never touch HKLM or anything that triggers UAC.
- **Apply is transactional (2 recovery paths).** Snapshot to `~/.custom_cursors/_pending_apply.snapshot` before mutating; delete on success. In-process write failure rolls back to the pre-apply values via `restore_roles` (`RoleStore` seam in `registry::transaction`). A leftover snapshot on startup means an interrupted apply (likely a crash) and the registry may be mixed, so recovery resets to **Windows default via `platform::recover_pending_snapshot_on_startup` — not the pre-apply values** (intentional safety choice).
- **PII redaction is mandatory.** Raw registry values and full SHA-256 must never appear in logs.
- **Archive sanitisation.** Any unzip path must go through `theme::sanitize_archive_path` with the documented size limits (50 MB compressed / 200 MB expanded / 10 MB per image / 1 GB total).

## Pitfalls

- The `zip` crate v2.6.x is yanked — pin a known-good version when bumping.
- `cargo test --lib` is the canonical test runner for the verification gate; integration tests in `tests/` are not run by `verify-gate.sh`.
- `winreg` は `[target.'cfg(windows)'.dependencies]` 限定。非 Windows では `registry/` の I/O 関数が `#[cfg(not(windows))]` スタブ (`AppError::Registry`) になる。新しい winreg / windows 呼び出しは必ず `#[cfg(windows)]` 関数に閉じ、対になるスタブを書く。CI `rust-check-linux` (ubuntu, `cargo check --lib --bins` + clippy) が `-D warnings` で検出する。
