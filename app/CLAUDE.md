# app/ — Frontend (Nuxt 4 / Vue 3 SPA)

This file is loaded automatically when working under `app/`. Root `../CLAUDE.md` is also loaded — **read it first** for cross-cutting invariants (HKCU only / transactional apply / PII redaction / archive sanitization / no v-html), the verification gate, and the documentation update policy.

## Layout

- `pages/` — `index.vue` (Library), `creator.vue`, `marketplace.vue`, `settings.vue` (4 pages; helpers in `index.helpers.ts` / `marketplace.helpers.ts`).
- `components/{shell,library,creator,marketplace,settings,preview,panic,icons,ui}/` — domain-grouped SFCs. `nuxt.config.ts` sets `pathPrefix: false`, so reference components by file name (`<ThemeCard>`, not `<LibraryThemeCard>`). **Filenames in `components/` must stay globally unique.**
- `composables/` — IPC wrapper (`useTauri`), domain state (themes / settings / keystore / updater / notify / ui-theme), Creator helpers, marketplace helpers, UI utilities. Vitest specs in `composables/__tests__/`. **Full list with descriptions: `docs/architecture.json` → `frontend.composables[]` and `docs/file_inventory.md` section 2-3.**
- `locales/{ja,en}.ts` — keys typed `as const`; **must stay in parity** (CI gate via `scripts/check-i18n.mjs`).
- `types/` — IPC payload types (`config.ts`, `theme.ts`, `marketplace.ts`, `githubAuth.ts`). Must mirror `serde`-derived Rust structs in `src-tauri/src/commands/`.

## Conventions

- SFC + `<script setup>` + Composition API + TypeScript.
- IPC: always go through the `useTauri` composable; do not call `@tauri-apps/api`'s `invoke()` directly from components.
- Prefer extending an existing composable over introducing duplicate code. Add a Vitest spec for any non-trivial logic.
- **No `v-html` anywhere** — SVG icons go through render functions in `UiIcon.vue` / `CursorIcon.vue`.
- **i18n parity is a CI gate.** Adding a key to one locale without the other fails `scripts/check-i18n.mjs`.

## CSS (Tailwind v4)

Tailwind v4 utility classes are the default styling mechanism.

- **Design tokens** live in `assets/css/tailwind.css` (`@theme` block), aliasing legacy `--*` tokens.
- **Cross-cutting shared utilities** (`.btn`, `.card`, `.chip`, `.input`, `.tag`, `.toolbar`, `.tabs`, `.prop-section`, `.lib-row`, `.lt-*`, `.modal*`, `.content`, `.page-head`, `.grid`, etc.) are defined at the top level (unlayered) of `assets/css/tailwind.css`. **Do not** wrap them in `@layer components` — Tailwind preflight (e.g. `button { color: inherit }`) is emitted unlayered, so rules inside `@layer` lose the cascade.
- **Component-specific styles** belong in each `.vue` file's `<style scoped>`. Declare `@reference '~/assets/css/tailwind.css';` at the top, then use `@apply`.
- `assets/css/global.css` is **strictly limited** to `:root` tokens, CSS reset, scrollbar customisation, `:focus-visible`, `prefers-reduced-motion`, shared `@keyframes` (pulse / fade-in / slide-in-right / spin), and `html.light` token overrides. Do not add component-specific styles here (Phase 10-12 collapsed it from 3327 → 223 lines).

## Nuxt-specific pitfalls

- **`routeRules: { '/**': { ssr: false } }`** in `nuxt.config.ts`is intentional. Nuxt 4.4.4 has an IPC bug with`ssr: false`at the top level; do not change to`ssr: false` directly.
- `npm run dev` (Nuxt-only) has no Tauri runtime — IPC calls will fail. Use `npm run tauri:dev` from the repo root to exercise IPC.

## Visual regression — Tauri MCP

When you (Claude) edit `app/**/*.vue` or `app/**/*.ts` and `tauri:dev` is running, capture before/after snapshots of the active page to confirm no unintended visual or DOM change:

1. **Before edit** — `mcp___hypothesi_tauri-mcp-server__webview_screenshot` + `mcp___hypothesi_tauri-mcp-server__webview_dom_snapshot` (save to `c:/tmp/visual/before-<page>.png|json`).
2. **After HMR** — same two calls into `after-<page>.png|json`.
3. **Compare** — DOM diff (refactors should be zero diff). Image diff via `compare` (ImageMagick) when intentional design changes; review the pixel diff to confirm scope.

Skip for trivial single-line CSS tweaks; mandatory for component splits, modal/layout changes, and composable-driven state refactors. The PostToolUse hook will remind you (`💡 Visual regression reminder:`) after each frontend edit.

## Test commands

```bash
npm test                                # vitest run (all)
npm run test:watch                      # vitest watch
npx vitest run app/path/to/file.test.ts # single file
npx vue-tsc --noEmit                    # type check
node scripts/check-i18n.mjs             # i18n parity (CI gate)
```
