# CLAIM AUDIT: docs/web/browser-matrix-20261110.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 7 |
| REAL 5 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The markdown is a faithful render of its machine record (docs/web/browser-matrix-20261110.json):
I diffed every scenario row (names, statuses, ms values, error strings) between the two and
they match exactly. The machinery it cites exists (runner script, config, 750-line
harness). Its honesty posture is exemplary: 10/17 pass rate published with the full fail
cluster, unrunnable lanes marked not-runnable (never simulated), and the wasm artifact
sha256 recorded but not re-verifiable (target/ not committed).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "This report is RENDERED from that record by ui/scripts/web-matrix-run.mjs … adds no facts of its own" | :4-5 | REAL | python3 cross-diff of md tables vs docs/web/browser-matrix-20261110.json `lanes[chromium].modes[*].suite.scenarios`: all 17 rows × both modes match exactly (status/ms/error strings); ui/scripts/web-matrix-run.mjs exists |
| 2 | chromium lane = real playwright-run; Playwright 1.64.1; Chromium 157.0.8103.50 | :9-11 | REAL | JSON: playwrightVersion "1.64.1", browserVersion "157.0.8103.50", evidence "playwright-run", userAgent HeadlessChrome/157 — all present in the record |
| 3 | "10/17 scenarios passed" in both isolation modes | :45, :61, :87 | REAL | python3 count over the JSON: isolation-off pass=10 fail=7; coop-coep pass=10 fail=7 — exact |
| 4 | open fails with typed refusal `web.decoders_absent` — "the designed v1 posture, reported honestly" | :66, :109-115 | REAL | JSON error text verbatim: "unsupported (web.decoders_absent): the wasm32 module ships the engine core without the builtin decoders (the codecs crate's external engines do not build for wasm32-unknown-unknown)" — consistent with the codecs/rav1d wasm32 findings audited in rows-170-182 and crates/codecs/Cargo.toml:62 |
| 5 | 12 pump-timeout failures are downstream of the decoder-less open refusal, "not an independent defect" | :116-119 | REAL-UNPROVEN | the causal reading is sound (every timed-out scenario requires an open session; the JSON shows the same idle-state report string for all 12) — but it is an interpretation, not a re-run |
| 6 | wasm artifact 1294311 bytes, sha256 4a0f33…d87; fixture 425516 bytes | :33-35 | UNVERIFIED | target/wasm32-unknown-unknown/release/lumen_web.wasm not committed (expected); contentLength "1294311" appears 14× in the JSON (self-consistent); fixture size matches the committed fixture seen in memory-audit (:425516) |
| 7 | shipped COOP/COEP posture off by default (`serving.coopCoepDefault = false`) | :181-189 | REAL | ui/e2e/web/matrix.config.json:64 `"coopCoepDefault": false` — exact |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none internal — report and record are byte-consistent) | — | — |

## 4. Fabrication / inflation indicators
- None. The opposite: it publishes a 10/17 fail rate, marks 4 of 5 lanes NOT RUN
  (with the download-forbidden reason), records the capture tree as DIRTY, and labels
  its own restamp mechanism ("never re-simulated").

## 5. Matrix rows
Feeds web-column evidence for platform rows (web=not_started claims in rows 179-180 of
my chunk): the engine boots and negotiates WebGL2 in a real browser, but media decode is
refused (`web.decoders_absent`) — consistent with web=not_started everywhere in
feature-matrix web cells. No overclaim.

## 6. Open questions for the next agent
- Re-verify the wasm sha256 if a build lane exists; the artifact is not committed.
- harness.js is 750 lines vs the doc's "751" — trivial count drift.
