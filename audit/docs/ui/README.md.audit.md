# CLAIM AUDIT: ui/README.md
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Claims extracted | 12 |
| REAL 7 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 2 / UNREACHABLE 0 | (1 claim stale-but-framed) |
| Internal contradictions found | 1 |

## 1. Blunt verdict
The README is an honest snapshot of card 4.A1 that was never updated. Two of
its claims are now FALSE (the screens layout and the font-bundling gap), and
its most quotable line — "Nothing is faked" — describes a MockTransport that
`npm run dev` no longer uses: the dev shell runs DemoTransport, a simulated
engine with baked-in demo clips. Anyone reading this README to decide whether
the UI is wired to a real backend would be misled in the optimistic direction.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "typecheck ✅ · test 9/9 ✅ · lint 0 errors ✅ · e2e smoke 1/1 ✅" | README:19-20 | REAL (stale, framed "at card closure") | baseline: vitest 1705 passed / 1 skipped today |
| 2 | "screens/ ScreenPlaceholder (honest placeholders, doc 08 §1.6)" | README:46 | FALSE | rg ScreenPlaceholder in src -> no matches; screens/ holds convert, discovery, media-info, network-stream, not-found, player, playlist, settings, ... (real implementations) |
| 3 | "Only `system.list-commands` is simulated (empty listing); every other command returns ... `feature-not-exposed` ... Nothing is faked." | README:52-55 | FALSE (as a description of the dev shell) | transport.ts:72-80 still does this — but main.tsx:22+56 selectAppTransport -> app-transport.ts:75 `new DemoTransport()` in the browser; demo-transport.ts:64 ships DEMO_CLIPS. The dev shell IS a simulation. |
| 4 | "All versions pinned exact in package.json" | README:59 | REAL | package.json: react 19.3.1, vite 8.3.5, typescript 6.0.3, vitest 5.0.5 — all match |
| 5 | "`npm run tokens:check` is the drift guard for CI" | README:116 | REAL | package.json:23 `tokens:check: node scripts/gen-tokens.mjs --check` |
| 6 | "Font bundling (§6.4) still pending the typeface decision." | README:162 | FALSE | main.tsx:5 imports fonts.css -> InterVariable.woff2 bundled (my file audit #1); MATRIX-20261113.md:110 "repo-bundled type — no host font dependency" |
| 7 | "e2e smoke (chromium, green)" + seeded-regression proof via `npm run visual:seed` | README:4, 154-156 | REAL-UNPROVEN | scripts exist; not executed (playwright install forbidden by load discipline) |
| 8 | "Memory note ... `tsc` + `vite build` can be OOM-killed" | README:26-33 | REAL-UNPROVEN | plausible, not reproduced (no builds allowed) |
| 9 | "React + TypeScript (strict) ... noUncheckedIndexedAccess" | README:3, 14 | REAL | tsconfig not re-read; typecheck passes per baseline |
| 10 | Lint rules descriptions (3 local rules) | README:91-104 | REAL | eslint.config.js exists; semantic-tokens-only rule is my file #16 |
| 11 | "0 vulnerabilities reported by npm audit at install time" | README:85-86 | UNVERIFIED | historical claim, cannot re-run install |
| 12 | "Vite dev/build, vitest + RTL unit tests, Playwright e2e smoke" | README:3-5 | REAL | package.json scripts verified |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Nothing is faked" (dev transport, README:55) | app-transport.ts:73-75 "Browser context: the demo simulation (an honest simulated engine" | The demo simulation is real: DemoTransport with DEMO_CLIPS drives the browser dev shell |
| "Font bundling still pending" (README:162) | fonts.css:28-34 @font-face + MATRIX-20261113.md:110 | Fonts ARE bundled |

## 4. Fabrication / inflation indicators
No fabrication — the failures are staleness, not invention: closure-era status
numbers preserved verbatim ("test 9/9"), a layout section ("as built so far")
that describes a scaffold two eras gone, and a known-gaps list whose gap was
since closed without updating the doc. The dangerous line is #3: "Nothing is
faked" is technically true of MockTransport (which still exists for tests) but
the README's own dev command table points users at the demo simulation.

## 5. Matrix rows
Not a matrix chunk — n/a.

## 6. Findings (doc severity)
F-001 [S3] Claims #2/#3/#6 false or misleading today — the README actively
misdescribes the dev shell's simulation and the screens' maturity.
F-002 [S4] Claim #1 stale status block (framed as closure status, but 17 waves
of drift make it useless as a current-state reference).
