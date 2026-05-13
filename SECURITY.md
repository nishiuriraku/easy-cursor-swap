# Security Policy

EasyCursorSwap writes to the Windows registry, handles user-supplied cursor packs, and verifies cryptographic signatures. We take security reports seriously.

## Supported versions

Only the **latest `0.x` release** receives security fixes during the pre-1.0 phase. Once `1.0.0` ships, this table will be updated.

| Version | Supported          |
| ------- | ------------------ |
| 0.x     | :white_check_mark: (latest only) |
| < 0.1   | :x:                |

## Reporting a vulnerability

**Please do _not_ open a public GitHub issue for security reports.**

Use **[GitHub Private Vulnerability Reporting](https://github.com/nishiuriraku/easy-cursor-swap/security/advisories/new)** to submit a confidential advisory. This routes the report directly to the maintainer with end-to-end privacy.

When filing, please include:

- A description of the vulnerability and its impact
- Reproduction steps or a proof-of-concept
- Affected version(s) (commit hash or release tag)
- Your suggested fix, if you have one
- Whether you'd like credit in the published advisory

## Response timeline

This is a single-maintainer project, so timelines are best-effort:

| Stage | Target |
| --- | --- |
| Acknowledgement | within 7 days |
| Initial assessment / triage | within 14 days |
| Coordinated disclosure window | up to 90 days from acknowledgement |

If you don't hear back within 7 days, feel free to send a follow-up via the advisory thread.

## In scope

- The EasyCursorSwap desktop application (Tauri runtime + Rust backend + Vue frontend)
- The installer (NSIS / MSI) and Tauri Updater signature flow
- Sample assets and tooling shipped under this repo (e.g. `tools/`, `scripts/`)
- The `.cursorpack` / `.cursorprofile` archive formats and their validation logic

## Out of scope

- The separate marketplace index repository ([`nishiuriraku/easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index)) — report there
- The crash-report-worker private repo — report directly to the maintainer
- Third-party themes hosted outside the official index
- Vulnerabilities that require physical access to an unlocked Windows session
- Issues in dependencies that have not yet been disclosed upstream (please report them to the upstream project first)

## Disclosure policy

We follow **coordinated disclosure**: once a fix is released, we will publish a GitHub Security Advisory crediting the reporter (unless anonymity was requested) and update the [`CHANGELOG.md`](CHANGELOG.md) under the corresponding version.
