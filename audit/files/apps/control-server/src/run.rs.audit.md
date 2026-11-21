# AUDIT: apps/control-server/src/run.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 128 / 6 items (module docs; struct ×1; fn ×1; test helpers ×2; test ×1) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `apps/control-server/src/main.rs:248 serve_loop(&mut server, &mut dispatch, &mut sink, args.sessions)` is the binary's entire serve stage (`lumen-control-server`, a workspace member; compiled in baseline #1, its unittests ran green: `Running unittests src/main.rs (…lumen_control_server…)` nofailfast:1512). Integration coverage drives the same loop with a real client: `e2e_full_session_through_the_app_stack ... ok` (nofailfast:1477-1487, 6/6). Whether packaging lanes ship the binary: not checked here (packaging wave's lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full Read (run.rs + main.rs for the caller); read crates/control-protocol/src/server.rs:425-467 (`serve_one` contract: "Protocol breaches are NOT errors here — they are typed closes recorded in the summary" — the load-bearing fact for this loop's fatal-on-Err design) and :228-238 (`ServerConfig::strict` binds 127.0.0.1:0 — ephemeral, parallel-test safe); read tests/e2e.rs:270-405 (real-client serve_loop coverage incl. outcome + session-end assertions); `rg "serve_loop\|serve_one"` apps + crates → callers; `rg "run::tests"` nofailfast → :1466 green; sed -n verification of both quotes. |

## 1. One-paragraph verdict
A genuinely boring — by its own design goal — 28-line accept/serve loop that is correct as written: the cap is checked before the blocking accept (cap 0 returns without ever touching the socket), every finished session emits exactly one `session-end` record, a `serve_one` error emits a `serve-failed` record and returns verbatim for the caller to map (main.rs maps it to exit 4), and the fatal-on-Err design is sound because the protocol crate guarantees per-client protocol breaches come back as Ok'd typed-close summaries, not errors (server.rs:431-435) — so one hostile client cannot kill the loop. Execution evidence is real on both tiers: the unit test (cap-0 short-circuit) and the e2e lane (a live client through the app stack, cap 1, asserting `sessions==1, capped==true`, the session-end record's `requests_served=3/ops_denied=1/authenticated=true`, and that the log never carries the token). The only defects are documentation-level: `RunOutcome.capped` is constant-`true` in every reachable `Ok` return and its doc comment describes a `false` state that cannot occur, and the lone unit test's name promises "after serving that many sessions" while serving zero.

