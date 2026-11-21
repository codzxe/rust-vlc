# AUDIT: ui/scripts/web-matrix-run.mjs#part1 (lines 1–411)
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 411 (chunk 1–411 of 822) / 6 functions + 1 CLI branch |
| File verdict | REAL |
| Reachable from shipped entry point? | Dev tooling, not shipped: `npm run webmatrix` (ui/package.json:16). EXECUTION EVIDENCE EXISTS: the committed docs/web/browser-matrix-20261110.{json,md} were produced by this script's renderer (capture.recorded 2026-11-11T03:13:30.976Z, chromium lane 2 modes × 17 scenarios). NOT in any baseline run (needs a browser). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | sed quote verification; python json walk of the committed record (lane/mode/scenario counts); rg date/count literals; harness.js existence + wc; markdown-vs-record cross-read. |

## 1. One-paragraph verdict
Part 1 is the browser-matrix driver's config validation, the markdown report renderer, and the `--restamp` record-maintenance path. The engineering discipline is high: config validation fails loudly on 25 required keys plus type checks; the renderer is a deterministic pass over the JSON record; `--restamp` preserves capture facts verbatim and labels the regeneration honestly; the never-run Firefox/WebKit/Safari lanes are explicitly composed as BCD-cited expectations, never fabricated. Execution evidence is real: the committed markdown matches this renderer's output shape and the record's lanes (2 isolation modes × 17 scenarios, 10 pass each — counted from the JSON). The defects are renderer-literal drift: the capture DATE is hardcoded ("2026-11-10") in two places and already contradicts the record it renders (recorded 2026-11-11T03:13:30Z) — the committed report asserts both dates four lines apart; and two prose blocks embed facts not read from the record (a fix-status claim and the scenario count). An unguarded `modes[0]` index can crash `--restamp` on a degenerate record.

## 2. What actually works (verified)
- validateConfigShape (:47-118): 25 required-key paths, mode-entry shape, launch-arg `--` prefix, positive-integer knobs — a malformed config fails with `fail()` (exit 2) BEFORE any run; the duplicate-JSON-key rationale (:44-46) is sound.
- `--restamp` (:361-396): schema check, capture-facts preservation into `record.capture` (verbatim commit/branch/recorded + an explanatory note), git block re-derived, `recordRegenerated` block with reason (env-overridable), JSON + markdown re-rendered, exit 0. Bookkeeping only — matches its "no evidence production" charter (:352-360).
- Renderer honesty core: every table row is read from the record (scenario name/category/status/ms/evidence, wasm digests via dedup, lane evidence tiers); the harness line count is DERIVED by reading the file (:153-158 — 750 lines today; the committed report says 751, showing the file changed after the last render — the derived count is the right design); pipes escaped in cells (:133); evidence clipped at 220 chars with "full text in the JSON" disclosed (:249).
- Committed-artifact consistency: the record's chromium lane has 2 modes × 17 scenarios (10 pass each — python count), matching the markdown's per-mode "10/17 scenarios passed" and lane summary "playwright-run (REAL)"; other lanes render "NOT RUN — BCD-cited expectations only" (:226) — no fabricated lanes, exactly as the header promises (:14-16).
- `git.dirty: true` in the record renders as an explicit "DIRTY (uncommitted changes present)" provenance row (:193) — honest.

## 3. Findings
### F-001 [S3]
Where: ui/scripts/web-matrix-run.mjs:163 and :335 (contradicting the record at docs/web/browser-matrix-20261110.json:12)
Code:
```
    "# Browser matrix — the 2026-11-10 record (card 7.I1, issue #49)",
```
What is wrong: The capture date is a hardcoded literal in TWO renderer lines (:163 title, :335 "Capture date: 2026-11-10"), while the record it renders carries `capture.recorded: "2026-11-11T03:13:30.976Z"`. The COMMITTED markdown therefore asserts "the 2026-11-10 record" and "Capture date: 2026-11-10" while its own honesty note (rendered from the record, :172) says "captured 2026-11-11T03:13:30.976Z" — an internal contradiction shipped in docs/web/browser-matrix-20261110.md (title line vs line 8 vs line 178). Any future re-run or `--restamp` re-render keeps printing 2026-11-10 regardless of the actual capture.
Why it matters: This report's entire charter is provenance honesty ("it adds no facts of its own", :167; "Derived, never a literal", :153-155 — the line-count rule). A wrong capture date in the title of the provenance document is precisely the class of fabrication the renderer was built to prevent.
What correct looks like: `capture.recorded` is already in scope at :148-149 — derive both the title date and the Method date from it (e.g. `recorded.slice(0, 10)`).
Confidence: HIGH — Verification: read script + python-walked the record + read the committed markdown lines.

