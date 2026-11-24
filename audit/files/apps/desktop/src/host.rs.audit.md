# AUDIT: apps/desktop/src/host.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 396 / 11 production items + 5 tests |
| File verdict | REAL — small, honest, tested correlation seam; delegates the actual logic (RequestIdAllocator / StalenessFilter / DeliveryLog) to the commands crate instead of duplicating it; the typed webview-pending refusal is genuine. Two doc/observability defects below. |
| Reachable from shipped entry point? | PARTIAL YES: the shipped `desktop` binary (main.rs) calls `DesktopHost::active_presentation_path()` (main.rs:62) and `desktop::strategy_for(os)` (main.rs:74) — both defined here. The `DesktopHost` correlation half is consumed only by `ShellRuntime` (shell.rs:102) → dead in the release lane (see shell.rs audit F-002), live only in the compile-broken excluded crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `violations` over apps/ crates/commands/src (readers: only host.rs's own tests + correlation.rs's own tests); rg baseline host tests (cargo-test-workspace.txt:2545-2549, all ok); sed quote verification |

## 1. One-paragraph verdict
This is the cleanest file of my assignment: a 225-line production surface that does exactly what it says — request-id allocation with §4.3 log pre-registration, staleness-filtered event delivery, generation-bump detection for the mailbox flush, and ADR-003's per-OS landing as queryable data — all by DELEGATING to the commands crate's correlation primitives rather than re-implementing them. `open_webview` is a typed refusal, never a fake window, and says so. The defects are: the §4.3 violation surface is write-only in production (nothing anywhere reads `violations()`; the doc claims "the host surfaces the violation"), and the ADR-004 status strings have drifted (host says "pending"; the admission record says the code half was ADMITTED 2026-11-13, and the Windows "prototype decides" row is stale now that the canvas-stream wrapper ships in release payloads).

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:2545-2549 (`host::tests::{generation_bumps_signal_the_mailbox_flush, ids_are_monotonic_and_responses_echo_legally, stale_events_are_dropped_current_ones_kept, strategy_table_is_the_adr_landing, webview_open_is_a_typed_pending_never_a_fake}` all `ok`). These assert real behavior: ghost-id responses produce violations, generation raises produce bumps, strictly-lower generations drop, the typed pending is `HostError::WebviewPending { adr: "ADR-004" }`. (Not re-run: covered by the shell.rs live run's crate compilation; cargo-build-budget.)

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/host.rs:195-199, 203-207
Code:
```
        // The seq check records violations but does not gate delivery
        // (§4.3 is a validator: the host surfaces the violation, the
        // conformance suite fails on it — the UI still sees the
        // event).
        let _ = self.log.observe_event(event);
```
What is wrong: the comment claims "the host surfaces the violation" — but `violations()` has NO production reader anywhere: rg over apps/ and crates/commands/src shows callers only in this file's own tests (host.rs:276,287,345) and the commands crate's own tests (correlation.rs:352-394). Neither the shell runtime, nor the tauri wrapper, nor the UI ever queries it; the `observe_event` return is itself discarded. A §4.3 seq violation on the desktop lane is recorded into a buffer nobody drains. Why it matters: the "validator" half of the §4.3 contract is theater in production — violations exist only to make tests pass; the conformance-suite claim is about other lanes, not this host's runtime. What correct looks like: the wrapper's `forward_pending` logs violations (it already holds the lock), or the doc stops claiming surfacing. Confidence: HIGH — Verification: read + rg.

F-002 [S4]
Where: apps/desktop/src/host.rs:212-214, 99-102
Code:
```
    pub fn active_presentation_path() -> &'static str {
        "skeleton: canvas-stream protocol core only — no webview host compiled (ADR-004 pending; docs/prototypes/video-in-webview.md)"
    }
```
and
```
        TargetOs::Windows => (
            PresentationVariant::PrototypePending,
            "ADR-003 leaves A-or-B to the prototype; matrix cells pending a desktop run",
```
What is wrong: two stale status strings. (a) "ADR-004 pending" — the admission record says the code half is ADMITTED (apps/desktop-tauri/INFRA-BLOCKED.md: "Status: ADMITTED (code half) — 2026-11-13"); what remains pending is launch verdicts. (b) The Windows row reports "A-or-B (prototype decides)" while the canvas-stream (B) wrapper is the admitted, release-payload-shipped path for all three OSes (INFRA-BLOCKED §B.1 "the real app ships in every payload") — the "truthful capability reporting" table (`strategy_for`, doc comment: "decision 4: the active path is an engine fact, never marketing") is one decision-behind on Windows. Why it matters: main.rs prints this table as the per-OS landing; the feature matrix quotes it. Confidence: HIGH — Verification: read + INFRA-BLOCKED.md comparison.

F-003 [S4]
Where: apps/desktop/src/host.rs:199
Code:
```
        let _ = self.log.observe_event(event);
```
What is wrong: the observation result is discarded (distinct from F-001's reader problem: even the boolean return is dropped). Deliberate per the comment (does not gate delivery), but combined with F-001 the entire §4.3 event-side check produces zero runtime effects. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PresentationVariant (enum) + label | 29-53 | REAL | — | static ADR labels |
| TargetOs (enum) + webview | 57-76 | REAL | — | static webview names |
| StrategyReport (struct) | 79-87 | REAL | — | data row |
| strategy_for | 93-113 | REAL | S4 | ADR data; Windows row stale (F-002b) |
| ResponseObservation (struct) | 117-126 | REAL | — | log_ok + generation_bump |
| EventDelivery (enum) | 129-136 | REAL | — | Kept / DroppedStale |
| DesktopHost (struct) | 139-144 | REAL | — | composition of commands-crate primitives |
| DesktopHost::new | 149-155 | REAL | — | fresh correlation state |
| next_request_id | 159-163 | REAL | — | allocate + log pre-registration |
| observe_response | 167-185 | REAL | — | watermark before/after, bump detection correct incl. None→Some arm |
| deliver_event | 191-201 | REAL | S3 | §4.2 filter then seq check; write-only diagnostics (F-001/F-003) |
| violations | 205-207 | REAL | S3 | no production reader (F-001) |
| active_presentation_path | 212-214 | REAL | S4 | honest for THIS crate; stale ADR-004 wording (F-002a) |
| open_webview | 222-224 | REAL | — | typed refusal, always; tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ids_are_monotonic_and_responses_echo_legally | YES | asserts a+1==b, ghost id 99 → !log_ok + violation recorded |
| generation_bumps_signal_the_mailbox_flush | YES | None→Some and 2→5 bumps, same-gen no-bump |
| stale_events_are_dropped_current_ones_kept | YES | strictly-lower drop, equal keep, newer keep + watermark raise, non-monotonic seq recorded |
| strategy_table_is_the_adr_landing | WEAK | pins static data to static data (would catch a table edit, not a behavior bug) |
| webview_open_is_a_typed_pending_never_a_fake | YES | asserts the Err variant + Display text |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the host surfaces the violation, the conformance suite fails on it" | host.rs:196-197 | FALSE in production | no reader of violations() outside tests (rg, F-001) |
| "ADR-003 per-OS landing ... with the honest status line for each" | host.rs:13-17 | PARTLY STALE | Windows row predates the admitted B wrapper (F-002b) |
| "never fake a window" (typed pending) | host.rs:10-11, 219-224 | REAL | open_webview always Err(WebviewPending); test pins it |
| "ADR-004 pending" | host.rs:213 | STALE | INFRA-BLOCKED.md: ADMITTED (code half), 2026-11-13 |

## 7. Dead / duplicated / unreachable code
- `violations()`: production-dead (test-only API). Not duplicated; delegation posture is correct.

## 8. Dependencies & flags
- Only `commands` crate types + crate-local HostError. No features, no unsafe, std-only (the crate's C5 posture).

## 9. Open questions for the next agent
- Does the commands crate's `DeliveryLog` get read by the CLI/control-server hosts at runtime (making the §4.3 surface live in those lanes)? If yes, only the desktop lane is blind.
- W14-A04: when fixing the ADR-004 status strings, sync host.rs:213 + the Windows row with the admission record.
