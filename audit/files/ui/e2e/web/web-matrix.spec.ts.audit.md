# AUDIT: ui/e2e/web/web-matrix.spec.ts
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / tests | 162 / 1 test per isolation mode (2 modes from matrix.config.json) |
| File verdict | REAL — design + committed run record; NOT in the PR e2e lane |
| Reachable from shipped entry point? | NO — e2e spec, excluded from `npm run test:e2e` (playwright.config.ts excludes e2e/web/**); runs only via `npm run webmatrix` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; rg for run evidence; read docs/web/browser-matrix-20261110.md |

## 1. One-paragraph verdict
A genuinely honest browser-lane spec: it drives the in-page conformance
harness against the REAL wasm Worker in Chromium once per COOP/COEP mode,
asserts the isolation truth matches the served headers, writes the
machine-readable fragment BEFORE the pass/fail verdict (137-140: "writing
fragments only on a green run would silently erase red evidence"), and
throws on any failed scenario. It refuses to fabricate: `requireEnv`
fails loudly if the webmatrix driver is absent, and the header documents
that Firefox/WebKit lanes are recorded as "not runnable in this sandbox,
never a fabricated result". The committed record (docs/web/
browser-matrix-20261110.json/.md) proves one real chromium Playwright run
(capture 2026-11-11, wasm sha256 recorded, mixed verdicts reported as
mixed).

## 2. What actually works (verified)
- Committed run evidence (docs/web/browser-matrix-20261110.md:9-22, verbatim):
```
- **The chromium lane is a real Playwright run** (evidence tier `playwright-run`):
  captured 2026-11-11T03:13:30.976Z against commit `3755dc350213f71b2031f50225841ece9f773a81` on branch `w24/a`,
```
and
```
- **Firefox / WebKit / Safari / Safari-iOS never executed** — no binaries exist in this
  sandbox and downloading is forbidden. Their lanes carry BCD-cited expectations and the
  next step (real machines / CI runners), never a fabricated result (AGENTS §3.4).
```
- Red-evidence preservation (136-140, 141-154): fragment written before the failed-scenario throw.
- Isolation-truth cross-check (125-135): observed `crossOriginIsolated` must equal the mode's configured headers.
- Config-driven: modes/pump/waits/tolerances from ui/e2e/web/matrix.config.json (62-64, exists in-tree).

## 3. Findings
F-001 [S3] — zero baseline execution; not wired into any gate
Where: ui/e2e/web/web-matrix.spec.ts (whole file) vs audit/runs/baseline/{rc-smoke,full_gate}.txt
What is wrong: rg finds no `webmatrix`/`browser-matrix` execution in the
orchestrator baseline (rc-smoke runs web-ui-build + browser-load, not this
spec), and the spec is excluded from the standard e2e config. Its only
execution evidence is the committed 2026-11-10 record, which is now ~2
weeks stale relative to HEAD and captured on branch `w24/a` (with a DIRTY
tree per the record's own honesty note) — not on main.
Why it matters: the web/wasm capability claims (matrix web column) rest on
a single dated, branch-local run; nothing re-verifies them per push.
What correct looks like: a scheduled CI lane running `npm run webmatrix`
(its own README records that as the intended next step).
Confidence: HIGH — Verification: rg over baseline logs + read of the
committed record.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| interfaces | 24-46 | REAL | typed result records |
| matrix config load | 50-64 | REAL | json file exists in-tree |
| requireEnv | 66-75 | REAL | loud failure naming the driver |
| per-mode test | 87-161 | REAL | wasm response capture (MIME evidence), in-page `__lumenWebMatrix.run`, isolation cross-check, fragment-first writes, fail throw |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| chromium lane per mode | YES | crossOriginIsolated mismatch throws; per-scenario failures throw with names; fragment records everything either way |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT part of npm run test:e2e ... needs the matrix harness server" | header :9-13 | REAL | requireEnv enforces; playwright.config exclusion documented |
| "never a fabricated result" (non-chromium lanes) | header :15-18 | REAL | only chromium test bodies generated; report builder cites BCD |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
@playwright/test, node:fs/path/url. Driven by ui/scripts/web-matrix-run.mjs (env: WEB_MATRIX_BASE_URLS / OUT_DIR / FIXTURE_BYTES). wasm lane requires the wasm build (rc-smoke baseline: wasm build passes with wasm32 rust-std component installed).

## 9. Open questions
- Does ui.yml or any workflow schedule `npm run webmatrix`? rg found none — verify with the CI auditor (workflows domain).