## 2. What actually works (verified)
- `serve_loop` cap semantics: `max_sessions.is_some_and(|cap| served >= cap)` is checked BEFORE `server.serve_one(dispatch)` blocks on accept — cap 0 returns immediately (proven by the unit test, which binds a real listener and would hang if the check moved after accept). — verified by read + `run::tests::the_cap_stops_the_loop_after_serving_that_many_sessions ... ok` (nofailfast:1466).
- Fatal-on-Err is safe for hostile clients: `ControlServer::serve_one`'s contract (crates/control-protocol/src/server.rs:431-435) — "Protocol breaches are NOT errors here — they are typed closes recorded in the summary"; only accept/deadline-setting Io trouble errors. A garbage-sending client produces an Ok summary (`protocol_errors`, `close` fields) → the loop emits its session record and keeps serving. — verified by read of the callee.
- The happy path with a real client: e2e.rs:281 `serve_loop(&mut server, &mut dispatch, &mut sink, Some(1))` on a spawned thread + a live `TestClient` (hello → welcome → 4 requests → clean close 1000), asserting `outcome.sessions == 1`, `outcome.capped`, exactly one `session-end` record with `requests_served=3`, `ops_denied=1`, `authenticated=true`, and no `token` key in the log — green: `e2e_full_session_through_the_app_stack ... ok` (nofailfast:1483). — test-cited.
- `served.saturating_add(1)` — no overflow panic possible; u32 ceiling (4.29 B sessions) is unreachable in a process lifetime. — verified by read.
- Module-doc claims are accurate: "Single-connection by design" (serve_one's own doc, server.rs:430-431), "`None` = serve until the process is terminated — std-only, no signal handling" (main.rs:264-270 confirms: no signal handling, exit-code mapping only). — verified by read.

## 3. Findings

### F-001 [S4]
Where: apps/control-server/src/run.rs:26-29
Code:
```
    /// Whether the session cap stopped the loop (`false` = the loop
    /// was asked for unbounded service and is only returning because
    /// the caller capped it or the caller's test ended the stream).
    pub capped: bool,
```
What is wrong: `capped` is `true` in every `Ok` return `serve_loop` can produce — the only `Ok` path is the cap branch (lines 48-53, `capped: true`); an unbounded loop (`max_sessions == None`) never returns `Ok` at all (it either serves forever or returns `Err`). The documented `false` state ("asked for unbounded service and is only returning because the caller capped it" — self-contradictory: if the caller capped it, it was not unbounded — "or the caller's test ended the stream" — a closed listener surfaces as `Err(ProtocolError::Io)`, not as `Ok { capped: false }`) is unreachable through this API.
Why it matters: A pub API field that is de-facto constant, with a doc comment rationalizing an impossible state, misleads every consumer (main.rs's `" (cap reached)"` suffix logic reads as if `false` were a live outcome).
What correct looks like: Either drop the field (the Ok-type already implies "stopped at cap") or implement a real graceful-shutdown path that can produce `capped: false`, and fix the doc.
Confidence: HIGH — Verification: read (all return paths enumerated).

### F-002 [S4]
Where: apps/control-server/src/run.rs:108-127
Code:
```
    #[test]
    fn the_cap_stops_the_loop_after_serving_that_many_sessions() {
```
```
        assert_eq!(outcome.sessions, 0);
        assert!(outcome.capped);
        assert!(sink.records.is_empty());
```
What is wrong: The test name promises "after serving that many sessions" but the body serves ZERO sessions — it exercises only the cap-0 short-circuit (as its inline comment honestly admits: "No clients connect: with cap 0 the loop must return immediately"). The `UnitDispatch` (93-106) and `engine_info()` (78-90) helpers are inert — the dispatch trait is never invoked and the engine info never reaches a client. The real multi-session/cap-1/increment/record coverage lives entirely in tests/e2e.rs:281-405.
Why it matters: Name-promises-more-than-body (checklist class); a reader auditing run.rs alone would over-estimate in-file coverage; the mutation "move the cap check after serve_one" is caught here only as a HANG (timeout), not a clean failure, and the "served increments / session record emitted / cap reached after N>0 sessions" mutations are invisible in-file.
What correct looks like: Rename to `cap_zero_returns_without_accepting` and keep it (it is a genuine no-block proof); the serving path stays in e2e (already adequate — arguably move a 2-session variant of it in-file for the increment logic).
Confidence: HIGH — Verification: read + e2e.rs cross-read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-10 | REAL | — | accurate (single-connection, cap semantics, signal-handling deferral — all verified) |
| `RunOutcome` | 22-30 | REAL | S4 | F-001: `capped` constant-true; doc describes impossible state |
| `serve_loop` | 40-67 | REAL | — | cap-before-accept; one record per session; Err → serve-failed record + verbatim return (main.rs:257-260 → exit 4); saturating counter; no panics, no unwraps |
| tests::`engine_info` | 78-90 | REAL | S4 | inert in the cap-0 test (see F-002); live values, protocol literal annotated |
| tests::`UnitDispatch` | 93-106 | REAL | S4 | inert in the cap-0 test (never dispatched); correct shape |
| tests::`the_cap_stops_the_loop_after_serving_that_many_sessions` | 108-127 | REAL | S4 | F-002: honest body, overpromising name; binds a real ephemeral listener (strict → 127.0.0.1:0, parallel-safe) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_cap_stops_the_loop_after_serving_that_many_sessions (run.rs) | PARTLY | cap-check regression (check moved after accept) → hangs → fails by timeout, not assertion; `capped`/`sessions` fields asserted; but zero-session coverage only (F-002) |
| e2e_full_session_through_the_app_stack (tests/e2e.rs — covers this file's happy path) | YES | live client, cap 1: asserts sessions==1, capped, session-end record fields (requests_served=3, ops_denied=1, authenticated=true), no token in log — green (nofailfast:1483) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "accept one connection, run it to completion … log the SessionSummary, repeat" | run.rs:3-5 | REAL | serve_loop 47-66; session_record per summary; e2e asserts the record |
| "a transport-level ProtocolError (accept trouble, listener gone) is returned after a serve-failed record" | run.rs:34-35 | REAL | serve_one contract server.rs:431-435 (protocol breaches = typed closes, not Err); loop's Err arm 59-64 emits then returns |
| "Single-connection by design (the protocol crate's bounded posture)" | run.rs:5-6 | REAL | serve_one serves exactly one connection; host loops (server.rs:430) |
| "`max_sessions` caps the loop … `None` = serve until the process is terminated" | run.rs:6-8 | REAL | loop structure; main.rs runs with args.sessions (None default → unlimited) |
| "no signal handling: SIGTERM/SIGINT end the process and its in-flight session" | run.rs:8-10 | REAL | main.rs:264-270 — plain main, exit-code mapping only |
| `capped` `false` semantics | run.rs:26-28 | FALSE | F-001: state unreachable through serve_loop |

## 7. Dead / duplicated / unreachable code
- `RunOutcome.capped == false` — unreachable value (F-001).
- In-file test helpers `engine_info`/`UnitDispatch` — constructed but never exercised by the cap-0 test (F-002); not dead code repo-wide (same shapes used by e2e.rs's real lane).

## 8. Dependencies & flags
Depends on `control_protocol` (ControlServer/ProtocolError/Dispatch), `crate::logging` (Level/LogRecord/LogSink/session_record) — all used. No feature gates, no unsafe, std-only (per the module's own design statement). The binary reuses `CatalogDispatch` + `JsonStderr` from the crate lib (main.rs:246-247).

## 9. Open questions for the next agent
- control-server lib.rs/config.rs/dispatch.rs owner (W13-A02): the `NotReady`+`Stage::Control` refusal the e2e observes for granted ops (e2e.rs:327-333) is the same code choice shell.rs F-005(b) makes — confirm the refusal-code taxonomy is deliberate across both hosts.
- If a graceful-shutdown path is ever added (making `capped: false` real), main.rs's exit-0 message logic ("cap reached") needs a third branch — noted for whoever touches F-001.
