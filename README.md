# EasyCursorSwap

**Next-generation mouse cursor manager for Windows**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2022H2%2B-blue)](https://github.com/nishiuriraku/easy-cursor-swap)
[![Tauri](https://img.shields.io/badge/Tauri-v2-orange)](https://tauri.app)

[日本語版 README はこちら](README.ja.md)

---

EasyCursorSwap is a Windows-only desktop application for managing custom mouse cursor themes.
It lets you import, create, and switch cursor themes as `.cursorpack` files, with full support
for all 17 Windows cursor roles, 6 DPI sizes, and Ed25519-signed theme distribution.

## Features

- **Theme library** — Import `.cursorpack` files by drag-and-drop or file dialog; browse, filter, and sort your collection
- **One-click apply** — Writes all 17 cursor slots × 6 resolutions to the registry with atomic snapshot/rollback
- **Creator mode** — Build cursor themes from PNG/SVG images; assign hotspots; export signed `.cursorpack` files
- **Official index** — Browse and install Ed25519-verified themes from the curated community index
- **Panic button** — One-click restore to Windows default or the pre-install snapshot, at any time
- **Cursor size control** — Built-in 15-step slider that updates `HKCU\Control Panel\Cursors\CursorBaseSize` (DWORD 32-256), independent of the active theme. The slider stays editable only while Windows Accessibility cursor size is `1`; when Windows enlarges the pointer (ease-of-access pipeline active), the slider is disabled and a deep-link to Windows Settings is shown. Positions are intentionally not synchronized with the Windows Settings UI slider.
- **Tray resident** — Runs silently in the system tray; optional silent launch on OS startup
- **Security hardened** — Ed25519 signatures, ZIP bomb protection, magic byte validation, path traversal prevention, SVG sanitisation, PNG metadata stripping
- **Auto-update** — Background update delivery via signed Tauri Updater; major-version jumps require manual confirmation

## System Requirements

| Requirement  | Minimum                                                                  |
| ------------ | ------------------------------------------------------------------------ |
| OS           | Windows 10 22H2 (build 19045) or Windows 11                              |
| Architecture | x64 (ARM64 planned)                                                      |
| WebView2     | Evergreen runtime (built-in on Windows 11; auto-installed on Windows 10) |
| Disk space   | ~30 MB for the installer; ~100 MB typical for a theme library            |

> **Not supported:** Remote Desktop (RDP), Citrix, Windows Server, UAC Secure Desktop,
> lock screen, and multi-user simultaneous sessions.

## Installation

Download the latest installer from the
[Releases page](https://github.com/nishiuriraku/easy-cursor-swap/releases):

| File                           | Description                                  |
| ------------------------------ | -------------------------------------------- |
| `EasyCursorSwap_x64-setup.exe` | NSIS installer (per-user, no admin required) |
| `EasyCursorSwap_x64_en-US.msi` | MSI installer                                |

Both are signed with a minisign key (verified by the built-in updater).
See [docs/updater_signing.md](docs/updater_signing.md) for signature verification instructions.

> **SmartScreen notice:** Until the installer accumulates enough download reputation,
> Windows SmartScreen may show an "Unknown publisher" warning.
> Click **More info → Run anyway** to proceed.
> The app is signed via [SignPath Foundation](https://signpath.org/) (OSS code
> signing); see [docs/code_signing_policy.md](docs/code_signing_policy.md) for
> the full signing policy (team, privacy, build reproducibility).

### Auto Updates

EasyCursorSwap checks for updates on app startup if **auto-update is enabled**
in Settings → Updates. To respect GitHub's rate limits the check runs at most
**once every 24 hours**; subsequent launches within that window skip silently.

When a new release is found, a Windows Toast notification appears. Major
version bumps (e.g. v1.x → v2.0) are **suppressed** from the toast — they
require explicit user action through Settings → Updates so that breaking
changes are reviewed before installing.

All update payloads are Ed25519-signed (minisign) and verified against the
public key embedded in the app at build time before installation.

If the app fails to start 3 consecutive times, a Windows MessageBox offers
to **automatically download and reinstall the previous version**. The
fallback installer is signature-verified the same way before being launched
silently.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.82+; pinned via `src-tauri/Cargo.toml` `rust-version`)
- [Node.js](https://nodejs.org/) 20+
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows 11 includes this)

### Quick start

```bash
git clone https://github.com/nishiuriraku/easy-cursor-swap.git
cd easy-cursor-swap
npm install

# Run in development mode (Tauri dev window + Nuxt HMR)
npx tauri dev

# Type-check Rust only
cargo check --manifest-path src-tauri/Cargo.toml

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Production build → generates .msi + .exe in src-tauri/target/release/bundle/
npx tauri build
```

### Marketplace auto-submit (optional development setup)

The Marketplace auto-submit flow uses GitHub's OAuth Device Flow. To exercise it
in `npm run tauri:dev`, register a GitHub OAuth App at
<https://github.com/settings/applications/new> (no callback URL required) and
export its Client ID:

```powershell
$env:EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID = "Iv1.xxxxxxxx"
npm run tauri:dev
```

Without `EASY_CURSOR_SWAP_GITHUB_OAUTH_CLIENT_ID`, the app still builds and runs — only the
auto-submit IPC reports "OAuth Client ID not configured" and users fall back
to the manual submission flow.

### Project structure

```
easy-cursor-swap/
├── app/                        # Nuxt 4 frontend (SPA mode)
│   ├── assets/css/             # Design tokens + global CSS
│   ├── components/             # Vue SFCs (Composition API + <script setup>)
│   ├── composables/            # Shared reactive logic (useThemes, useAppConfig, …)
│   ├── locales/                # i18n keys: ja.ts / en.ts (must stay in parity)
│   ├── pages/                  # Route pages (index, creator, marketplace, settings, …)
│   └── types/                  # TypeScript interfaces for IPC payloads
├── src-tauri/                  # Tauri + Rust backend
│   ├── src/
│   │   ├── main.rs             # Entry point: tray, health check
│   │   ├── lib.rs              # Module declarations (23 modules)
│   │   ├── commands/           # Tauri IPC command handlers (52 endpoints across 9 sub-modules)
│   │   ├── config.rs           # Config manager (RwLock, schema migration, backups)
│   │   ├── cursor/             # PNG → .cur / .ani pipeline (6 sizes, hotspot, ANI read/write)
│   │   ├── registry/           # HKCU registry read/write, Schemes, SPI_SETCURSORS
│   │   ├── theme/              # Theme manager (.cursorpack import/export, sanitisation)
│   │   ├── bulk_import/        # Folder/file bulk resolve + cursorpack-for-creator parser
│   │   ├── marketplace.rs      # HTTP index fetch, SHA-256 + Ed25519 verification
│   │   ├── keystore.rs         # Ed25519 key generation, DPAPI encryption, .cfkey
│   │   ├── health.rs           # Startup failure counter, rollback detection
│   │   └── …                   # tray, logging, backup, accessibility, …
│   ├── benches/                # Criterion micro-benchmarks
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                       # Architecture, security, distribution, signing docs
└── .github/workflows/          # ci.yml / performance.yml / release.yml
```

## Architecture

EasyCursorSwap uses a layered architecture where **Rust is the single source of truth** for all
system state:

```
Vue (UI) ──IPC──▶ Tauri commands ──▶ Rust modules ──▶ Windows registry / filesystem
```

- The frontend communicates exclusively through typed IPC commands (`invoke()`).
- All registry writes are transactional: a snapshot is saved before each apply, and
  automatically rolled back on crash (detected on next startup via a pending-snapshot file).
- Cursor files live in `%USERPROFILE%\.custom_cursors\` and survive uninstallation.

See [docs/architecture.md](docs/architecture.md) for details.

## Security Model

| Layer               | Mechanism                                                               |
| ------------------- | ----------------------------------------------------------------------- |
| Theme integrity     | Ed25519 signatures (ed25519-dalek), key_id = SHA-256[:16] of public key |
| Private key storage | Windows DPAPI (`CryptProtectData`) — tied to the user account           |
| Key export          | XChaCha20-Poly1305 + Argon2id passphrase encryption (`.cfkey` format)   |
| Download safety     | SHA-256 hash check + 50 MB / 200 MB / 10 MB three-stage size limits     |
| Archive safety      | Path traversal prevention, symlink rejection, ZIP bomb detection        |
| Image safety        | PNG metadata stripping (eXIf, iTXt, zTXt), SVG sanitisation             |
| Transport           | rustls-tls (no OS TLS stack dependency)                                 |

See [docs/architecture.md#security](docs/architecture.md#security) for the full model.

## Submitting Themes to the Official Index

1. Create a cursor theme in Creator mode and export a signed `.cursorpack`.
2. Upload the file to a GitHub Release (or any stable CDN URL).
3. In EasyCursorSwap, go to **Index → Submit to Index**, fill in your GitHub username and the
   download URL, preview the entry JSON, then click **Open GitHub PR**.
4. The app opens GitHub's web editor pre-filled with your `entries/{id}.json`.
5. After the PR is merged, the CI pipeline validates the signature, SHA-256 hash, and
   VirusTotal scan before the entry appears in the public index.

See [docs/key_rotation.md](docs/key_rotation.md) if you need to rotate your signing key.

## Known Limitations

| Limitation                     | Notes                                                            |
| ------------------------------ | ---------------------------------------------------------------- |
| No `.ani` authoring            | Animated cursors can be imported but not created                 |
| No live preview                | Changes are applied immediately to the registry; no preview mode |
| No undo                        | Apply is intentionally one-way; use the panic button to restore  |
| UAC Secure Desktop             | Shows Windows built-in cursors during elevated dialogs           |
| Lock screen / sign-in screen   | Shows Windows built-in cursors                                   |
| Multi-user sessions            | Each Windows user account has independent cursor settings        |
| Remote Desktop (RDP)           | Not supported; cursor rendering is controlled by the RDP host    |
| ARM64                          | Not yet tested; x64 binary runs via emulation on ARM64 Windows   |

## Contributing

Pull requests are welcome. See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full workflow, but in short:

1. Run the verification gate: `bash scripts/verify-gate.sh`
2. Keep i18n keys in parity: run `node scripts/check-i18n.mjs` (must exit 0)
3. Follow the coding conventions in [CLAUDE.md](CLAUDE.md):
   - Rust comments in Japanese
   - Vue: Composition API + `<script setup>`
   - CSS: Tailwind v4 utility classes (see `app/assets/css/tailwind.css`)
   - No `v-html` (XSS prevention)

## Community

- [Support and questions](SUPPORT.md) — where to ask
- [Code of Conduct](CODE_OF_CONDUCT.md) — Contributor Covenant 2.1
- [Security policy](SECURITY.md) — vulnerability reporting via GitHub Private Vulnerability Reporting
- [Changelog](CHANGELOG.md) — release notes (Keep a Changelog format)

## License

MIT — see [LICENSE](LICENSE).
