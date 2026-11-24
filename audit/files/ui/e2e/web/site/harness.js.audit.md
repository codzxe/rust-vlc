# AUDIT: ui/e2e/web/site/harness.js
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 750 / 1 class + 8 helper fns + runSuite (17 recorded scenarios) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (e2e test infra). Wired: ui/package.json `webmatrix` -> ui/scripts/web-matrix-run.mjs (821 lines, exists) -> ui/e2e/web/web-matrix.spec.ts -> `window.__lumenWebMatrix.run` (harness.js:738). Excluded from default `npm run test:e2e` by ui/playwright.config.ts:21 `testIgnore: ["**/visual/**", "**/web/**"]` — runs only via the dedicated webmatrix lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg for __lumenWebMatrix callers; cat matrix.config.json; read committed report docs/web/browser-matrix-20261110.md (chromium lane 10/17 x2 modes) |

## 1. One-paragraph verdict
A real, hostile-by-design in-page conformance harness: it drives a real module Worker, real wasm boot, real fetch transport, and asserts through EVENTS (state trails, seq monotonicity, envelope invariants) rather than command echoes, with typed error-path pinning (stage+code+capability) and redaction checks. It does not fake success: the committed browser-matrix report records 10/17 scenarios passing per mode with the 7 failures shown verbatim (happy-path lanes fail on the honest "web.decoders_absent" wasm build). This is the strongest test file audited so far in this assignment.

## 2. What actually works (verified)
- Read committed evidence docs/web/browser-matrix-20261110.md:45,54,61 — "chromium | playwright-run (REAL) | 10/17 (isolation-off), 10/17 (isolation-coop-coep) scenarios passed", full per-scenario tables (17 rows each mode) with timings and evidence JSON. Scenario names in the report match the 17 `record(...)` calls in harness.js (boot-handshake … audio-context-and-worklet), matching matrix.config.json runner comment "the suite runs 17 scenarios per mode".
- Failure honesty verified: open-play-pause-stop fails with the verbatim engine error `{"stage":"decode","code":"unsupported","message":"unsupported (web.decoders_absent)..."}` (report :67) — the harness surfaces the real engine limitation instead of passing vacuously.
## 3. Findings
F-001 [S3]
Where: harness.js:642-646
Code:
```
    const sampleA = lane.pumps()[lane.pumps().length - 1];
    await sleep(200);
    const sampleB = lane.pumps()[lane.pumps().length - 1];
    assert(
      sampleB.report.positionUs === sampleA.report.positionUs,
```
What is wrong: if zero pump reports have arrived (an engine that never pumps), `sampleA` is `undefined` and the scenario dies on a TypeError (`Cannot read properties of undefined`) instead of a clear assertion message. Why it matters: obscures the actual defect class in the failure row. What correct looks like: `assert(sampleA && sampleB, "pump reports arrived before the park freeze")`. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: harness.js:36,40
Code:
```
// (matrix.config.json owns the knob — W24-R8 P3-1), and runSuite asserts the
// knob is present before any scenario runs, so a config regression can never
let harnessTimeoutMs = 30000;
```
What is wrong: nothing functional — the fallback literal is pre-validated by runSuite (:243-248 asserts waits.timeoutMs integer > 0 before overwriting). Noted only to confirm the comment's claim IS upheld by code (verified lines 243-248). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: harness.js:214-229
Code:
```
    const last = lane.pumps()[lane.pumps().length - 1];
    if (last && !last.report.busy && slices < config.pump.maxSlices) {
      arm();
    }
    if (Date.now() > deadline) {
```
What is wrong: drivePump polls at 5 ms with a wall-clock deadline; between `arm()` and the deadline check it can post pump slices faster than the worker drains them when the engine repeatedly releases busy immediately (maxSlices 4096 posts possible in a tight loop bounded only by the 5 ms sleep — acceptable, bounded). Noted: the loop is mechanically bounded; no unbounded spin exists. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert / assertEq | 42-54 | REAL | — | message formatting correct |
| sleep | 56 | REAL | — | |
| observedEvents | 59-67 | REAL | — | flattens batches |
| stateTrail | 70-74 | REAL | — | arrival-order `.to` |
| EngineLane (class) | 81-146 | REAL | — | fresh module Worker; waiters matched against log too (race-safe: :100-104 existing-log find) |
| probeRenderer | 149-166 | REAL | — | real gpu/webgl2 probes, requestAdapter bounded by 2 s race |
| bootLane | 169-175 | REAL | — | posts boot + waits booted |
| fetchFixture | 178-182 | REAL | — | asserts response.ok |
| openFixtureLane | 185-197 | REAL | — | loadBlob -> blobLoaded -> session.open (class-A) |
| drivePump | 206-230 | REAL | S4 | bounded re-arm; see F-003 |
| runSuite | 242-735 | REAL | S3 | 17 scenarios; scenario failures become fail rows (never throw); malformed config throws loudly (:243-247); status element progress per scenario (:256-261) |
| 11 error-path / invariant scenarios | 469-629 | REAL | — | typed stage/code/capability pinning incl. URL-redaction assert (:485) and seq-strict-increase loop (:587-589) |
| autoplay-gate scenario | 633-667 | REAL | S3 | F-001 undefined-sample risk; asserts position freeze + once-per-cycle latch + resume-to-end |
| fetch range scenarios | 671-699 | REAL | — | 206 + content-range window + exact 1024-byte body; blind server 200 + full-body byteLength == config.fixtureBytes |
| audio-context-and-worklet | 701-724 | REAL | — | honest posture: records state, asserts only constructible + worklet parse |
| __lumenWebMatrix export | 738-750 | REAL | — | status summary line; returns result object |

