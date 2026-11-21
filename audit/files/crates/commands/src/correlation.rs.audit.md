# AUDIT: crates/commands/src/correlation.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-21 |
| Lines / functions | 407 / 12 + 11 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/Cargo.toml:52 `commands` dep → apps/cli/src/host.rs:167-168 `allocator: commands::RequestIdAllocator` / `staleness: commands::StalenessFilter`; :849/:858/:1093 `allocator.allocate()`; :676 `staleness.filter_event(&wire)` on every wire event. Web (apps/web/src/host.rs:843/:1325) and desktop (apps/desktop/src/host.rs:141-153, all three types) wire it too. All 11 unit tests executed in baseline run #3 (test-workspace.log.txt:1339-1349); the CLI pump path (incl. `filter_event`) ran in the baseline MP4/AVI open-session smokes.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "StalenessFilter\|RequestIdAllocator\|DeliveryLog"` across apps/crates/tools (all callers); `rg -n "observe_response"` across apps (exactly one production caller: apps/desktop/src/host.rs:171); `rg -n "staleness"` apps/cli/src/host.rs, apps/web/src/host.rs (init + filter_event only, no observe_response); `rg -n "is_stale_at_drain" crates/playback-engine/src/session.rs` (engine-side mitigation, :748); `rg -n "fn is_pipeline_scoped" -A 8 crates/commands/src/envelope.rs` (scoped definition matches); `sed -n` verification of every quote; baseline test log lines 1339-1349 |

## 1. One-paragraph verdict
The correlation core is real, small, and correct: monotonic request ids from 1, a per-session generation watermark that only rises, a staleness rule that drops strictly-older pipeline-scoped events, and a seq-monotonicity/echo-validity validator — all proven by 11 executed baseline tests including the card 1.D3 end-to-end "command storm" scenario. The problems are wiring and overclaiming, not logic: `StalenessFilter::observe_response` has exactly ONE production caller (desktop host) — the CLI and web hosts run the event half of the §4.2 rule without ever feeding responses in, surviving only because the engine separately suppresses stale events at drain; and `DeliveryLog` documents three invariants but implements two — the third (origin causality) has no code, its supporting `answered` field is written and never read, and the claim that "every host binding feeds" the harness is false (only desktop does). The allocator logic itself has no defects beyond a theoretical u64 wrap.

## 2. What actually works (verified)
— Watermark semantics correct by trace: `raise` (93-98) only increases; stale `Accepted` responses cannot lower it (test `accepted_responses_raise_the_generation_watermark`, baseline log 1339); `filter_event` (75-86) drops exactly `generation < watermark`, keeps `==` and `>`, and never drops session-less/generation-less events — matches `Event::is_pipeline_scoped` (envelope.rs:139-141: both fields Some).
— Card 1.D3 AC exercised end-to-end: `the_full_host_side_pipeline_over_a_command_storm` (369-395) does seek A @gen 3 → seek B @gen 4 → B's event kept → A's ghost dropped — executed ok (baseline log 1348).
— RequestIdAllocator: first id 1, strictly monotonic (test, log 1342); `Default` == `new()` (both `next: 0`), so the derive cannot desync.
— Seq monotonicity validator: `observe_event` (151-164) flags `seq <= last` (repeats AND regressions) — test, log 1345.
— Echo validity: `observe_response` (137-147) flags responses for unallocated ids — test, log 1344.

