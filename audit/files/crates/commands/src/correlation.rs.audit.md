# AUDIT: crates/commands/src/correlation.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2025-06-27 |
| Lines / functions | 407 / 12 fns + 11 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (desktop: apps/desktop/src/host.rs:141-153 `DesktopHost{ids,staleness,log}` -> correlation::{RequestIdAllocator,StalenessFilter,DeliveryLog}; also apps/cli/src/host.rs:167-240, apps/web/src/host.rs:88,795-843) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full Read of file (407 lines); `rg` for type users repo-wide (exit 0); `sed -n` §4.1-§4.3 of docs/05-command-layer.md; python3 byte-scan of `answered` occurrences; baseline citation |

## 1. One-paragraph verdict
Real, small, and mostly honest: the request-id allocator, the §4.2 staleness filter (rule C6) and the §4.3 delivery validator are genuinely implemented, wired into all three shipped hosts (desktop/CLI/web), and covered by 11 passing tests that would fail under the mutations they describe (stale-drop, watermark-raise, seq monotonicity, echo validity). The C6 rule matches doc 05 §4.2/§4.1 exactly (strictly-lower generation dropped, equal kept, per-session watermarks, sessionless/genless events never dropped). The one real defect is a lie in `DeliveryLog`'s doc comment: it claims to check three invariants but implements only two — the `answered` set is populated and then never read, because the claimed origin/causality check does not exist anywhere in `observe_event`.

## 2. What actually works (verified)
- `rg` -> apps/desktop/src/host.rs:141-153 (DesktopHost fields `ids: RequestIdAllocator, staleness: StalenessFilter, log: DeliveryLog`), :161 `note_allocated`, :171 `staleness.observe_response`, :192 `staleness.filter_event`, :199 `log.observe_event` — the module is live in the shipped desktop host, not just tests.
- C6 semantics: read of filter_event (75-86) — `generation < watermark` dropped, equal/newer kept and raised; matches doc 05 §4.2 (L433-438) and C6 (L63).
- `accepted_generation()` (envelope.rs:84-89) returns `Some` only for `Accepted`, so `observe_response`'s `unwrap_or(response.generation)` correctly uses the envelope gen for `Completed`/`Err` — sound.
- Tests executed: audit/runs/baseline/cargo-test-workspace.txt:1472-1482 — 11/11 `correlation::tests` `ok`.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/correlation.rs:104-112, 116, 145, 151-164
Code:
```
/// Invariants checked:
/// 1. `seq` is strictly increasing across the observed events.
/// 2. A response never arrives for a request id the host has not
///    allocated (echo validity).
/// 3. An event carrying `origin` (a request id) is causally consistent:
///    it is only invalid if the SAME origin was never observed as a
///    response — the full causal-order proof (response positioned
```
What is wrong: Invariant 3 is not checked. `observe_event` (151-164) only validates `seq` monotonicity; it never looks at `event.origin`. The `answered` set (L116), populated at L145 (`self.answered.insert(response.request_id);`), is never read anywhere — python byte-scan of the file finds exactly two occurrences of `answered` (declaration + insert). The dead set is the smoking gun for a check that was specified, half-written (the write side), and never finished. Why it matters: this struct is documented as "the conformance harness every host binding feeds"; the desktop host surfaces `violations()` as §4.3 evidence, and a real causality violation (event citing an origin that was never answered) sails through silently. What correct looks like: in `observe_event`, if `event.origin` is `Some(id)` and `!self.answered.contains(&id)` (and the engine has no reason to emit pre-response origins), record a violation — or delete the claim and the field. Confidence: HIGH — Verification: read + python byte-scan + rg (no external reader of `answered`).

F-002 [S4]
Where: crates/commands/src/correlation.rs:23-26
Code:
```
    pub fn allocate(&mut self) -> RequestId {
        self.next += 1;
        RequestId(self.next)
    }
```
What is wrong: `u64` overflow behavior: debug builds panic at 2^64 allocations; release builds wrap and silently reuse ids, violating the documented "reuse forbidden" (L21-22) and doc 05 §4.1. Practically unreachable (2^64 allocations per connection), so hygiene-level, but the contract wording is absolute and the code cannot uphold it in release mode. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/commands/src/correlation.rs:44, 115-116
Code:
```
pub struct StalenessFilter {
    watermarks: BTreeMap<SessionId, Generation>,
}
```
What is wrong: Unbounded growth: `watermarks` is never pruned (no `session.close` hook; `raise` only inserts/raises), and `DeliveryLog.allocated`/`answered` grow by one entry per request for the process lifetime. Only `violations` is documented as "kept small" (L118-120). For a long-lived desktop host cycling many sessions this is a slow leak of small entries — hygiene, not a crash. What correct looks like: drop the session's entry when the host observes session teardown. Confidence: HIGH — Verification: read (no removal path exists in file).

