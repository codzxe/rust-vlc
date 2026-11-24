# CLAIM AUDIT: docs/reviews/w24-a-web-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A04 (recovered by W19-R3) |
| Claims extracted | 13 |
| REAL 4 / TRUE-at-time-since-fixed 6 (fix comments cite this review's finding IDs) / REAL-UNPROVEN 2 (runs/git delta) / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A review whose quality is proven by its aftermath: every P2/P3 finding I re-checked was
subsequently fixed in-tree WITH code comments citing this review's own finding IDs
("W24-R8 P2-2", "P3-2", "W24-R8 P3-4") — the strongest possible corroboration that the
findings were real and the fixes deliberate. The P0 closure holds today (0 merge markers,
both routes present). Its §4 note on `#[must_use]` display artifacts is byte-verified
correct (see below) and matters repo-wide.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "P0 fix verified closed: App.tsx merge-marker count 0 ... /discovery AND /convert preserved" | :31-39 | REAL (still holds) | `grep -cE '^(<<<<<<<|=======|>>>>>>>)' ui/src/App.tsx` -> 0; dynamic imports for both screens at App.tsx:52/:60, route map :227 |
| 2 | "npm run test (vitest) — 718/718 pass, 80 files" | :20 | REAL-UNPROVEN | not re-run (load discipline); baseline vitest 1705 passed on the later tree corroborates growth-consistency |
| 3 | P2-1 "chromium lane is "modes": []" — final evidence not committed | :106-113 | TRUE-at-time-since-fixed | docs/web/browser-matrix-20261110.json now carries 2 chromium modes (isolation-off + isolation-coop-coep) with full evidence: 14 wasmResponses 200s per mode, per-scenario rows incl. HONEST failures (open-play-pause-stop = typed `web.decoders_absent` refusal; play-to-end/seek = pump-loop timeout) |
| 4 | P2-2 "`report.markdownName` is a dead knob" | :114-122 | TRUE-at-time-since-fixed | web-matrix-run.mjs:122 comment "report.markdownName's writer (W24-R8 P2-2): a deterministic renderer over" + :391/:795 write it; docs/web/browser-matrix-20261110.md EXISTS |
| 5 | P3-1 "HARNESS_TIMEOUT_MS = 30000 ... one timeout literal" | :125-129 | TRUE-at-time-since-fixed | rg HARNESS_TIMEOUT_MS in ui/e2e/web/site/harness.js -> 0 hits (knob removed/threaded) |
| 6 | P3-2 "playwright.web.config.ts:44 timeout: 240_000 literal" | :130-133 | TRUE-at-time-since-fixed | playwright.web.config.ts:22-25 "the per-test timeout come straight from the config ... P3-2 moved the runner timeout under the same rule" + config `timeoutMs` field :35 |
| 7 | P3-3 "spawn-failure path skips graceful server shutdown" | :134-139 | TRUE-at-time-since-fixed | web-matrix-run.mjs:655-660 `child.once("error", reject); }).finally(() => {` + "finally closes them on EVERY exit path" |
| 8 | P3-4 "parseRange silently mishandles multi-range requests (bytes=0-1,5-6 served as 0-1)" | :140-145 | TRUE-at-time-since-fixed | web-matrix-run.mjs:485-492 — comma guard returns null (200 full body) with console.warn, comment "the old shape swallowed everything after the first comma via parseInt ... (W24-R8 P3-4)" |
| 9 | P3-5 "Worklog entry not yet committed" | :146-148 | TRUE-at-time-since-fixed | worklog.md carries the wave-24 entries (e.g. :848 "fix verified on LOCAL w24/a 70ed69c/771259c") |
| 10 | "wasm32-safe throttle: ZeroMonotonic ... events.rs:562-590" | :72-80 | REAL | events.rs:557 `throttle: Arc::new(default_throttle_source())`, :567/:575 cfg-split, :579-582 ZeroMonotonic doc + struct — present as described |
| 11 | "a plain grep can display `#ust_use]` artifacts — terminal/ANSI display artifact of #[must_use], not file corruption; octal dump confirms" | :84-87 | REAL (byte-verified, IMPORTANT) | od -c of events.rs:172 -> `#   [   m   u   s   t   _   u   s   e   ]`; python byte scan: 34 `ust_use` sites, ALL clean `#[must_use]`, ZERO `#ust_use]` — yet rg displays them as `#ust_use]`. Cross-agent implication: W19-A09's log claims "259 files/1044 sites of #ust_use] — tree cannot compile" — that is a display artifact, contradicted by byte-level evidence and by the clean-build baseline |
| 12 | "Firefox/WebKit/Safari/Safari-iOS lanes committed as evidence: not-runnable-in-sandbox + BCD-cited expectations" | :61-63 | REAL | browser-matrix-20261110.json lanes: exactly `evidence: "not-runnable-in-sandbox"` + BCD summary + nextStep per lane |
| 13 | "Scope discipline ... delta touches 15 files, all inside lane A's ownership" | :89-95 | UNVERIFIED | git history not available in sandbox; no contradictory evidence in-tree |

## 3. Contradictions
None. (One repo-wide correction handed forward: the `#ust_use]` display artifact vs
W19-A09's corruption claim — see claim 11; the file bytes are clean.)

## 4. Fabrication / inflation indicators
None. The review's own limits are stated (":150-157 chromium executable absent, could
NOT re-run the browser matrix end-to-end") and its P2/P3s were all confirmed real by the
fix waves that cite them.

## 5. Reachability
N/A (review doc). The audited artifacts (browser-matrix harness, events.rs wasm path) are
real shipped/CI paths.
Commands: grep -c markers, rg on App.tsx/routes, python json lane dump, rg
HARNESS_TIMEOUT_MS / markdownName / timeoutMs / finally / parseRange, od -c +
python byte scan of events.rs, worklog greps.
