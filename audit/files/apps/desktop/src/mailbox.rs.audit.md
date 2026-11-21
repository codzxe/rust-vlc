# AUDIT: apps/desktop/src/mailbox.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 364 / 20 |
| File verdict | REAL (core mailbox real and tested; two documented disciplines are unwired in production) |
| Reachable from shipped entry point? | CONDITIONAL — NOT from the `lumen` CLI. Chain to the shipped desktop binary: `lumen-tauri` (apps/desktop-tauri, workspace-EXCLUDED crate) → depends on `desktop` crate → shell.rs `take_frame_records` (shell.rs:219-224) → frame_tap → `MailboxTransport` (transport.rs:184-237) → `FrameMailbox::offer` (via `send`, transport.rs:231). But `lumen-tauri` itself NEVER BUILT in the sandbox (baseline #2, exit 101, missing gdk/gtk pkg-config) — production reachability UNVERIFIED at runtime; and the workspace-visible `lumen-desktop` bin is a typed stub (reports WEBVIEW_PENDING, exits 86 — pinned by apps/desktop/tests/pipeline.rs:317-322). The lib is exercised by its own suites (baseline green). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | Grep FrameMailbox/MailboxConfig/LatencyAccountant across apps/ + crates/ (users: lib.rs re-export :105-106, transport.rs :25/:184-237, frame_tap.rs :39/:57; LatencyAccountant consumer = tests/pipeline.rs only); Grep flush_stale/generation_bump across apps/ and apps/desktop-tauri (ZERO production callers; signal produced host.rs:175-183, consumed only by tests pipeline.rs:294-299); Read transport.rs 120-249 (MailboxTransport::send → mailbox.offer); Read host.rs 95-169 + 280-344 (generation_bump is a signal, no flush wiring); Read shell.rs 185-259 (drain path, no generation gating); rg mailbox tests in baseline: orchrun-test-workspace-nofailfast.log.txt:2284-2287 + 2333 all `ok`; Read tests/pipeline.rs 240-324; cat apps/desktop-tauri/Cargo.toml + apps/desktop/Cargo.toml (bin names, dependency direction) |

## 1. One-paragraph verdict
The mailbox core is honest, small, and correctly tested: bounded latest-wins with counted overflow drops, wholesale stale flush, exact floor-arithmetic latency stats, and a conservation identity — all pinned by real tests that ran green in the baseline. Two of the module's four advertised disciplines are nevertheless UNWIRED in production: (1) the generation flush — the docs say "the host flushes on generation changes … `host.rs` wires it", but `host.rs` only EMITS a `generation_bump` signal that no production code consumes; `flush_stale` has zero production callers, and the integration test performs the flush by hand under the comment "The wiring flushes on the signal" — the test is the wiring; (2) latency accounting — `LatencyAccountant` is fed only by the test simulation and the JS prototype, never by a production draw path. Both are S2/S4 drift between the module's discipline documentation and the shipped code, with a concrete user-visible consequence (up to `capacity` stale frames presented after a seek/stop in the desktop lane once lumen-tauri actually builds).

## 2. What actually works (verified)
- All 4 unit tests + the integration flush test green in baseline: `test mailbox::tests::{generation_flush_drops_everything_counted_once, latency_accountant_is_exact_floor_arithmetic, latest_wins_drops_oldest_and_counts, zero_capacity_is_unconstructible_but_typed} ... ok` (orchrun-test-workspace-nofailfast.log.txt:2284-2287) and `test generation_bump_flushes_in_flight_frames ... ok` (:2333).
- Latest-wins mechanics verified by hand against `offer` (mailbox.rs:148-151): drop-oldest-when-full, counted, never blocks, never reorders; the test's draw order (ids 3,4 after 5 offers at capacity 2) pins it.
- `LatencyAccountant` arithmetic: min/max exact, mean floor-division with i128 sum (mean of i64 samples is always within i64 — the `as i64` cast at :236 cannot truncate); negative samples preserved, floor semantics pinned by test (:344-348).
- `MailboxConfig::default()` capacity 2 via const `NonZeroUsize` (the `loop {}` arm is compile-time dead — a build hang, not a runtime panic).

## 3. Findings
### F-001 [S2]
Where: apps/desktop/src/mailbox.rs:13-20 (claim), 184-190 (the unwired API)
Code:
```rust
//! - **Generation-stale frames are purged wholesale**: the host
//!   flushes on generation changes (the command layer's gen-bumping
//!   events — `host.rs` wires it; the engine-side twin is
//!   `PresentationQueue::publish`).
```
What is wrong: The wiring does not exist. `host.rs` computes `ResponseObservation.generation_bump` (host.rs:125, 175-183) — a SIGNAL — but a repo-wide grep shows **no production consumer**: nothing reads `generation_bump` to call `flush_stale`; `flush_stale`'s only callers are mailbox.rs's own unit tests and the integration test, which performs the flush manually:
```rust
    // The wiring flushes on the signal.
    let flushed = bridge.transport_mut().mailbox_mut().flush_stale();
```
(apps/desktop/tests/pipeline.rs:298-299 — the comment describes wiring the test itself supplies). The shell's drain path (`take_frame_records` → `tap.drain_records`, shell.rs:219-224) has no generation gating, and the excluded `lumen-desktop-tauri` crate contains no flush wiring either (grep: no matches).
Why it matters: The doc's own scoping says the flush exists for "the ≤ capacity frames already in flight at a seek/stop" — exactly the frames a real desktop session would present STALE after a seek/stop/track-switch once `lumen-tauri` builds (default capacity 2 → up to 2 pre-seek frames drawn post-seek). The discipline is advertised as wired and is not; the one test that claims to prove the wiring fabricates the wiring inside the test body.
What correct looks like: The ticker/`pump_events` seam (or the bridge) consumes `generation_bump` and calls `mailbox_mut().flush_stale()`; the integration test then drives ONLY the signal (present frames → observe response → assert the mailbox drained itself), not the flush call.
Confidence: HIGH — Verification: read + repo-wide greps (apps/ and apps/desktop-tauri/); UNVERIFIED only at runtime (lumen-tauri never built — baseline #2).

### F-002 [S4]
Where: apps/desktop/src/mailbox.rs:27-33 (claim), 201-240 (the harness-only type)
Code:
```rust
//! Every record carries `emitted_ns` (bridge serialize instant) and
//! `pts_ns` (scheduled slot). The consumer stamps its draw instant
//! and feeds [`LatencyAccountant`]: transport+queue+draw latency is
//! `drawn − emitted`; slot drift (the A/V-sync observable) is
//! `drawn − pts`.
```
What is wrong: No production consumer stamps a draw instant or feeds an accountant. `LatencyAccountant`'s only Rust consumer is the test simulation (apps/desktop/tests/pipeline.rs:64-65, 150); `emitted_ns` is stamped by the bridge (bridge.rs:231) but nothing in `src/` computes `drawn − emitted` or `drawn − pts`. The doc sentence describes the harness/prototype design in the present tense as if it were the shipped consumer path.
Why it matters: A reader believes A/V-sync observability exists in the product; it exists only in the headless simulation and the JS prototype. Latency cells in any desktop matrix would have no production source.
What correct looks like: Either the shell drain (or tauri wrapper) stamps draw time and feeds the accountant, or the doc says "the HARNESS's consumer feeds" — same fix shape as F-001.
Confidence: HIGH — Verification: read + grep (`LatencyAccountant` consumers: lib.rs re-export + tests/pipeline.rs only).

### F-003 [S4]
Where: apps/desktop/src/mailbox.rs:351-363
Code:
```rust
    fn zero_capacity_is_unconstructible_but_typed() {
        // NonZeroUsize makes the config honest; the typed error stays
        // for future knobs (documented invariant, probed through the
        // error type's existence).
        let err = MailboxError::Misconfigured {
            reason: "zero capacity",
        };
        assert!(err.to_string().contains("zero capacity"));
```
What is wrong: The name promises a proof that zero capacity is unconstructible; the body hand-constructs the error variant (trivially possible — the enum is `pub`) and smoke-tests two `Display` impls. Nothing about construction, the `NonZeroUsize` invariant, or `FrameMailbox::new` is exercised. The dead `capacity == 0` arm in `offer` (:143-147) is likewise never reached by any test.
Why it matters: Test names are evidence claims; this one claims a type-safety proof and delivers a string check (checklist: "names promising more than the body").
What correct looks like: Rename to what it is (`error_display_shapes`), or make it a compile-pass proof (e.g. a `const _: () = ...` assertion that `MailboxConfig`'s capacity field is `NonZeroUsize`).
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/desktop/src/mailbox.rs:106-113
Code:
```rust
    /// Conservation identity: every offered frame is drawn or
    /// dropped, exactly once.
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self) -> bool {
        self.offered == self.drawn + self.dropped_overflow + self.dropped_stale
    }
```
What is wrong: The identity is false whenever frames are queued (mid-stream): `offered == drawn + dropped + backlog`. The unit test knows this and asserts the EXTENDED identity with `mailbox.len()` (mailbox.rs:294-298) before drawing, then calls `conserves()` only after the queue empties (:305). The doc comment states the identity unconditionally, so a mid-stream `conserves()` check on a perfectly healthy mailbox returns `false`.
Why it matters: The one-line "observable evidence" getter misleads exactly when an engineer would use it (live diagnostics mid-playback).
What correct looks like: `self.offered == self.drawn + self.dropped_overflow + self.dropped_stale + self.queue.len() as u64`, or a doc that says "holds when the queue is empty".
Confidence: HIGH — Verification: read + the test's own two-phase assertion pattern.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MailboxConfig` + `Default` | 41-62 | REAL | — | Capacity 2 const-constructed; `loop {}` impossible arm is a compile-time dead end (documented 2.C4 convention, clippy allow justified). |
| `MailboxError` + `Display`/`Error` | 64-85 | REAL | — | Single variant, forward-knob placeholder; honest. |
| `ShippedFrame` alias | 87-89 | REAL | — | Alias over wire::DecodedFrame — the wire record IS the mailbox payload (no re-wrap). |
| `MailboxStats` + `conserves` | 91-113 | REAL | S4 (F-004) | Stats fields all written by real paths; conserves doc false mid-stream. |
| `FrameMailbox::new` | 132-138 | REAL | — | `with_capacity` pre-allocation; Err arm structurally unreachable (documented). |
| `offer` | 142-154 | REAL | — | Latest-wins drop-oldest, counted; dead `capacity == 0` arm (NonZeroUsize upstream) documented as never firing. |
| `draw_next` | 157-161 | REAL | — | FIFO pop, counted. |
| `peek` / `len` / `is_empty` | 163-179 | REAL | — | Pure inspection. |
| `flush_stale` | 181-190 | REAL | S2 (F-001) | Correct implementation, zero production callers — the documented wiring (host.rs) does not exist. |
| `stats` | 192-196 | REAL | — | Copy out. |
| `LatencyAccountant::record` | 209-223 | REAL | S4 (F-002) | Exact min/max, i128 sum; no production feeder. |
| `LatencyAccountant::summary` | 225-239 | REAL | — | Floor mean; `as i64` safe (mean bounded by sample range). |
| `LatencySummary` | 241-253 | REAL | — | Plain readout. |
| `tests::frame` | 262-278 | REAL | — | Hand-built detached frames (documented convention). |
| `tests::latest_wins_drops_oldest_and_counts` | 280-306 | REAL | — | Mutation-hard: counts, order, backlog identity, conservation. |
| `tests::generation_flush_drops_everything_counted_once` | 308-328 | REAL | — | Flush idempotence + post-flush flow pinned. |
| `tests::latency_accountant_is_exact_floor_arithmetic` | 330-349 | REAL | — | Exact + floor-on-negative pinned. |
| `tests::zero_capacity_is_unconstructible_but_typed` | 351-363 | REAL | S4 (F-003) | Display smoke test wearing a proof's name. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| latest_wins_drops_oldest_and_counts | YES | Exact offered/dropped/drawn counts + draw order + backlog identity. |
| generation_flush_drops_everything_counted_once | YES | Flush count, non-cumulative second flush, post-flush flow. |
| latency_accountant_is_exact_floor_arithmetic | YES | Exact mean, floor on negatives, empty-state Nones. |
| zero_capacity_is_unconstructible_but_typed | NO | Nothing about the subject (mailbox construction) is exercised — Display strings only (F-003). |
| tests/pipeline.rs::generation_bump_flushes_in_flight_frames (integration) | NO for the WIRING claim | The test calls `flush_stale` itself (pipeline.rs:299); deleting the (nonexistent) production wiring changes nothing — it would still pass. It proves the flush API works, not that the host flushes. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Bounded: capacity frames, non-zero by construction" | module docs :7-8 | REAL | NonZeroUsize + tests. |
| "Latest-wins … order is never violated" | module docs :9-12 | REAL | offer/draw_next + latest_wins test (baseline :2286). |
| "the host flushes on generation changes … `host.rs` wires it" | module docs :13-16 | FALSE | F-001: host.rs emits a signal; no production consumer; the integration test hand-flushes (pipeline.rs:298-299). |
| "The consumer stamps its draw instant and feeds `LatencyAccountant`" | module docs :27-31 | SHALLOW | True only in the headless simulation (tests/pipeline.rs) and the JS prototype; no production path (F-002). |
| "Every event is counted … measured arithmetic, not vibes" | module docs :21-23 | REAL | MailboxStats written on every path; conservation tested. |

## 7. Dead / duplicated / unreachable code
- `flush_stale`: production-unreachable (F-001) — reachable only from tests.
- `LatencyAccountant`: production-unreachable (F-002) — simulation-only.
- `offer`'s `capacity == 0` arm: structurally dead (documented).
- No duplication: the engine-side twin (`PresentationQueue::publish`) is a separate implementation in the engine crate by design (cross-process vs in-process), not a copy.

## 8. Dependencies & flags
- std-only + crate::wire (DecodedFrame). No feature gates. The desktop crate builds clean in the workspace (baseline #1); the consumer binary (lumen-tauri) is workspace-excluded and failed to build in-sandbox (baseline #2) — the entire module's production lane rides on a binary with zero recorded successful builds.

## 9. Open questions for the next agent
- bridge.rs / shell.rs owners (W13 siblings): confirm no other flush consumer exists outside apps/ (I grep'd apps/ and crates/); if the tauri wrapper in desktop-tauri is supposed to flush on `generation_bump`, that crate has the wiring gap (grep found none there either).
- Who owns `PresentationQueue::publish` (engine-side twin) — does IT flush on generation changes in production? If yes, the engine path is covered and only the desktop variant-B lane carries stale frames.
- The `[PROPOSAL: v2 wire "flush" record]` (module doc :19-20) — if protocol v2 lands a wire flush, F-001's blast radius shrinks to in-process hosts; track it.