F-004 [S4]
Where: crates/commands/src/correlation.rs:59-70
Code:
```
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) {
        let Some(session) = response.session else {
            return;
        };
```
What is wrong: The filter keys exclusively off the ENVELOPE's `session`; the `Accepted` outcome's own `session: Option<SessionId>` (envelope.rs:69-70) is never consulted. If an engine/host ever delivers an `Accepted` whose envelope session is `None` but whose outcome session is `Some`, the watermark is not raised and subsequent current events would be wrongly kept/dropped relative to that session. Whether such a response can occur depends on engine-side construction I did not audit — UNVERIFIED reachability. Confidence: MEDIUM — Verification: read; reachability of the None/Some combination UNVERIFIED (engine crate outside this assignment).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RequestIdAllocator::new | 17-19 | REAL | — | next=0, ids from 1 |
| RequestIdAllocator::allocate | 23-26 | REAL | S4 | wraparound reuse in release (F-002) |
| RequestIdAllocator::last | 29-31 | REAL | — | correct None-before-first |
| StalenessFilter::new | 49-53 | REAL | — | |
| StalenessFilter::observe_response | 59-70 | REAL | S4 | envelope-session only (F-004); Accepted-gen precedence correct |
| StalenessFilter::filter_event | 75-86 | REAL | — | exact C6: < watermark dropped, == kept, > raises |
| StalenessFilter::watermark | 89-91 | REAL | — | |
| StalenessFilter::raise | 93-98 | REAL | — | monotonic raise only |
| DeliveryLog::new | 125-127 | REAL | — | |
| DeliveryLog::note_allocated | 131-133 | REAL | — | |
| DeliveryLog::observe_response | 137-147 | REAL | S3 | populates dead `answered` set (F-001) |
| DeliveryLog::observe_event | 151-164 | REAL | S3 | seq check only; no origin check (F-001) |
| DeliveryLog::violations | 167-169 | REAL | — | |
| tests (11) | 172-407 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| request_ids_are_monotonic_and_never_reused (225) | YES | asserts concrete 1..=5 sequence + set-unique |
| accepted_responses_raise_the_generation_watermark (249) | YES | 3->4 raise, stale 2 does not lower |
| error_responses_track_the_envelope_gen (262) | YES | Err response uses envelope gen |
| stale_pipeline_events_are_dropped_current_ones_kept (273) | YES | the core C6 matrix incl. post-raise drop |
| sessionless_and_genless_events_are_never_dropped (294) | YES | both None cases kept |
| watermarks_are_per_session (316) | YES | session 9 gen-3 current while stale for session 7 |
| response_without_session_does_not_touch_watermarks (328) | YES | |
| seq_must_be_strictly_increasing (346) | YES | repeat + going-back both rejected |
| responses_must_echo_allocated_requests (357) | YES | unallocated 99 rejected |
| the_full_host_side_pipeline_over_a_command_storm (369) | YES | end-to-end: ghost event of superseded gen dropped, no violations |
| request_envelope_carries_the_allocated_id (398) | YES | trivial but real |
Notably ABSENT: any test of the claimed invariant 3 (origin causality) — consistent with it not being implemented (F-001).
All 11 green in baseline (cargo-test-workspace.txt:1472-1482).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Allocates RequestIds … monotonic from 1, reuse forbidden" | correlation.rs:7-9, doc 05 §4.1 | REAL (with S4 caveat) | allocator code + test 225; release-wrap caveat F-002 |
| "drops any pipeline-scoped event whose generation is lower than that watermark" | correlation.rs:39-41 | REAL | filter_event 75-86 + test 273 |
| "Invariants checked: 1…2…3 (origin causality)" | correlation.rs:104-112 | FALSE (item 3) | observe_event 151-164 has no origin check; `answered` never read |
| "the conformance harness every host binding feeds" | correlation.rs:101-102 | SHALLOW | only desktop host wires DeliveryLog (apps/desktop/src/host.rs:143); CLI and web hosts use allocator+filter only (verified by rg) |

## 7. Dead / duplicated / unreachable code
- `DeliveryLog.answered` (L116): written (L145), never read — dead field (F-001).
- No other dead code; no duplication.

## 8. Dependencies & flags
std-only (`std::collections::BTreeMap/BTreeSet`); depends on `crate::envelope` types. No feature gates, no `#[allow]`, no unsafe. (Note: `#[must_use]`-style attributes exist in this crate's hosts but none in this file.)

## 9. Open questions for the next agent
- Does any engine path emit `CommandResponse { session: None, .. }` with an `Accepted { session: Some(..) }` outcome? If yes, F-004 becomes a correctness bug (watermark not raised). Audit the engine's response construction.
- Phase-2 conformance suite (doc 05 §4.3 terminal invariants) — where is it? `crates/control-protocol/tests/conformance.rs` exists; if it is meant to consume `DeliveryLog` for origin checks, F-001 escalates to S2 for that consumer.
