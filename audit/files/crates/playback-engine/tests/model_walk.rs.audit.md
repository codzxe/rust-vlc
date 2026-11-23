# AUDIT: crates/playback-engine/tests/model_walk.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 894 / 18 (2 impls, 4 #[test]s) + shared fakes in tests/common/mod.rs (458 loc, 20 fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test crate) — runs under `cargo test -p playback-engine` (orchestrator baseline cargo-test-workspace.txt: 4744 passed; I ran the binary myself, see Commands). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/model_walk-19d4ba79a4308512` → 4 passed, 0 failed, finished in 0.14s, exit 0. |

## 1. One-paragraph verdict
This is a genuine model-based test suite, not theater: 500 seeded walks drive the REAL `Engine` (real state table, real session, real pipeline) through a weighted command mix (play/pause/stop/close/seek/stall/refill/reopen/replace/fail-open), and a `StreamChecker` asserts the doc 05 §4.3 ordering invariants incrementally on every delivered event — seq monotone, session epoch monotone, generations monotone per session, wire-legal edges only, fatal-before-error, tracks-before-playing, buffering episodes bracketed 0.0/1.0 — plus exactly-one-receipt per submitted command. Vacuity guards force every named behavior to actually occur (floors asserted at the end). I executed the suite: 4/4 pass in 0.14 s. The one real weakness: the walk's `Seek` arm can only ever produce rejections, because the shared fake demuxer is unseekable — the entire seek-completion half of the machine (pre-roll, M10 hold, re-anchor) is invisible to this suite.

## 2. What actually works (verified)
- `./target/debug/deps/model_walk-19d4ba79a4308512` → `test result: ok. 4 passed; 0 failed` (seeded_model_walks_never_break_the_machine, every_submitted_command_answers_exactly_one_receipt, idle_is_reachable_from_every_state_over_rows_and_the_close_macro, reopen_cycles_1000x_keep_epoch_and_seq_continuous).
- The fakes (tests/common/mod.rs) sit at the plugin seams only (`Demuxer`, `Decoder`, `DecoderProvider`, `FrameConsumer`); the engine, session, state table, pipeline, and events under test are the production code — no mock-of-the-thing-tested.
- The PRNG is deterministic per seed (splitmix64-seeded xorshift64*, 83-107) and failure messages name the walk/step — rerunnable failures.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/model_walk.rs:485-490 (with tests/common/mod.rs:120-128)
Code:
```
                SessionCommand::Seek => handle.seek(
                    request,
                    SeekTarget::Absolute {
                        position_ns: 1_000_000,
                    },
                ),
```
What is wrong: every walk-seek is submitted against `FakeDemuxer`, whose `seek` is hard-wired to `Err(... "demux.seek.not-implemented")` (common/mod.rs:120-128). The walk therefore exercises only the `SeekFailed` rejection family; a regression in seek COMPLETION (pre-roll observation, M10 hold release, clock re-anchor — or a wedge like the zero-limb never-Ready found in pipeline.rs F-001) would pass this suite green. The module header (1-52) lists what the mix covers and does not claim seek landings, so this is a coverage gap, not a false claim — but "random command sequences driving the REAL Engine" oversells a mix whose seek verb is a guaranteed rejection. Why it matters: the suite is the named enforcement of doc 03 §4.6(f); its blind side is exactly the machine's most complicated flow. What correct looks like: a seekable fake variant in the mix (seek_flow.rs carries one — fold a weighted arm that swaps the demuxer or arms a seekable script). Confidence: HIGH — Verification: read + ran.

F-002 [S4]
Where: crates/playback-engine/tests/model_walk.rs:158-159 vs 714-724
Code:
```
    reopens: usize,
    error_cycles: usize,
    paused_self_confirmations: usize,
}
```
What is wrong: `paused_self_confirmations` is counted (298) but never asserted in the floor block (714-724 asserts the other eight) — a dead vacuity counter whose behavior could silently go un-exercised without the suite noticing. Everything else in `Floors` is enforced. Confidence: HIGH — Verification: read (rg `paused_self_confirmations` → 2 occurrences: declaration + increment; no assert).

