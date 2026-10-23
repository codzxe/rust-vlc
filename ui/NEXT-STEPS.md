# NEXT-STEPS — card 4.A1 successors

Session 1 (agent P4-A1) delivered: Vite + React + TS-strict scaffold, vitest +
RTL (9/9 green), ESLint flat config with the two local rules (verified live),
mock command transport + ClientProvider hook, routing skeleton for all doc 08
§1.6 routes, Playwright config (no browsers). typecheck/test/lint all green.

Closure session (P4-A1 relaunch 2, 2026-10-23): chromium installed
(156.0.8078.4), smoke spec GREEN 1/1 (needed `baseURL` added to
playwright.config.ts — the scaffold config had omitted it), HTML reporter
added for the CI artifact, `.github/workflows/ui.yml` PROPOSED (Lead wires it
at merge), README brought to closure state. All four gates green.

## Do next (roughly in order)

1. ~~Playwright browsers~~ DONE (closure session) — smoke green on chromium;
   matrix per doc 08 §9.3 still future work.
2. **ESLint rules v2**: promote the two inline rules in `eslint.config.js` into
   a proper local plugin (e.g. `tools/eslint-plugin-lumen/`) with real schema +
   tests; cover template literals and literal props on presentation components
   with the technical-identifier exemption list the old placeholder sketch
   carried (`className`, `data-testid`, `icon`, `key`, `ref` — doc 07 §6.5);
   consider `no-restricted-imports` for Tauri APIs (doc 08 §8). The legacy
   `ui/.eslintrc.json` sketch was deleted at s1 (ESLint 10 ignores eslintrc;
   flat `eslint.config.js` is live) — its content is in git history.
3. **Command client growth**: extend `src/client/transport.ts` — requestId
   allocation, pending-request map, event dispatch, stale-generation handling
   (doc 05 §4); keep the `src/generated/` import seam in one file. Add a
   `useCommand` hook. Real fixture replay comes from doc 05 §7 testkit.
4. **Screen stubs**: split ScreenPlaceholder into real per-screen modules under
   `src/screens/` as cards land (4.A2 tokens first — placeholder text already
   flows through useMsg()/MessageKey so i18n stays clean).
5. **CI UI job** (Lead-owned at merge): PROPOSED at `.github/workflows/ui.yml`
   in branch wip/p4-a1 (closure session) — `working-directory: ui`,
   setup-node 24 + npm cache, `npm ci`, then typecheck/lint/test/e2e with
   `if: always()` keep-going aggregation, playwright browser cache, HTML
   report artifact. Lead reviews pins + wires it.
6. **Directory drift note**: doc 08 §8 wants `src/bindings/` (generated client)
   + `src/app/` (root providers/router); current scaffold has `src/generated/`
   (1.D4 output path — don't move it without updating `cargo xtask codegen`)
   and root-level `App.tsx`/`main.tsx`. Reconcile when components land.
7. **README**: keep the dependency table in sync (AGENTS.md audit rule) — every
   new package needs name/version/license/why.

## Honest caveats from session 1

- `react-refresh/only-export-components` warns on `useClient` living beside
  `ClientProvider` (provider/hook pairing) — accepted warning, not suppressed.
- `ui/.eslintrc.json` from the placeholder era was superseded by the flat
  `eslint.config.js` (ESLint 10); the i18n-allowlist entry format is now
  `{ text, reason, owner }`.
- Node 24.21.0 / npm 11.19.0; registry reachable (PONG 201ms).
- Vite 8 / vitest 5 / TS 6 / ESLint 10 are the current majors — if any behave
  oddly, pinning down one major is acceptable, record why here.
