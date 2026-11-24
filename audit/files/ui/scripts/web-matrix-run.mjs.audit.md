# AUDIT: ui/scripts/web-matrix-run.mjs
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 821 / 9 (validateConfigShape, mdCell, wasmResponsesDigest, renderMatrixMarkdown, --restamp block, firstExisting, parseRange, startServer, gitInfo + top-level driver) |
| File verdict | REAL (as dev tooling; honest about non-runnable lanes) — with a lying-date renderer defect |
| Reachable from shipped entry point? | NO — dev/e2e tooling by design; output record docs/web/browser-matrix-20261110.json feeds doc-09 evidence |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n` quote verification (3 blocks); `python3` parse of docs/web/browser-matrix-20261110.json (lanes/evidence/scenario statuses); `rg -n "decoders_absent|Fail shapes"` on the committed .md; `wc -c` on config/record/md |

## 1. One-paragraph verdict
This is the rare honest evidence tool: the committed record (docs/web/browser-matrix-20261110.json) really contains a chromium Playwright lane (2 modes, 10/17 scenarios passed, failures recorded verbatim — `open-play-pause-stop` fails with the typed `web.decoders_absent` refusal) and four non-chromium lanes explicitly marked `not-runnable-in-sandbox` with BCD-cited expectations, never fabricated results. The renderer, however, hardcodes capture-date facts ("the 2026-11-10 record", "it was NOT re-run to produce this report") as literals — any future re-run will emit a report that denies it happened, violating the renderer's own "adds NO facts" contract. Provenance also records the declared package.json semver as the Playwright version that ran.

## 2. What actually works (verified)
— verified by `python3` parse of the committed record: chromium lane `evidence: "playwright-run"`, 2 modes, 10/17 pass each, 7 real failure rows (pump-loop timeouts downstream of the decoder-less wasm posture), firefox/webkit/safari/safari-ios all `evidence: "not-runnable-in-sandbox"` with 0 modes. `rg` confirms `web.decoders_absent` and 12 `pump loop exceeded` occurrences in the JSON, matching the markdown's fail-shape interpretation gates (L266-274).

## 3. Findings
F-001 [S3]
Where: ui/scripts/web-matrix-run.mjs:163, 335-336
Code:
```
    "# Browser matrix — the 2026-11-10 record (card 7.I1, issue #49)",
```
and
```
    "- Capture date: 2026-11-10. The record you are reading is that capture (see the honesty",
    "  note); it was NOT re-run to produce this report.",
```
What is wrong: capture facts are hardcoded literals in the renderer instead of derived from `record.capture.recorded`. The renderer's contract (L122-126) says it "adds NO facts — every value below is read from the record" — these two facts are not read from the record. A future real re-run will produce a report titled "2026-11-10" asserting "it was NOT re-run", i.e. a false provenance statement baked into the generator. Note the committed record's `capture.recorded` is `2026-11-11T03:13:30.976Z` — the title date and the record's own capture timestamp already disagree (timezone drift at minimum). What correct looks like: `renderMatrixMarkdown` derives title and capture-date lines from `capture.recorded`. Confidence: HIGH — Verification: read + python3 parse of record.
F-002 [S4]
Where: ui/scripts/web-matrix-run.mjs:755
Code:
```
  playwrightVersion: pkg.devDependencies["@playwright/test"] ?? null,
```
What is wrong: records the declared semver RANGE from package.json (e.g. `^1.x.y`) as `playwrightVersion`, and the markdown renders "driving ... with Playwright ${chromiumLane.playwrightVersion}" — presenting a dependency spec as the version that executed. Why it matters: provenance block lies about tooling version on drift between package.json and node_modules/lockfile. What correct looks like: read the installed version (`node_modules/@playwright/test/package.json` version field). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: ui/scripts/web-matrix-run.mjs:523-524
Code:
```
    const filePath = path.normalize(path.join(siteDist, route));
    if (!filePath.startsWith(siteDist)) {
```
What is wrong: string-prefix traversal guard — a sibling directory whose absolute path begins with `siteDist` (e.g. `.../sitedist-evil`) passes `startsWith(siteDist)`. Blast radius minimal: server binds 127.0.0.1 (matrix.config.json `server.host`), serves only during the run, and the escape requires a sibling dir sharing the prefix. Correct form: compare against `path.dirname`/exact segment boundary. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| validateConfigShape | 47-117 | REAL | — | loud failure on malformed config; checks launch-arg prefixes, positive ints |
| mdCell | 130-134 | REAL | — | whitespace collapse, 220-char clip, pipe escape |
| wasmResponsesDigest | 136-144 | REAL | — | derived from responses, no literals |
| renderMatrixMarkdown | 146-350 | REAL w/ defect | S3/S4 | F-001 (hardcoded date/never-re-run), honest mixed-verdict prose otherwise |
| --restamp block | 361-396 | REAL | — | preserves capture verbatim, names regen reason; re-renders md with F-001 literals |
| firstExisting | 414-424 | REAL | — | candidate probe for wasm artifact |
| parseRange | 480-514 | REAL | — | RFC 9110 end-inclusive; multi-range → 200 full body with warn (disclosed) |
| startServer | 516-567 | REAL | S4 | range-aware/range-blind routes, per-mode COOP/COEP; F-003 prefix guard |
| gitInfo | 724-738 | REAL | — | records dirty-tree status honestly |
| top-level driver | 398-821 | REAL | — | stages site, spawns Playwright async (rationale documented L642-648), merges fragments, propagates exit code, warns on missing fragments |

## 5. Test honesty
No unit tests cover this script (only `e2e/web/web-matrix.spec.ts`, the in-browser suite it drives). Mutation reasoning: if `parseRange` were broken, the committed record's `fetch-range-blind-server` PASS row (status 200, bodyBytes 425516) would not exist — the record itself is the executed evidence. The renderer (F-001) has no test that would catch the date lie. UNVERIFIED: live Playwright run (no browser binary in this sandbox; forbidden to download — script itself documents this).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Firefox/WebKit/Safari lanes are NEVER fabricated" (L14-16) | header | REAL | record: 4 lanes `not-runnable-in-sandbox`, 0 modes |
| "it adds NO facts — every value below is read from the record" (L123-125) | comment | FALSE | F-001: 2026-11-10 title + "was NOT re-run" are literals |
| "the range-blind route was never staged as a file ... fixed on w24/a" (L295-299) | comment | REAL | record shows `fetch-range-blind-server` PASS (200, 425516 bytes); L468 stages the fixture under rangeBlindPath |
| "Exit code: Playwright's, propagated verbatim" (L27) | header | REAL | L821 `process.exit(playwrightStatus)` |

## 7. Dead / duplicated / unreachable code
None found. All branches reachable via CLI modes (run / --restamp).

## 8. Dependencies & flags
node: stdlib only (no new npm deps, verified import list L29-37). Reads env `WEB_MATRIX_CHROMIUM`, `PLAYWRIGHT_BROWSERS_PATH`, `CARGO_TARGET_DIR`, `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` set for child. Config-driven knobs per matrix.config.json (schema `lumen.ui.web-matrix/1`).

## 9. Open questions for the next agent
- docs/web/web-column-pass-20261110.md and the feature-matrix web column cite this record — doc agents should treat "web" platform statuses as 0-verified per the parity baseline; this record's honest 10/17 chromium pass does NOT verify web media playback (decoders absent by design).
- If a browser re-run ever lands, F-001 will silently produce a self-contradicting report — fix before that run.