### F-002 [S4]
Where: ui/scripts/web-matrix-run.mjs:293-300 (also :236, :287)
Code:
```
        "  capture, fixed on w24/a after it (the fixture is now staged under the range-blind name",
        "  too); the fix awaits a browser re-run on a machine that has one.",
```
What is wrong: The fail-shape prose embeds historical FACTS that are not in the record: "fixed on w24/a after it (the fixture is now staged under the range-blind name too)" — a fix-status assertion that will print verbatim for ANY future capture whose `fetch-range-blind-server` scenario fails with the 404 signature (:272-274 gates only on name+status), mislabeling a regression as already-fixed. Same class: "the full 17-scenario conformance suite" (:236 — currently true, counted 17, but a literal where the suite length is data) and "pump loop exceeded 30000ms" (:287 — the budget is config-driven).
Why it matters: The renderer's charter is "adds NO facts — every value below is read from the record" (:124-126). Interpretation of present signatures is allowed; fix-status and counts are facts.
What correct looks like: Move fix-status to the record (a `notes` field maintained at capture time) or drop the tense; derive the scenario count from `scenarios.length` like the per-mode line already does (:220).
Confidence: HIGH — Verification: read; record contains no fix-status field (python walk).
### F-003 [S4]
Where: ui/scripts/web-matrix-run.mjs:234
Code:
```
      `Playwright ${chromiumLane.playwrightVersion}; user agent \`${chromiumLane.modes[0].userAgent}\`.`,
```
What is wrong: `chromiumLane.modes[0]` is unguarded while the neighboring lines use optional chaining (`chromiumLane?.modes?.[0]?.browserVersion` at :173 and :331). A record whose chromium lane has an empty `modes` array (hand-edit, partial write, schema drift) crashes `--restamp` with a TypeError instead of the script's own `fail()` diagnostics — and `--restamp` is the no-browser maintenance path most likely to touch odd records.
Why it matters: The script's stated philosophy is loud, clean config/record failures (:44-46); an index crash contradicts it and leaves the record half-maintained (JSON already written at :390 before the markdown render at :392 would crash mid-way, splitting the two artifacts).
What correct looks like: `chromiumLane?.modes?.[0]?.userAgent ?? "unknown"` (the :173 pattern), or validate the lane shape in the `--restamp` schema check.
Confidence: HIGH on the crash path (read), MEDIUM on real-world likelihood (machine-produced records always carry modes).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module head/imports/config load | 1-42 | REAL | — | Zero-dep claim verified (node:* only); configPath read unguarded — ENOENT stack (loud, acceptable) |
| validateConfigShape (+ call :118) | 47-118 | REAL | — | Runs before --restamp too; fail() hoisted (:401) — works; 25 key paths + shape/type checks |
| MD_CLIP / mdCell | 128-134 | REAL | — | Whitespace collapse, clip, pipe-escape — correct |
| wasmResponsesDigest | 136-144 | REAL | — | Dedupes status/type/length across fetches |
| renderMatrixMarkdown | 146-350 | REAL | S3/S4 | F-001 dates, F-002 prose literals, F-003 index; otherwise deterministic record-driven render (verified against committed artifact) |
| --restamp branch | 361-396 | REAL | — | Capture-facts preservation verified against the committed record (capture block present with note); JSON-then-markdown write order is the F-003 blast radius |
| fail / resolveRepo / site consts | 398-411 | REAL | — | fail = exit 2; boundary: staging/server/playwright driver is part2 (A09) |

## 5. Test honesty
No unit tests target this script (ui/scripts has no *.test.* for web-matrix-run). Execution evidence is the committed record+markdown pair (real past run, 2026-11-11, dirty tree disclosed). Would a broken renderer be caught? Only by a human diffing markdown against JSON — the F-001 date contradiction survived exactly because nothing re-derives the dates.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Firefox/WebKit/Safari lanes are NEVER fabricated" | :14-16 | REAL | Committed record: those lanes carry evidence:"bcd-cited" + NOT RUN rows (:226 renders them) |
| "a deterministic renderer over the SAME object the JSON serializer emits, so the two artifacts can never disagree" | :122-124 | FALSE in practice | F-001 — the artifacts already disagree on the capture date (md says 2026-11-10; json says 2026-11-11T03:13:30Z) |
| "adds NO facts — every value … read from the record" | :124-126, :167 | FALSE (two prose blocks) | F-002 |
| "Derived, never a literal: the harness line count" | :153-155 | REAL | :156-158 reads the file (750 today; committed report's 751 shows the derived design working as intended) |
| "--restamp … capture facts are preserved verbatim, never re-simulated" | :23-26 | REAL | record.capture block present; verified |
| "Exit code: Playwright's, propagated verbatim; --restamp exits 0" | :27 | part2 cross-ref | Exit at :395 confirmed for restamp; playwright propagation in part2 |

## 7. Dead / duplicated / unreachable code
None in this chunk. (The `spawn`/`spawnSync`/`createHash`/`createReadStream` imports at :29-31 are used only in part2 — cross-chunk, not dead.)

## 8. Dependencies & flags
node:* only (verified — no npm deps); reads ui/e2e/web/matrix.config.json (25-key contract), docs/web/browser-matrix-*.json (schema `lumen.web.browser-matrix/1`), e2e/web/site/harness.js (line count). Env: WEB_MATRIX_CHROMIUM (part2), WEB_MATRIX_RESTAMP_REASON (:387). `--restamp` CLI flag. Not part of any baseline gate.

## 9. Open questions for the next agent
- Part2 (W15-A09, lines 412-822) owns the staging/server/playwright driver — the "range-blind staging defect fixed on w24/a" claim (F-002) should be verifiable against part2's staging code (is the fixture now staged under the range-blind name?).
- docs/web/browser-matrix-20261110.md/json are doc-path artifacts — if a docs auditor owns them, F-001's committed-artifact contradiction should be cross-counted once (owned here at the renderer).
- The record's `git.dirty: true` capture posture (uncommitted tree) — wave 12/13 web auditors may already have context; cross-ref only.