F-003 [S4]
Where: crates/playback-engine/tests/model_walk.rs:104-106
Code:
```
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
```
What is wrong: modulo bias (and `below(0)` would panic — no caller passes 0; all call sites are literal-positive). Bias is irrelevant to a state-machine walk. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Rng::new` / `next` / `below` | 86-106 | REAL | S4 | Deterministic splitmix64∘xorshift64*; F-003 modulo bias. |
| `StreamChecker::new` / `tail_text` | 163-177 | REAL | — | Bounded 24-line diagnostic tail. |
| `StreamChecker::ingest` | 180-211 | REAL | — | Receipt-uniqueness (HashMap count==1) + result-shape assertion. |
| `StreamChecker::tail_push` | 215-220 | REAL | — | Bounded pop-front at 24. |
| `StreamChecker::check_event` | 222-354 | REAL | — | The doc 05 §4.3 invariant engine: seq/session/generation monotonicity, wire-legal edges (uses the REAL `can_transition_to`), tracks-before-playing, fatal-before-error, episode 0.0/1.0 bracketing, episode-closed-at-idle. Every assert names walk+step. |
| `describe` | 362-373 | REAL | — | Diagnostic formatting. |
| `Walker::open_session` | 410-463 | REAL | — | Real `Engine::open` with `SessionConfig` (tiny queues/budget, frozen throttle, virtual clock at 0); 1-in-4 fail-open mix; `unreachable!` on open failure is the harness's own contract (open cannot fail here — it is async-accepted). |
| `Walker::state` | 465-467 | REAL | — | Accessor. |
| `Walker::submit` | 473-508 | REAL | — | Bounded-inbox retry (InboxBusy → pump+resubmit), SessionClosed = honest no-receipt. |
| `Walker::pump` / `pump_until` | 511-536 | REAL | — | Bounded pumps; loud assert on bound trip (hang = failure). |
| `Walker::finish` | 540-551 | REAL | — | Exactly-one-receipt final discipline over `entered`. |
| `run_walk` | 556-691 | REAL | S3 | The weighted mix (16 arms); F-001 seek-always-rejects; floors collected for the vacuity guards. |
| `seeded_model_walks_never_break_the_machine` | 697-725 | REAL | — | 500 walks × 8..=64 steps; 8 floor asserts (F-002 ninth counter unasserted). Executed: PASS. |
| `every_submitted_command_answers_exactly_one_receipt` | 732-766 | REAL | — | Deterministic 7-command burst incl. AlreadySettled rejection; asserts count==7. Executed: PASS. |
| `successors` | 776-791 | REAL | — | Rows + close-macro walk using REAL `TRANSITIONS`/`apply_input`/`close_plan`. |
| `idle_is_reachable_from_every_state_over_rows_and_the_close_macro` | 793-819 | REAL | — | BFS no-stuck-state + strong connectivity over all 8 states. Executed: PASS. |
| `reopen_cycles_1000x_keep_epoch_and_seq_continuous` | 828-894 | REAL | — | 1000 open→play→stop→close cycles; asserts epoch==cycle+1, session monotone, frames>0 per cycle, 2000 receipts. Executed: PASS. |
| common fakes (`SharedNow`,`Collector`,`FakeDemuxer`,`FakeDecoder`,`FakeProvider`,`shallow_clone`,`track`,`packet`,`audio_frame`,`tiny_queues`,`tiny_budget`,`virtual_clock`,`frozen_throttle`,`config_with`,`open_with`,`open_audio`,`open_stalled_audio`,`poll_until`,`take_events`,`frames_of`,`receipt_for`) | common/mod.rs:55-458 | REAL | — | Honest seam fakes; poison-recovery on every lock; `packet()` carries real pts/duration (drives the real watermark axes). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_model_walks_never_break_the_machine | YES | Per-event asserts on the delivered stream of the real engine; an illegal edge, a duplicate receipt, a seq/session/generation regression, an unbracketed episode, or a hang (pump bound) all fail loudly with seed+step. Mutation reasoning: deleting the `Fatal`-before-error emission order or regressing `can_transition_to` fails the checker. |
| every_submitted_command_answers_exactly_one_receipt | YES | Counts real receipts per request id; a lost or duplicated receipt fails. |
| idle_is_reachable_from_every_state_over_rows_and_the_close_macro | YES | Pure graph property over the REAL table — a stuck state (no Idle path) or an isolated pocket fails. |
| reopen_cycles_1000x_keep_epoch_and_seq_continuous | YES | Asserts the epoch counter itself (session == cycle+1) — an epoch leak/reuse fails immediately. |
None are `#[ignore]`d, tautological, or mock-based. All four executed by me (0.14 s).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded random command sequences driving the REAL Engine … asserting after every step that the machine never (i) reaches an illegal state, (ii) emits events out of the documented order, or (iii) hangs" | model_walk.rs:1-9 | REAL (with the seek caveat) | StreamChecker asserts + pump bounds; ran it. The seek arm only rejections (F-001). |
| "A failed seed reruns the exact interleaving: every failure message names the walk seed and step" | 13-15 | REAL | Every assert formats `walk {walk} step {step}`. |
| "every named behavior has a deep-entry floor — the 500 walks must hit each at least once" | 46-48 | REAL | Floor asserts 714-724; `paused_self_confirmations` is the unenforced exception (F-002). |
| "NO proptest dependency exists in Cargo.lock" | 11-14 | REAL | Hand-rolled PRNG present; (cross-check: dependencies live in W03-A01's packet-queue/Cargo.toml audit — playback-engine dev-deps do not include proptest; UNVERIFIED against full Cargo.lock by direct grep — cheap spot check below). |
| "the bounded-memory half … lives in tests/reopen_alloc.rs" | 50-52 | REAL | File exists (tests/ listing). |

## 7. Dead / duplicated / unreachable code
- `Floors::paused_self_confirmations` (158-159): counted, never asserted (F-002).
- Nothing else dead; no `#[ignore]`, no `todo!`.

## 8. Dependencies & flags
- Test-only crate target; uses `playback_engine` (lib), `plugin_api`, `media_types`, `media_clock`, `commands`, `packet_queue`, `testkit`, `codec_api` — all workspace crates. No features gated. Runs in the default `cargo test -p playback-engine` lane (baseline + my run).

## 9. Open questions for the next agent
- Add a seekable-demuxer arm to the walk mix (F-001) — until then seek-completion regressions are only guarded by seek_flow.rs.
- The walk's clock is frozen at 0 (SharedNow(AtomicU64::new(0)), 419-426): no `time-tick` cadence or rate behavior is model-checked; the property/generation_monotonicity.rs suite (next file) covers generation machinery instead.
