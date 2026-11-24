# AUDIT: ui/src/screens/player/tools/equalizer/harness.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A03) |
| Date | 2026-11-24 |
| Lines / functions | 115 / 4 tests (2 harness + 2 CSS pins) |
| File verdict | SHALLOW (harness half) + REAL (CSS-pin half) — file-level: REAL with one vacuous test |
| Reachable from shipped entry point? | Vitest lane YES (baseline ui-test.txt:133, 4 tests, 52ms). Harness components are dev-only by design; the CSS pins guard the REAL EqualizerTool sheet |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | it-count 4 == baseline; sed quote verify |

## 1. One-paragraph verdict
Split personality, honestly labeled: a dev-only vision-harness smoke (render two harness
components without crashing) + an env-gated HTML-snapshot exporter + two strong CSS contract
pins (the g15 rail-pan fix and the j10 P3-6 data-preset keying, including a negative assertion
banishing :nth-child keying). The exporter is vacuous in every normal CI run — its body only
executes when W26C1_HARNESS_OUT is set — and the baseline's "4 passed" therefore counts a
no-op among them.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:133 `✓ src/screens/player/tools/
equalizer/harness.test.tsx (4 tests) 52ms` (1705-passed run); 4 it() blocks on disk match.
The two CSS-pin tests read the real committed equalizer.css (:89-92) — they fail on drift.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/equalizer/harness.test.tsx:50-52
Code:
  it("exports standalone HTML snapshots when W26C1_HARNESS_OUT is set", () => {
    const out = process.env.W26C1_HARNESS_OUT;
    if (out === undefined || out === "") return; // normal runs skip the export
What is wrong: without the env var the test body returns immediately — it asserts nothing and
passes vacuously in every committed gate (no gate sets W26C1_HARNESS_OUT; rg over
.github/workflows + scripts: none). The export path's real behavior (clicks, inlining,
writeFileSync, existence asserts) is only exercised in a human-driven contract §5 loop.
Why it matters: "4 passed" overstates — 1 of the 4 is a no-op in CI. What correct looks like:
vitest `it.skipIf(!env)` or an explicit #[ignore]-style marker so the lane counts honestly.
Confidence: HIGH — Verification: read + rg (no gate sets the variable).
F-002 [S4]
Where: ui/src/screens/player/tools/equalizer/harness.test.tsx:54-57
Code:
      return existsSync(path) ? readFileSync(path, "utf8") : `/* missing: ${file} */`;
What is wrong: a missing CSS file silently degrades to a comment in the exported snapshot —
the headless comparison loop would screenshot an unstyled harness with no loud failure.
What correct looks like: throw on missing file (the harness is a vision tool; silent
un-styling defeats it). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| renders all frame states (42) | SHALLOW | — | crash-guard: innerHTML.length > 0 only — by design for a vision smoke |
| exports HTML snapshots (50) | VACUOUS-in-CI | S4 | F-001 |
| rail shrink+pan pins (94) | REAL | — | width/max-width 748/overflow/scrollbar pinned |
| preset glyph keying (104) | REAL | — | data-preset markers + :nth-child ban |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash guard (42) | only on throw | no behavioral assertions |
| exporter (50) | never in CI | env-gated no-op (F-001) |
| CSS pins (94,104) | YES | regexes bind to the committed sheet; the :nth-child ban catches the pre-k10 regression shape |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "dev-only vision harness renders every frame state without crashing" | header 1-2 | REAL | test 42 (smoke-grade, as claimed) |
| "writes standalone HTML snapshots … for the contract §5 comparison loop" | header 2-4 | REAL-UNPROVEN | export path only runs under W26C1_HARNESS_OUT; the loop record lives in the lane log (not re-run here) |

## 7. Dead / duplicated / unreachable code
HARNESS_STATES fixture consumed by both harness tests; CSS_FILES consumed by the exporter.

## 8. Dependencies & flags
In-repo + node:fs/path (dev-only export). The env gate W26C1_HARNESS_OUT is set by NO
committed workflow or script (rg: zero).

## 9. Open questions for the next agent
Did the w26.b10-pattern comparison loop (contract §5) ever run committed evidence for the EQ
harness, or is "the lane log" a local record like the visual matrix's pre-CI pins?
