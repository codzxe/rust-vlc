# AUDIT: crates/streaming/tests/netfault/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 239 / 9 items (Fault enum, FaultScript×5, FaultProxy×2, HttpFetch::fetch) — no in-file tests (tested by sibling e2e binaries) |
| File verdict | REAL (deterministic test-support; mechanism honestly disclosed in code) with one misleading doc column |
| Reachable from shipped entry point? | NO by construction — lives under tests/ (test-support only; claim at L38-39 verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg over the two consumer binaries; baseline citation |

## 1. One-paragraph verdict
FaultProxy is an HttpFetch decorator that scripts per-fetch faults — but it does NOT fault the wire: after the inner transport (MockClient over a real socket) SUCCEEDS, the proxy discards the healthy response and synthesizes the typed error (BodyTruncated / Io{ConnectionReset} / ReadTimeout) or paces a virtual delay. This is disclosed twice (module intro "a deterministic fault-injection layer over the fetch port", and L202-205 "no socket-timing races to reconstruct what 'half a body' means"), and it is well-matched to the retry design it tests: net_interrupted.rs's `mid_segment_drop_resumes_byte_exact` proves the wrapper re-issues the IDENTICAL ranged request and gets a byte-exact answer — since retries never consume partial bytes, the proxy's discard-the-body semantics cannot mask a resume bug in the current design. The counters (fetched/fired), offset-guards (`len > at` — a fault scripted past the body never fires), poison-safe locks, and the finite 64-entry `forever` are all correct and asserted by the 7 netfault_proxy_e2e tests. The only defect is the "Wire shape" table column, which describes the real-world condition each fault STANDS FOR ("a TCP RST mid-body") — no RST ever occurs on any wire through this proxy; real socket-level truncation exists only via MockConfig.truncate_body_at in mockhttp.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt: tests/netfault_proxy_e2e.rs → `delay_paced_by_injectable_clock ... ok`, `drop_after_offset_fires_exactly_once ... ok`, `offset_past_body_end_never_fires ... ok`, `fault_script_sequence_fires_in_order_then_heals ... ok`, `reset_after_offset_fires_exactly_once ... ok`, `truncate_at_offset_fires_exactly_once ... ok`, `persistent_fault_script_never_heals_within_its_budget ... ok` → 7 passed (0.15s). Consumer net_interrupted.rs: 3 passed incl. `mid_segment_drop_resumes_byte_exact` (read L75-135: fired==1, fetched==2, byte-exact resume, backoff ceiling asserted on the virtual clock).

## 3. Findings
F-001 [S4]
Where: crates/streaming/tests/netfault/mod.rs:11-16
Code:
```
//! | [`Fault`]             | Wire shape                                        | Typed result                                              |
//! | --------------------- | ------------------------------------------------- | --------------------------------------------------------- |
//! | [`Fault::TruncateAt`] | `Content-Length` advertised, then a clean close   | [`NetworkError::BodyTruncated`] `{expected, received}`     |
//! | [`Fault::ResetAfter`] | a TCP RST mid-body                                | [`NetworkError::Io`] (`ErrorKind::ConnectionReset`)        |
```
What is wrong: the "Wire shape" column describes network conditions that never occur through this proxy — the inner fetch completes successfully over a real socket, then the healthy response is discarded and the typed error is synthesized in-process (L201-237). Why it matters: a reader (or a doc citing this suite as "real network fault injection over sockets") would believe TCP RSTs and mid-body closes are exercised; they are not — the only real-socket truncation in the repo is MockConfig.truncate_body_at (mockhttp) plus its client-side mapping tests. The mechanism IS honestly disclosed at L4 and L202-205, so this is a misleading summary table, not a hidden fake. Confidence: HIGH — Verification: read both the proxy code and both consumer binaries.
F-002 [S4]
Where: crates/streaming/tests/netfault/mod.rs:120-125
Code:
```
    pub fn forever(fault: Fault) -> FaultScript {
        // 64 entries: more than any retry budget in this crate's
        // policies (max_retries default 4), still finite so a buggy
        // caller cannot spin forever on an exhausted script.
        FaultScript::new(vec![fault; 64])
    }
```
What is wrong: "forever" is 64 — a caller that retries more than 64 times silently observes a healed network; the e2e guard (`persistent_fault_script_never_heals_within_its_budget`) only covers budgets ≤64. Deliberate and documented; recorded so no one mistakes it for a true infinite outage. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Fault (enum, 4 variants) | 50-76 | REAL | — | positional offsets, Delay clock-paced |
| FaultScript::new / once / forever | 96-125 | REAL | S4 | forever=64 (F-002) |
| FaultScript::take | 127-139 | REAL | — | advances fetched even when script exhausted; consumes the fault slot even if the inner fetch later fails (deterministic; noted) |
| FaultScript::record_fired / fetched / fired | 141-165 | REAL | — | poison-safe; offset-past-end faults correctly not counted |
| FaultProxy::new | 181-195 | REAL | — | inner + script + injectable clock |
| FaultProxy::fetch (HttpFetch impl) | 198-239 | REAL | S4 context | post-success error synthesis (F-001); guards `len > at`; Delay fires unconditionally (non-positional); inner errors pass through |

## 5. Test honesty
No tests in this file (shared mod for two e2e binaries — the correct shape for test support). Coverage lives in netfault_proxy_e2e.rs (7 tests, all would fail if take/guards/counters broke — exact fired/fetched counts asserted) and net_interrupted.rs (3 scenario tests with byte-exact resume + backoff-ceiling assertions on FakeClock).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A deterministic fault-injection layer over the fetch port" | netfault/mod.rs:4 | REAL | L198-238 decorates HttpFetch |
| "applies a scripted per-fetch fault, at byte offsets" | netfault/mod.rs:8-9 | REAL | `if len > at` guards; `offset_past_body_end_never_fires` test |
| "Wire shape" table (RST mid-body, clean close, black hole) | netfault/mod.rs:11-16 | MISLEADING (emulated, not caused) | F-001 |
| "Delay goes through the injectable clock, not a real sleep" | netfault/mod.rs:27-32 | REAL | L210 `self.clock.sleep_ms(ms)`; `delay_paced_by_injectable_clock` test |
| "Test-support only — nothing here is compiled into production paths" | netfault/mod.rs:38-39 | REAL | file lives at tests/netfault/mod.rs, included only by test binaries |

## 7. Dead / duplicated / unreachable code
None. Style deliberately mirrors mockhttp's ScriptedRoute/ScriptedStatus (shared-state handle + counters) — consistent, minor duplication of the pattern by design.

## 8. Dependencies & flags
streaming (clock/fetch/error), std only. No features, no unsafe, no unwrap/expect. The clock seam (WallClock/FakeClock) is the production crate's own injectable interface.

## 9. Open questions for the next agent
- If reconnect.rs ever implements PARTIAL resume (resume-from-`received`), FaultProxy's discard-the-body semantics will silently stop being equivalent to a real truncation — the e2e suite would need a real-socket truncation lane (mockhttp truncate_body_at) at that point. Flag for whoever audits reconnect.rs.
- Does any doc/matrix row describe net_interrupted as testing "real network failures over sockets"? If yes, that row inherits F-001's caveat.