## 5. Test honesty
| Test (scenario) | Would it fail if code were broken? | Reason |
|---|---|---|
| open-play-pause-stop | YES | asserts event-trail transitions, ordering (playing precedes paused), position > 0 — proven live: it FAILS today on the decoder-less wasm build (report :67) |
| play-to-end | YES | position-vs-duration tolerance, busy release, frame geometry w*h*4, end-of-media event — fails today (report :68) |
| open-rtsp/file/http-refused | YES | exact stage/code/capability triple; also no-URL-leak redaction assert |
| event-envelope-invariants | YES | seq strictly increasing across every event; session+gen on pipeline families; requestId echo |
| autoplay-gate | YES (modulo F-001) | position-frozen identity across 200 ms + single gestureRequired latch + resume reaches ended |
| fetch-range-supported/blind | YES | 206 vs 200, content-range prefix, exact byte counts — a misbehaving server fails it |
| idempotent-pause-echo | YES | trail equality before/after echo — any spurious transition fails |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the suite runs 17 scenarios per mode" | matrix.config.json runner.comment | REAL | counted 17 record() calls; report shows 17 rows per mode |
| "success is asserted through EVENTS, never the Accepted echo" | harness.js:11-12 | REAL | :361/:363/:381 wait on stateTrail, only class-A acceptance asserted on echoes |
| "Firefox/WebKit lanes … recorded as BCD-cited expectations … never a fabricated result" | web-matrix.spec.ts:23-25 | REAL | matrix.config.json browsers.firefox evidence "bcd-expectation-only"; report :45 shows chromium playwright-run only |
| "Zero npm dependencies (hand-written…)" | harness.js:26 | REAL | no import statements in file |
## 7. Dead / duplicated / unreachable code
None found. `crossOriginIsolated` at :731 is the standard browser global (not a local), returned in probes — correct usage.
## 8. Dependencies & flags
Depends on apps/web/worker.js + audio-worklet.js glue and the wasm artifact staged by web-matrix-run.mjs; fixture = tests/fixtures/generated/valid/avi_mjpeg_pcm.avi (matrix.config.json server.fixture) — the same parent fixture audited earlier in this assignment (parent of avi trunc4/garbage0). No feature flags.
## 9. Open questions for the next agent
- The 7 failing scenarios are an engine-build posture (wasm without builtin decoders), recorded honestly — but no CI lane FAILS on them (webmatrix is manual). Does any gate consume docs/web/browser-matrix-*.json and go red? UNVERIFIED (web-matrix-run.mjs 821 lines not yet audited; check its report merge/exit-code path).
