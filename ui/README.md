# ui/ — Lumen player UI workspace (card 4.A1 scaffold, doc 08 §8)

React + TypeScript (strict) application scaffolded per doc 08 §8. Vite dev/build,
vitest + RTL unit tests, Playwright e2e (config only — browsers not yet installed),
ESLint with two scaffold-local rules (`local/i18n-no-literals`, `local/boundary-imports`).

## Dev commands

| Command | What it does |
|---|---|
| `npm run dev` | Vite dev server (mock command transport) |
| `npm run build` | typecheck + production build to `dist/` |
| `npm run typecheck` | `tsc --noEmit` (strict, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`) |
| `npm run test` | vitest run (jsdom + RTL, `src/**/*.test.{ts,tsx}`) |
| `npm run lint` | ESLint flat config incl. the two local rules |
| `npm run test:e2e` | Playwright (needs `npx playwright install chromium` first — deferred, see NEXT-STEPS.md) |

Status at scaffold commit: **typecheck ✅ · test 9/9 ✅ · lint 0 errors ✅** (1 benign
`react-refresh/only-export-components` warning: `useClient` exported beside
`ClientProvider` — standard provider/hook pairing, accepted for now).

## Layout (as built so far)

```
ui/
  package.json  tsconfig.json  vite.config.ts  eslint.config.js  playwright.config.ts
  index.html                 shell entry (desktop Tauri + web shared, doc 08 §8)
  src/
    generated/commands.ts    1.D4 codegen output — NEVER hand-edit
    client/                  CommandTransport seam over the generated types,
                             MockTransport + ClientProvider/useClient context hook
    i18n/messages.ts         useMsg() + MessageKey stand-in (real packs: doc 07)
    screens/                 ScreenPlaceholder (honest placeholders, doc 08 §1.6)
    App.tsx main.tsx         routing skeleton + root providers
    test/setup.ts            RTL cleanup (vitest globals:false) + jest-dom
  e2e/smoke.spec.ts          Playwright smoke (browsers pending)
```

`src/client/transport.ts` is the **only** module importing `src/generated/` — the
doc 08 §8 `bindings/` seam in miniature. Only `system.list-commands` is simulated
(empty listing); every other command returns the typed `feature-not-exposed`
error envelope. Nothing is faked.

## Dependencies (AGENTS.md dependency-audit rule)

All versions pinned exact in `package.json`; `package-lock.json` committed;
`node_modules/` ignored.

| Package | Version | License | Why |
|---|---|---|---|
| react | 19.3.1 | MIT | UI framework (MP §2 React owns presentation only) |
| react-dom | 19.3.1 | MIT | DOM renderer |
| react-router-dom | 7.18.4 | MIT | Routing for the doc 08 §1.6 screen inventory |
| vite | 8.3.5 | MIT | Dev server + bundler (doc 08 §8 [PROPOSAL] toolchain) |
| @vitejs/plugin-react | 6.1.2 | MIT | Vite React/fast-refresh integration |
| typescript | 6.0.3 | Apache-2.0 | Strict typecheck |
| vitest | 5.0.5 | MIT | Unit test runner (doc 08 §9.1) |
| jsdom | 30.1.3 | MIT | vitest DOM environment |
| @testing-library/react | 16.3.3 | MIT | RTL (doc 08 §9.1) |
| @testing-library/dom | 10.4.2 | MIT | RTL peer dep (queries/matchers core) |
| @testing-library/jest-dom | 7.0.1 | MIT | DOM matchers (`toBeInTheDocument`) |
| @playwright/test | 1.64.1 | Apache-2.0 | e2e/visual/keyboard-nav (doc 08 §9.3/§9.6) |
| eslint | 10.13.0 | MIT | Lint incl. custom rules |
| @eslint/js | 10.0.1 | MIT | ESLint recommended base |
| typescript-eslint | 8.72.0 | MIT | TS-aware lint |
| eslint-plugin-react-hooks | 7.1.1 | MIT | rules-of-hooks / exhaustive-deps |
| eslint-plugin-react-refresh | 0.5.7 | MIT | fast-refresh export hygiene |
| @types/react | 19.3.0 | MIT | React types |
| @types/react-dom | 19.3.0 | MIT | react-dom types |

Resolved 2026-02-05 from registry.npmjs.org (network PONG 201ms), `npm install
--save-exact`, 0 vulnerabilities reported by npm audit at install time.
Card 4.A2 added ZERO packages — the token pipeline (scripts/gen-tokens.mjs,
ThemeProvider, generated CSS/TS) is pure node + react + CSS custom
properties.

## Lint rules (scaffold-local, v1)

- `local/i18n-no-literals` — flags JSX text nodes and literal `aria-label`/`alt`/
  `title` props (doc 07 §6.5: user-visible copy must come from `useMsg()` +
  `MessageKey`). Exemptions: `ui/i18n-allowlist.json` entries
  `{ "text": "...", "reason": "...", "owner": "..." }`. Test files skipped.
- `local/boundary-imports` — files under `src/` may not relatively import outside
  `ui/src`, and absolute-path imports are banned (doc 08 §8 import boundaries).
- `local/semantic-tokens-only` (card 4.A2) — raw `#hex`/`rgb()`/`rgba()` string
  literals and raw `px` on font-size/radius keys are rejected in TSX; colors and
  dimensions must resolve through `var(--semantic-token)` (doc 08 §3.2/§3.4).
  Generated and test files are exempt.

All three verified live against deliberate violations. v1 is
AST-literal/regex-grade (no template-literal coverage, no plugin package yet) —
see NEXT-STEPS.md.

## Token pipeline (card 4.A2, doc 08 §3)

`themes/tokens.json` (Rust-owned, card 1.B4) → `cargo xtask tokens` →
`themes/generated/tokens.css` → `npm run tokens:generate` (ui/scripts/
gen-tokens.mjs, zero deps) → `src/tokens/generated/{tokens.css,tokens.ts}`
(typed accessors + sha256). `npm run tokens:check` is the drift guard for CI.
See `src/tokens/README.md` for the full decision record. Theme switching:
`ThemeProvider` (`src/theme/`) sets `data-theme`/`data-accent` on `<html>` —
dark/light/auto × 9 accents, attribute-only per doc 08 §3.3.
