# AUDIT: ui/e2e/web/matrix.config.json
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 72 lines / 8 config keys (schema, server, modes, browsers, pump, waits, runner, playToEnd, serving, report) |
| File verdict | REAL (config, actively read + validated) — the lane it configures is gate-unwired |
| Reachable from shipped entry point? | As config: YES — read by ui/playwright.web.config.ts:40 (runner/launch knobs, validated :46) and ui/scripts/web-matrix-run.mjs:41 (port/paths/modes, fail-fast validation :86-94) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg consumers (both verified), ls docs/web/, rg web-matrix over workflows + local gates (zero) |

## 1. One-paragraph verdict
A genuinely config-driven harness definition — the claim in its own comment ("the run script,
the Playwright config and the page harness read this file, never a literal in code") checks
out at both consumer sites, each with fail-fast validation of the required keys. The knobs are
coherent (server 127.0.0.1:5187, wasm artifact + fallback, both isolation modes, pump budgets,
24-page 20 ms play-to-end tolerance), and the honesty markers are real: firefox/webkit are
"bcd-expectation-only" (no fake run claims), chromium evidence is "playwright-run" with an env
override documented for browser-download-forbidden sandboxes. The two defects: nothing in any
committed gate runs this lane (local gates and workflows have zero web-matrix references), and
the report output name is a date-frozen literal.

## 2. What actually works (verified)
— verified by rg + read: playwright.web.config.ts:40 `readFileSync(path.join(here, "e2e",
"web", "matrix.config.json"), "utf8")` with integer validation (:46); web-matrix-run.mjs:41
loads it and fails fast on missing keys (:86-94). docs/web/browser-matrix-20261110.{md,json}
exist on disk as the recorded evidence.

## 3. Findings
F-001 [S3]
Where: whole file + absence of wiring
Evidence: rg "web-matrix|web_matrix" over .github/workflows/* + scripts/full_gate.sh +
scripts/rc-smoke.sh → zero matches. The lane's only committed evidence is the dated local
report docs/web/browser-matrix-20261110.md (report.outputDir/markdownName, :68-70).
What is wrong: the config is real but nothing automated ever runs the lane it configures —
browser coverage claims rest on a 14-day-old local capture (2026-11-10 vs audit date
2026-11-24). Why it matters: same pattern as the visual matrix pre-CI pins: knobs green,
execution unwitnessed. What correct looks like: a workflow job (even nightly) running
`node scripts/web-matrix-run.mjs`, or an explicit README statement that the lane is manual.
Confidence: HIGH — Verification: read + rg (gates/workflows).
F-002 [S4]
Where: :69-70 `"markdownName": "browser-matrix-20261110.md"`
What is wrong: the output filename is a frozen date literal — any rerun after 2026-11-10
either overwrites the old-named file (name lies about its date) or requires a manual config
edit. Confidence: HIGH — Verification: read.

## 4. Per-function table (config keys)
| Key | Verdict | Notes |
|---|---|---|
| server (host/port/artifacts/fixture paths) | REAL | consumed by web-matrix-run.mjs (validated) |
| modes (isolation off/coop-coep) | REAL | both exercised per report |
| browsers.chromium | REAL | playwright-run evidence + env override documented |
| browsers.firefox/webkit | honest non-claims | "bcd-expectation-only" |
| pump (ticks/slices/timeout) | REAL | in-page harness budgets |
| waits.timeoutMs | REAL | threaded to page harness per comment |
| runner.timeoutMs | REAL | read by playwright.web.config.ts:40+ |
| playToEnd.positionToleranceUs | REAL | 20 ms tolerance |
| serving.coopCoepDefault | REAL | records shipped posture; does not alter harness serving (documented) |
| report | REAL | S4 date-frozen name |

## 5. Test honesty
N/A (config). Consumer validation IS the honesty mechanism — missing keys fail loudly at both
consumers (:46, :86-94).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a literal in code" | :3 comment | REAL | both consumers readFileSync the config; knob-to-consumer verified for runner + server + modes |
| "the report records the version + user agent that actually ran" | :38 | REAL-UNPROVEN | report JSON exists; not re-parsed here |

## 7. Dead / duplicated / unreachable code
None: every key has a consumer or is an explicitly-documented posture record (serving).

## 8. Dependencies & flags
No deps — JSON + JSON.parse at consumers. Note wasmArtifact path implies the wasm build
(rc-smoke baseline confirms wasm build passes).

## 9. Open questions for the next agent
Is docs/web/browser-matrix-20261110.md's chromium cell re-capturable in this sandbox
(agent-browser Chrome for Testing per :38)? If not, mark the whole lane's current evidence
set UNVERIFIED-live, exactly like the 5th visual re-pin.
