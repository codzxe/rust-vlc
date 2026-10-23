# NEXT-STEPS — card 4.A1/4.A2 successors

Session 1 (agent P4-A1) delivered: Vite + React + TS-strict scaffold, vitest +
RTL, ESLint flat config with two local rules, mock command transport +
ClientProvider hook, routing skeleton for all doc 08 §1.6 routes, Playwright
config. typecheck/test/lint green.

Session 2 (agent P4-A2) delivered the token pipeline (doc 08 §3):
`ui/scripts/gen-tokens.mjs` (zero deps) syncing the crates/theme-tokens
output (`themes/generated/tokens.css`) into `src/tokens/generated/` (CSS copy
+ typed accessors + sha256), `ThemeProvider`/`useTheme` (dark/light/auto × 9
accents via `data-theme`/`data-accent` on `<html>`), the
`local/semantic-tokens-only` lint rule, a `/dev/tokens` sandbox fixture, 20/20
unit tests + 4/4 e2e (browsers were already cached; added the missing
`baseURL` to playwright.config.ts — latent 4.A1 gap). Decision record:
`src/tokens/README.md`.

## Do next (roughly in order)

1. ~~**4.A2 completion pass**~~ — DONE (closure session 3, agent P4-A2
   relaunch 2): full 9-accent × light/dark visual matrix E2E (18 tests,
   computed-style assertions vs values from themes/tokens.json) green;
   `npm run tokens:check` green (sha256 832da9d0… unchanged — no Rust-side
   change, no re-pin needed); worklog entry filed; Rust-consumer survey
   written into `src/tokens/README.md` (next item).
2. ~~**Rust-constants side**: survey consumers~~ — DONE, survey only, see
   `ui/src/tokens/README.md` § "Rust-constants consumer survey": zero
   current consumers of `crates/theme-tokens/src/generated/tokens.rs`;
   would-be consumers = renderer clear/letterbox color (today hardcoded
   `color::BLACK`; `backend.rs` already takes `background: Rgba` as a
   param — the ARCH-3-correct seam) + future subtitle-safe margins (no
   margin constants exist yet). Lead gates any engine-side dep.
3. **Settings wiring**: ThemeProvider currently defaults to dark + orange
   with NO persistence — wire the "Accent color" + theme settings through
   the settings schema (doc 07) when those cards land (4.E2); `ui/src/theme/`
   is the seam.
4. **ESLint rules v2**: promote the three inline rules in `eslint.config.js`
   into a proper local plugin (e.g. `tools/eslint-plugin-lumen/`) with real
   schema + tests; template-literal coverage for
   `local/semantic-tokens-only` (currently regex-grade on quasis only);
   Stylelint for plain `.css` files (doc 08 §3.4) — TokenSandbox.css is
   var()-only by construction, not by enforcement.
5. **Command client growth**: extend `src/client/transport.ts` — requestId
   allocation, pending-request map, event dispatch, stale-generation
   handling (doc 05 §4); keep the `src/generated/` import seam in one file.
   Add a `useCommand` hook. Real fixture replay comes from doc 05 §7 testkit.
6. **Screen stubs**: split ScreenPlaceholder into real per-screen modules as
   4.B primitives land. The `/dev/tokens` route is a dev fixture, NOT a
   product screen — do not copy its pattern into product screens.
7. **CI UI job** (Lead-owned at merge): propose `.github/workflows/ui.yml` —
   `working-directory: ui`, steps: `npm ci`, `npm run typecheck`, `npm run
   lint`, `npm run tokens:check`, `npm run test`, then Playwright with
   cached browser install.
8. **Directory drift note**: doc 08 §8 wants `src/bindings/` + `src/app/`;
   current scaffold has `src/generated/` (1.D4 output path — don't move it
   without updating `cargo xtask codegen`) and root-level `App.tsx`/
   `main.tsx`. Reconcile when components land.

## Honest caveats from session 2 (4.A2)

- `react-refresh/only-export-components` now also warns on `useTheme` beside
  `ThemeProvider` — same accepted provider/hook-pairing warning as 4.A1.
- vitest stubs raw `.css` imports, so the generated CSS text is also carried
  inside `tokens.ts` (`TOKENS_CSS`) for the snapshot test — the browserside
  cascade still loads the real `tokens.css` via `main.tsx`.
- `local/semantic-tokens-only` v1 misses: CSS-in-`<style>` tags, non-literal
  (computed) color strings, px on non-font-size/radius keys. Documented in
  the rule comment; v2 promotion tracked above.
- `themes/generated/tokens.css` was NOT regenerated this session (Rust side
  untouched, read-only) — the pipeline consumes the committed 1.B4 output.

## Honest caveats from session 1 (4.A1)

- Node 24.21.0 / npm 11.19.0; registry reachable (PONG 201ms).
- Vite 8 / vitest 5 / TS 6 / ESLint 10 are the current majors — if any behave
  oddly, pinning down one major is acceptable, record why here.
