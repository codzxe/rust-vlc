# AUDIT: apps/desktop/src/mailbox.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 364 (incl. ~110 test lines) / 12 non-test items + 4 tests |
| File verdict | REAL (in-process queue + accounting verified by tests; shipped reachability conditional on a UI command) |
| Reachable from shipped entry point? | YES, conditionally: lumen-tauri main.rs → app.rs:93 `ShellRuntime` → shell.rs:180 `FrameTap::new()` (first `session.set-frame-channel` command) → :187 `set_present_tap` → frame_tap.rs (MailboxTransport) → transport.rs:199 `FrameMailbox::new(config)`. The tap is installed lazily — only when the webview UI sends `session.set-frame-channel`; until then the mailbox never exists. Runtime UNVERIFIED (desktop-tauri gdk-sys sandbox limit); unit level VERIFIED. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | ./target/debug/deps/desktop-c2864a891fe6f82b mailbox → 4 mailbox tests passed (plus frame_tap/host tap tests in the same filter, 6 total, 0.01s). |

## 1. One-paragraph verdict
A small, correct, honest data structure: a bounded latest-wins VecDeque with exact overflow/stale accounting and a deterministic min/max/floor-mean latency accumulator (i128 sum, no overflow, no clamping of negative samples). The arithmetic is pinned by tests that would fail on any off-by-one (5 offers into capacity 2 → dropped 3, survivors 3 and 4, order preserved; double flush counted once; floor mean on negatives). The defects are all hygiene: a structurally unreachable `capacity == 0` error arm (the type system already forbids it), a `conserves()` identity whose doc omits the empty-queue precondition (its own test has to add `mailbox.len()`), one Display-only tautology test, and a latency accumulator that no shipped runtime code consumes (it is the test harness's evidence primitive, used by apps/desktop/tests/pipeline.rs).

## 2. What actually works (verified)
— `./target/debug/deps/desktop-c2864a891fe6f82b mailbox` → 6 passed (4 in this module + frame_tap's full-mailbox test + host's generation-flush test): latest-wins drop-oldest counting, backlog-inclusive conservation, wholesale flush counted once, floor-mean arithmetic on negatives.
— The whole `offer`/`draw_next`/`flush_stale` state machine is 30 lines with no panics, no unchecked indexing, no blocking.

## 3. Findings
### F-001 [S4] The zero-capacity error arm is structurally unreachable
Where: apps/desktop/src/mailbox.rs:142-147
Code (sed-verified):
```rust
    pub fn offer(&mut self, frame: ShippedFrame) -> Result<(), MailboxError> {
        if self.capacity == 0 {
            return Err(MailboxError::Misconfigured {
                reason: "zero capacity",
            });
        }
```
What is wrong: `capacity: usize` is set only from `config.capacity.get()` (`NonZeroUsize`, :134) — no code path can produce 0, so the arm (and `MailboxError::Misconfigured` itself) can never fire. The doc (:128-131) admits it: "never fires for a constructed `MailboxConfig` today". Dead-by-design, tested only by constructing the error by hand. Confidence: HIGH — Verification: read (rg: no other constructor of FrameMailbox).

### F-002 [S4] `conserves()` documents an identity that only holds on an empty queue
Where: apps/desktop/src/mailbox.rs:107-112
Code (sed-verified):
```rust
    /// Conservation identity: every offered frame is drawn or
    /// dropped, exactly once.
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self) -> bool {
        self.offered == self.drawn + self.dropped_overflow + self.dropped_stale
```
What is wrong: with frames still queued, `offered` exceeds the right side by `len()` — the module's own test (:294-298) must add `mailbox.len() as u64` to make the identity hold mid-stream. The doc omits the precondition. Confidence: HIGH — Verification: read + ran (both tests passed).

### F-003 [S4] One tautology test + a harness-only type in the public API
Where: apps/desktop/src/mailbox.rs:351-363
Code (sed-verified):
```rust
        let err = MailboxError::Misconfigured {
            reason: "zero capacity",
        };
        assert!(err.to_string().contains("zero capacity"));
```
What is wrong: the test constructs the error itself and asserts its own literal — it proves Display plumbing, nothing about the mailbox (the name is honest about this). Related: `LatencyAccountant`/`LatencySummary` are re-exported in lib.rs:106 but consumed only by apps/desktop/tests/pipeline.rs — no shipped runtime caller; the module doc's "compatibility matrix cell is measured arithmetic" is a test-harness claim, not a product path. Confidence: HIGH — Verification: read (rg repo-wide: only lib.rs re-export + tests/pipeline.rs).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MailboxConfig::default | 50-62 | REAL | — | const-eval `loop {}` arm for the impossible zero; builds capacity 2 |
| MailboxError (Display/Error) | 66-85 | REAL | S4 | unreachable variant (F-001) |
| MailboxStats::conserves | 106-113 | REAL | S4 | precondition undocumented (F-002) |
| FrameMailbox::new | 132-138 | REAL | — | infallible for valid configs |
| FrameMailbox::offer | 142-154 | REAL | S4 | correct drop-oldest; dead zero-capacity arm (F-001) |
| FrameMailbox::draw_next | 157-161 | REAL | — | FIFO order pinned by test |
| FrameMailbox::peek / len / is_empty | 163-179 | REAL | — | trivial, correct |
| FrameMailbox::flush_stale | 184-190 | REAL | — | wholesale, counted once (test ran) |
| FrameMailbox::stats | 192-196 | REAL | — | copy out |
| LatencyAccountant::record | 209-223 | REAL | — | i128 sum; min/max correct incl. negatives |
| LatencyAccountant::summary | 226-239 | REAL | — | floor division documented and pinned (test ran: (10−3)/2 → 3) |
| tests (4) | 280-363 | REAL | S4 | 3 strong + 1 tautology (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| latest_wins_drops_oldest_and_counts | YES | exact counts (5/3/2), survivor ids 3 and 4, order, conservation with backlog |
| generation_flush_drops_everything_counted_once | YES | flush returns 3 then 0, stats 3/2, post-flush flow |
| latency_accountant_is_exact_floor_arithmetic | YES | min/max/mean exact incl. negative floor case |
| zero_capacity_is_unconstructible_but_typed | NO | constructs its own error, asserts its own literal (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Bounded: capacity frames, non-zero by construction" | :7-8 | REAL | NonZeroUsize; type system |
| "when the mailbox is full, the oldest un-consumed frame is dropped (counted) — the freshest frame always wins" | :9-12 | REAL | latest_wins test ran |
| "Generation-stale frames are purged wholesale" | :13-14 | REAL | flush test ran; host::tests::generation_bumps_signal_the_mailbox_flush ran |
| "Every event is counted — offered, drawn, dropped (overflow vs stale)" | :21-23 | REAL | stats asserted exactly in tests |
| "the compatibility matrix's 'dropped frames' cell is measured arithmetic" | :21-23 | REAL-UNPROVEN | counters are real; no shipped surface reads MailboxStats today (rg: no non-test reader outside frame_tap/transport seam — the UI receipt is the prototype harness) |

## 7. Dead / duplicated / unreachable code
- `MailboxError::Misconfigured` + offer's zero-capacity arm: unreachable (F-001).
- `LatencyAccountant`/`LatencySummary`: test-harness-only via public API (F-003) — exported but no shipped consumer.
- `MailboxStats::conserves`: test-only helper (used by mailbox.rs and shell.rs's own EventStats analogue).

## 8. Dependencies & flags
std-only + `crate::wire::DecodedFrame` (ShippedFrame alias). No unsafe, no feature gates, no clocks inside (deterministic; stamps come from callers).

## 9. Open questions for the next agent
- shell.rs owner: if the webview never sends `session.set-frame-channel`, the whole mailbox lane stays unconstructed — does the shipped frontend actually send it (apps/web UI source, unverifiable from this crate)?
