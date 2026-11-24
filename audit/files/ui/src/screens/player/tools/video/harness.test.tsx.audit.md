# AUDIT: ui/src/screens/player/tools/video/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 151 / 6 tests across 2 describes |
| File verdict | SHALLOW (1 honest smoke + 1 env-gated vacuous pass + 4 real CSS pins) |
| Reachable from shipped entry point? | DEV-ONLY harness (file header L1: "the dev-only vision harness"); CSS pins target the shipped video.css rules |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "harness.test" audit/runs/baseline/ui-test.txt` → `110: ✓ src/screens/player/tools/video/harness.test.tsx (6 tests) 200ms`; Grep `W26C4_HARNESS_OUT` repo-wide → only harness.test.tsx + harness.tsx L11 (a usage comment) — NO workflow/script sets it |

## 1. One-paragraph verdict
Four real regression pins on video.css (frame geometry: label-only slider rows, field-sizing crop inputs, capped-list grammar, shared-trigger alignment ownership) and one honest smoke (all four harness components render non-empty HTML). The sixth test — the HTML snapshot exporter — returns before its first assertion unless W26C4_HARNESS_OUT is set, and NOTHING in the repo (workflows, scripts) ever sets it: in the committed vitest baseline it is a counted PASS that proves nothing.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L110 `✓ ... (6 tests) 200ms` (inside 1705 passed).
- CSS pins read the real shipped sheet: L107-148 regex/contains assertions over src/screens/player/tools/video/video.css (readFileSync L99-101), including a negative pin (comment-stripped sheet contains NO scoped `text-align: start`, L144-148).
- Smoke test asserts innerHTML non-empty for all four components (L51-55) — weak but real.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/harness.test.tsx:58-61
Code:
```
  it("exports standalone HTML snapshots when W26C4_HARNESS_OUT is set", () => {
    const out = process.env.W26C4_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
```
What is wrong: an early `return` before any `expect` — in every committed gate (CI never sets W26C4_HARNESS_OUT; repo-wide Grep finds the variable only in this test and a usage comment in harness.tsx:11) this test is a vacuous green counted as a pass in the baseline's 1705. The snapshot export path (mkdir/writeFileSync/fireEvent open-dropdown, L62-89) is therefore NEVER verified by any committed run; a broken export (bad path handling, wrong CSS inlining, broken dropdown-open logic L69-75) would ship silently. Why it matters: the vitest count inflates real coverage; the lane's "contract §5 comparison loop" this exporter feeds has no committed evidence either (UNVERIFIED — no lane log in tree audited here). What correct looks like: `it.skip`/`describe.skip` with an explicit reason so the runner reports skips instead of passes, or a dedicated npm script gate that sets the env var and asserts outputs in CI.
Confidence: HIGH — Verification: read + Grep repo-wide (2 hits only).

## 4. Per-function table (>100 LOC, test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| render smoke | 49-56 | REAL | — | four components, non-empty innerHTML |
| HTML snapshot export | 58-90 | VACUOUS-IN-GATE | S3 | env-gated early return; no setter in repo (F-001) |
| frame-contract CSS pins | 93-149 | REAL | — | 4 pins incl. negative pin on scoped text-align |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "renders all four tools ... without crashing" | WEAK-YES | only innerHTML.length > 0; a component rendering an empty div passes |
| "exports standalone HTML snapshots..." | NO (in any committed run) | early return before assertions (F-001) |
| 4 CSS pins | YES | regex pins over the real shipped sheet |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "with W26C4_HARNESS_OUT set it additionally writes standalone HTML snapshots ... for headless screenshotting in the contract §5 comparison loop" | header L2-5 | REAL-UNPROVEN | the write path exists (L62-89) but no committed run sets the env (Grep) — loop evidence UNVERIFIED |

## 7. Dead / duplicated / unreachable code
The export path is effectively dead-in-CI (F-001) though reachable manually.

## 8. Dependencies & flags
@testing-library/react, vitest, node:fs/path; dev-only harness components; shipped video.css.

## 9. Open questions for the next agent
- Where is the "lane log" with the w26.c4 comparison-loop record the header cites? Not in audit/ or docs I can see (UNVERIFIED).
