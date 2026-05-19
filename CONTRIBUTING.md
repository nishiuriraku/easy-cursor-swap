# Contributing to EasyCursorSwap

Thanks for thinking about contributing! This is a small, single-maintainer OSS project — every well-formed bug report, fix, or idea genuinely helps.

Before you dive in, please skim:

- **[Code of Conduct](CODE_OF_CONDUCT.md)** — we follow Contributor Covenant 2.1.
- **[SUPPORT.md](SUPPORT.md)** — for "how do I use this?" questions (different from contributing).
- **[SECURITY.md](SECURITY.md)** — for vulnerability reports (do **not** open a public issue).

## What kind of contribution?

| Kind                                       | Where to go                                                                                                                                     |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Bug report                                 | Open a [GitHub Issue](https://github.com/nishiuriraku/easy-cursor-swap/issues/new) with reproduction steps                                      |
| Feature idea                               | Open an Issue first to discuss before coding — saves both of us time                                                                            |
| Small fix (typo, broken link, obvious bug) | Open a PR directly, no prior issue needed                                                                                                       |
| Larger change                              | Open an Issue first, get rough alignment, then PR                                                                                               |
| Security vulnerability                     | Use [Private Vulnerability Reporting](https://github.com/nishiuriraku/easy-cursor-swap/security/advisories/new), see [SECURITY.md](SECURITY.md) |
| New theme for the marketplace              | Submit a PR to the separate [`easy-cursor-swap-index`](https://github.com/nishiuriraku/easy-cursor-swap-index) repo                             |

## Development setup

```bash
git clone https://github.com/nishiuriraku/easy-cursor-swap.git
cd easy-cursor-swap
npm install
npm run tauri:dev          # dev window + Nuxt HMR
```

Windows 10 22H2+ / Windows 11 with Rust 1.82+ is required. Full command reference: **[`CLAUDE.md`](CLAUDE.md)**.

## Project layout

- `app/` — Nuxt 4 / Vue 3 frontend (SPA)
- `src-tauri/src/` — Rust backend (registry, cursor pipeline, marketplace, keystore, etc.)
- `docs/` — living docs (`architecture.md` / `file_inventory.md`) + runbooks
- See [`docs/architecture.md`](docs/architecture.md) for the IPC layout and module responsibility map.

## Coding conventions

- **Rust comments and doc strings:** Japanese (existing convention — don't switch).
- **Vue:** SFC + `<script setup>` + Composition API + TypeScript.
- **CSS:** Tailwind v4 utility classes (see `app/assets/css/tailwind.css`). No `v-html` anywhere — XSS prevention.
- **Filenames in `components/`:** Globally unique (Nuxt's `pathPrefix: false`).
- **i18n:** Update both `app/locales/ja.ts` and `app/locales/en.ts` so `node scripts/check-i18n.mjs` stays green.

Full conventions are in [`CLAUDE.md`](CLAUDE.md).

## Commits

We use **[Conventional Commits](https://www.conventionalcommits.org/)**. Examples:

```
feat(library): support multi-select delete
fix(creator): preserve hotspot when re-importing
docs: clarify SECURITY.md disclosure window
refactor(registry): split scheme.rs into its own module
```

Common types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `style`, `build`, `ci`.

**One feature = one commit.** Squash WIPs before pushing.

## Verification gate (required before pushing)

```bash
bash scripts/verify-gate.sh
```

This runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --lib`, `prettier --check`, `vue-tsc --noEmit`, the i18n parity script, and `vitest`. CI runs the same set — passing locally means CI will pass.

For PRs that change packaging or installer behaviour, also run `npm run tauri:build`.

## Pull requests

1. Fork → feature branch (`feat/short-description` or `fix/short-description`).
2. Keep PRs focused. If you find unrelated cleanup along the way, ship it as a separate PR.
3. Update `CHANGELOG.md` under `## [Unreleased]` with a one-liner describing the change.
4. If the change affects user-facing behaviour, update the README in both languages (`README.md` and `README.ja.md`).
5. Make sure the verification gate is green.
6. Open the PR — fill in what the change does and why, plus any screenshots for UI changes.

The maintainer reviews on a best-effort basis (this is not a full-time project). Expect first feedback within ~1 week; ping the PR if a fortnight goes by silently.

## Licence

By submitting a contribution, you agree that your work will be licensed under the project's [MIT licence](LICENSE). You retain copyright to your contribution; the project distributes it under MIT.

We do **not** require a CLA or DCO sign-off at this stage.

## Thanks

If you've read this far, you're already the kind of contributor this project is glad to have. 🎉
