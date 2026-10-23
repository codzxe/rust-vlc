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

Closure session (P4-A1 relaunch 2, 2026-10-23): chromium installed
(156.0.8078.4), smoke spec GREEN 1/1 (needed `baseURL` added to
playwright.config.ts — the scaffold config had omitted it), HTML reporter
added for the CI artifact, `.github/workflows/ui.yml` PROPOSED (Lead wires it
at merge), README brought to closure state. All four gates green.

## Do next (roughly in order)

> **4.B1 in progress** (agent P4-B1, session 1, commits `7c6eb1d` + `a79e9f7`):
> ALL components landed — button trio (`Button` default/primary variants,
> sizes 28/32/48; `IconButton` mandatory `aria-label`; `Toggle`
> `role="switch"`+`aria-checked` per doc 08 §4.5 — the task-card blurb said
> `aria-pressed`, doc wins, deviation recorded in
> `src/components/button/states.md`) AND form inputs (`TextField`,
> `NumberField` with clamping steppers, `Checkbox`, `RadioGroup` with roving
> tabindex + arrow keys, `Switch` = Toggle re-export). All states
> (hover/pressed/focus-visible/disabled/loading/error) token-only CSS; 62
> component tests (RTL behavior + DOM snapshots + CSS-source→semantic-token
> assertions via node:fs); full suite 82/82, typecheck + lint clean.
> **Remains to CLOSE 4.B1 (next session)**: (1) Toggle micro labels
> (`--font-size-micro`); (2) token-seam decisions: 3px knob radius, rx5 input
> radius, dedicated `--color-danger` (currently `--color-record`) — raise with
> token owner, don't hack locally; (3) full state-matrix audit vs doc 08 §4.4
> (loading on form inputs?); (4) worklog entry (---, Task ID: 4.B1) recording
> the visual-snapshot adaptation: 4.G1 harness absent + vitest stubs `.css`
> imports (`?raw` returns EMPTY; `new URL(rel, import.meta.url)` throws in
> vitest — use `import.meta.dirname`), so "visual snapshot" = DOM snapshot +
> CSS-source→token assertions. Swap for real screenshots when 4.G1 lands.

1. **4.A2 completion pass (later session)**: worklog entry (---, Task ID:
   4.A2) once the remaining items land; full 9-accent × light/dark visual
   matrix verification (unit tests already assert structure; add a
   Playwright loop over all 9 accents); check `cargo xtask tokens` still
   regenerates byte-stable after any themes/tokens.json change and re-pin
   `PINNED_SHA256` in `src/tokens/tokens.test.ts` in the same commit.
2. **Rust-constants side**: survey consumers of
   `crates/theme-tokens/src/generated/tokens.rs` (engine-side needs);
   renderer clear colors / subtitle-safe margins per doc 08 §3.1 — Lead
   gates which crates consume them (ARCH-3: engine owns no UI state).
3. **Settings wiring**: ThemeProvider currently defaults to dark + orange
   with NO persistence — wire the "Accent color" + theme settings through
   the settings schema (doc 07) when those cards land; `ui/src/theme/` is
   the seam.
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