## 3. Findings
### F-001 [S3]
Where: crates/commands/src/correlation.rs:59-70 (the unwired half; callers: apps/cli/src/host.rs:240+676, apps/web/src/host.rs:843+1325, apps/desktop/src/host.rs:171)
Code:
```
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) {
        let Some(session) = response.session else {
            return;
        };
        let watermark = response
            .result
            .as_ref()
            .ok()
            .and_then(|outcome| outcome.accepted_generation())
            .unwrap_or(response.generation);
        self.raise(session, watermark);
    }
```
What is wrong: This method — the "via `Accepted` responses" half of doc 05 §4.2's host staleness rule — has exactly ONE production caller in the entire repo: `apps/desktop/src/host.rs:171`. The CLI host (apps/cli/src/host.rs:240 constructs the filter, :676 calls only `filter_event`) and the web host (apps/web/src/host.rs:843, :1325) never feed responses in, so their watermarks rise only from events. Both hosts' own headers claim the full filter: CLI host.rs:14-16 "events through the wire conversion and the staleness filter"; web host.rs:8-10 "converts engine events to the wire stream through the staleness filter".
Why it matters: In CLI/web, a ghost event from a superseded generation that arrives BEFORE any current-generation event passes `filter_event` (watermark still old) and is delivered as fresh. No user-visible leak is demonstrable today only because the engine independently suppresses stale pipeline-scoped events at drain (playback-engine/src/session.rs:748 `if event.is_stale_at_drain(current)` — "the engine-side staleness rule"). The host filter is the second, independent half of a documented two-layer defense; two of three hosts deploy half of it. If the engine-side rule ever regresses, nothing in CLI/web notices. Doc 05 §4.2 explicitly makes BOTH observation sources part of the host rule ("via `Accepted` responses or events").
What correct looks like: CLI/web hosts call `staleness.observe_response(...)` wherever they convert receipts/responses (the CLI's `pending_receipts` path), mirroring apps/desktop/src/host.rs:167-186.
Confidence: HIGH — Verification: read + rg over all apps/crates/tools (caller lists pasted in log).

### F-002 [S3]
Where: crates/commands/src/correlation.rs:101-121 (claim), 116+145 (dead field), and the absent invariant-3 code
Code:
```
/// Validates the §4.3 ordering invariants on an observed delivery
/// stream — the conformance harness every host binding feeds.
///
/// Invariants checked:
/// 1. `seq` is strictly increasing across the observed events.
/// 2. A response never arrives for a request id the host has not
///    allocated (echo validity).
/// 3. An event carrying `origin` (a request id) is causally consistent:
///    it is only invalid if the SAME origin was never observed as a
///    response — the full causal-order proof (response positioned
```
(and line 116 `    answered: std::collections::BTreeSet<RequestId>,` / line 145 `        self.answered.insert(response.request_id);`)
What is wrong: Three defects in one struct. (a) Invariant 3 is documented but NOT IMPLEMENTED: no method of `DeliveryLog` ever reads `event.origin` — `observe_event` (151-164) checks only `seq`; there is no code path that could record an origin-causality violation. (b) The `answered` set is written at 145 and read by NOTHING (no getter, no assertion in any test) — the dead vestige of the missing invariant-3 check. (c) The doc claim "the conformance harness every host binding feeds" is false: only apps/desktop/src/host.rs:143/:180/:199 feeds a `DeliveryLog`; the CLI and web hosts have zero references to it.
Why it matters: The type overstates the guarantees it delivers: a reader (or the Phase-2 conformance suite the comment defers to) believes origin causality is host-checked and that all bindings are under the harness. Duplicate responses for the same request id — the thing `answered` exists to detect — also pass unflagged.
What correct looks like: Either implement invariant 3 (`observe_event`: if `event.origin` is `Some(id)` and `!answered.contains(id)` (and allocation observed) → violation) and flag duplicate responses (`if !answered.insert(id) { violation }`), or delete invariant 3 from the doc and delete `answered`; and fix "every host binding feeds" to "the desktop host feeds" until CLI/web wire it.
Confidence: HIGH — Verification: read (whole struct + all methods + all 11 tests; no origin read, no answered read).

### F-003 [S4]
Where: crates/commands/src/correlation.rs:42-45, 93-98
Code:
```
pub struct StalenessFilter {
    watermarks: BTreeMap<SessionId, Generation>,
}
```
What is wrong: Entries are only ever inserted/raised (93-98); there is no eviction API and no caller removes closed sessions. A long-lived host (desktop app running for weeks; `max_open_sessions = 4` concurrent but sessions open/close over time) accumulates one map entry per SessionId ever observed.
Why it matters: Unbounded growth by design in a host-lifetime structure — tiny per entry (~24 B), so this is hygiene, not a leak that bites; but nothing in the type or docs acknowledges it.
What correct looks like: A `forget(session)` called on session-close (the absorbed `SessionClosed` event is the natural hook), or a doc note that the map is bounded by host-lifetime session count and considered acceptable.
Confidence: HIGH — Verification: read (no removal path anywhere in crate; rg for `watermarks.remove` → none).

### F-004 [S4]
Where: crates/commands/src/correlation.rs:23-26
Code:
```
    pub fn allocate(&mut self) -> RequestId {
        self.next += 1;
        RequestId(self.next)
    }
```
What is wrong: `self.next += 1` overflows at `u64::MAX`: debug builds panic, release builds wrap to `RequestId(0)` and then reuse `RequestId(1)` — silently violating the "never reused" contract the type documents (line 21-22).
Why it matters: Practically unreachable (2^64 allocations ≈ 584 years at 1 G/s), so S4 — but the sibling counter type in media-types treats exhaustion as a typed error (`ErrorContext::new("generation.exhausted")`, ids.rs:473; `LimitKind::SessionGenerations`), so the asymmetry is a design inconsistency, not a live bug.
What correct looks like: `checked_add` → typed error (or the documented rationale that u64 makes exhaustion unreachable, mirroring the SessionGenerations treatment).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RequestIdAllocator` struct | 10-13 | REAL | — | `Default` == `new()` (both start 0). |
| `RequestIdAllocator::new` | 17-19 | REAL | — | next=0, first allocate → 1, matches doc "monotonic from 1". |
| `RequestIdAllocator::allocate` | 23-26 | REAL | S4 | Monotonic pre-increment; u64 wrap at 2^64 (F-004); no panic on valid input short of overflow. |
| `RequestIdAllocator::last` | 29-31 | REAL | — | `None` before first — correct guard `next > 0`. |
| `StalenessFilter` struct | 42-45 | REAL | S4 | No eviction (F-003). |
| `StalenessFilter::new` | 49-53 | REAL | — | == Default. |
| `StalenessFilter::observe_response` | 59-70 | REAL | S3 | Logic correct (Accepted gen preferred, envelope gen fallback, Err responses still raise to current context); ONE production caller — the wiring gap is F-001. Edge: envelope `session: None` + `Accepted { session: Some }` returns early and never raises — no production minter of that shape found (rg over catalog/control-server constructors), noted UNVERIFIED for the dispatch auditor. |
| `StalenessFilter::filter_event` | 75-86 | REAL | — | Drops exactly `generation < watermark`; inline scoped check mirrors `Event::is_pipeline_scoped` (envelope.rs:139-141) — verified identical semantics; executed in baseline CLI smokes. |
| `StalenessFilter::watermark` | 89-91 | REAL | — | Read-only accessor; used by desktop host's bump detection. |
| `StalenessFilter::raise` (private) | 93-98 | REAL | — | Monotonic max-update; cannot lower. |
| `DeliveryLog` struct | 113-121 | REAL | S3 | `answered` dead (F-002). |
| `DeliveryLog::new` | 125-127 | REAL | — | == Default. |
| `DeliveryLog::note_allocated` | 131-133 | REAL | — | Set insert. |
| `DeliveryLog::observe_response` | 137-147 | REAL | S3 | Echo validity implemented; duplicate responses unflagged; writes dead `answered` (F-002). |
| `DeliveryLog::observe_event` | 151-164 | REAL | S3 | Seq monotonicity implemented; invariant 3 (origin) absent (F-002). |
| `DeliveryLog::violations` | 167-169 | REAL | — | Slice accessor; desktop host surfaces it (host.rs:206). |

## 5. Test honesty (all 11 executed in baseline run #3, log lines 1339-1349)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `request_ids_are_monotonic_and_never_reused` (225) | YES | Exact id sequence + distinctness. |
| `accepted_responses_raise_the_generation_watermark` (249) | YES | Covers raise, re-raise, and the stale-Accepted-must-not-lower case. |
| `error_responses_track_the_envelope_gen` (262) | YES | Err → envelope generation fallback. |
| `stale_pipeline_events_are_dropped_current_ones_kept` (273) | YES | The core C6 rule incl. watermark raise via kept events and post-raise drop. |
| `sessionless_and_genless_events_are_never_dropped` (294) | YES | Both pass-through shapes. |
| `watermarks_are_per_session` (316) | YES | Cross-session isolation incl. same generation different staleness. |
| `response_without_session_does_not_touch_watermarks` (328) | YES | Early-return path. |
| `seq_must_be_strictly_increasing` (346) | YES | Repeat and regression both flagged; violation text asserted. |
| `responses_must_echo_allocated_requests` (357) | YES | Unallocated id flagged with message. |
| `the_full_host_side_pipeline_over_a_command_storm` (369) | YES | The card AC end-to-end: allocation, two bumps, ghost drop, interleaved response, order kept. |
| `request_envelope_carries_the_allocated_id` (398) | NO (tautology) | Asserts that a struct field it just set equals what it set (`req.request_id == RequestId(1)`); breaks only if the compiler breaks. Tests nothing about correlation. |
| NOT TESTED: invariant 3 (origin causality), duplicate-response flagging | — | Consistent with the code being absent (F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`request_id` — u64, assigned by the sender, starting at 1, strictly monotonic … reuse is forbidden" | docs/05-command-layer.md:425-427 §4.1 | REAL | RequestIdAllocator (17-31) + test (baseline 1342); all three hosts allocate via it. |
| "Staleness rule (hosts): ignore any pipeline-scoped event whose gen is lower than the highest gen observed for that session (via Accepted responses or events)" | docs/05-command-layer.md:436 §4.2 | REAL in type / SHALLOW in wiring | Type implements both observation sources (59-70, 75-86), tests executed; but CLI/web hosts observe via events only — F-001. |
| "The typed clients implement this once" | docs/05-command-layer.md:437 §4.2 | PARTIAL | Rust side: commands::StalenessFilter, yes. The TS "typed client" of §5.1 (`ui/packages/client`) does not exist in the tree (ui/ has no packages/ dir) — flagged to the doc-05/UI auditor, out of this file's scope. |
| "A host observes a strictly increasing seq stream (per connection)" | docs/05-command-layer.md:445 §4.3.1 | REAL | DeliveryLog::observe_event implements the check; test executed (baseline 1345). |
| "engine emits `Accepted` for request R before any event caused by R … delivery preserves that causal order" | docs/05-command-layer.md:446 §4.3.2 | REAL-UNPROVEN (host half) | The host-side validator this file documents for it (invariant 3) is unimplemented — F-002; the engine-side ordering (receipts-before-events in the output hub, CLI host.rs:14-15) is asserted by the file's callers but proven by the playback-engine wave, not here. |
| "the conformance harness every host binding feeds" | correlation.rs:102 | FALSE | Only apps/desktop feeds DeliveryLog; CLI/web have zero references — F-002. |
| "Correlation and ordering (doc 05 §4) — the card 1.D3 AC lives here" | correlation.rs:1 | REAL | The stale-generation AC tests exist and executed green in baseline (log 1339-1349). |

## 7. Dead / duplicated / unreachable code
`DeliveryLog.answered` (116) — written, never read (F-002). `StalenessFilter::observe_response` — not dead, but production-reachable from ONE of three hosts (F-001). No duplication; no `#[allow]`; no unsafe; no feature gates.

## 8. Dependencies & flags
Only `std` + `crate::envelope`. Crate `commands` is a dependency of apps/cli (Cargo.toml:52), apps/web (:54), apps/desktop (:33), apps/control-server, crates/playback-engine — the correlation types are the shared host vocabulary. Built clean in baseline #1; tests executed in #3/#4.

## 9. Open questions for the next agent
- apps/cli + apps/web auditors: confirm no receipt→response conversion path exists that SHOULD call `staleness.observe_response` (F-001) — the CLI's `pending_receipts` (host.rs:663) is the likely seam; if receipts carry generations, the fix is one line per host.
- commands/catalog auditor: does any dispatch path mint `CommandResponse { session: None, result: Ok(Accepted { session: Some(_) }) }` (session.open)? If yes, `observe_response`'s early return drops that watermark raise — see F-001/table note.
- doc-05 / UI auditor: `ui/packages/client` (§5.1's TS typed client "implementing the staleness filter … once") does not exist in the tree — where does the web UI actually filter stale events, if anywhere?
