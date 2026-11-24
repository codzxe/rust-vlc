# AUDIT: crates/testkit/src/faults.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 1261 / 18 prod fns (3 public types) + 17 unit tests |
| File verdict | REAL (code + own tests pass in baseline) but UNREACHABLE from any test lane outside its own unit tests |
| Reachable from shipped entry point? | NO — testkit is dev-only by contract (apps/web/Cargo.toml:124-126); moreover NO consumer outside this file: `rg FlakySource|DeviceLossSimulator|ClockJumpInjector crates/ tests/ apps/` (excluding testkit) → 0 hits |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg usage sweep (0 external users); baseline cite: audit/runs/baseline/cargo-test-workspace.txt:6208-6217 |

## 1. One-paragraph verdict
The injector kit itself is genuinely engineered: `FlakySource::poll` implements
its documented deterministic semantics (offset-stable faults, terminal-fault
ranking, sticky ends, FIFO delays) with saturating arithmetic and zero unsafe,
and its 17 unit tests are honest — every one asserts exact packet ids, exact
bytes (0xA5→0x5A), exact counts, and the baseline run passed all 17
(cargo-test-workspace.txt:6208-6217). But the kit is an orphan: not a single
test outside this file wraps a PacketSource, paces a DeviceLossSimulator, or
reads a ClockJumpInjector — the doc's claim that it is "the driver behind the
MP §13 scenario rows 'corrupt / missing files', 'interrupted network' and
'device loss'" (L2-3) is not backed by any consumer in the tree. The recovery
scenario that DOES exist (tests/integration/first-vertical/tests/recovery_matrix.rs)
consumes the byte-mutated fixture files, not this kit.

## 2. What actually works (verified)
— Baseline (orchestrator run; cargo unavailable in this environment):
audit/runs/baseline/cargo-test-workspace.txt:6208-6217 shows 17
`faults::tests::* ... ok` lines (rg -c → 17 tests, all passing).
— Mutation reasoning on `drop_removes_exactly_the_scripted_packet` (L737-757):
assert_eq!(ids, vec![0,1,2,4,5,6,7]) fails if Drop misses or over-drops;
applied_faults()/next_offset() fail if the counter/pull-position drifts. Honest.
— `poll()` semantics hand-checked against all six Fault arms and their
documented ranking (L108-147 vs code L260-378): consistent, including the
tricky clean-Truncate pre-pull scan (L269-291) and pending/dirty interactions
(L244-258, L354-377).

## 3. Findings
F-001 [S3]
Where: crates/testkit/src/faults.rs:1-4 vs repo-wide usage
Code:
//! Deterministic fault injection for recovery testing (doc 03 §8, card
//! 3.D2) — the driver behind the MP §13 scenario rows "corrupt / missing
//! files", "interrupted network" and "device loss", and the kit half of
//! cards 3.C2 (media-level recovery) and 3.C3 (environment recovery).
What is wrong: the claim "the driver behind the MP §13 scenario rows" is
false in-tree: no scenario row, no integration test, no engine test uses
FlakySource/DeviceLossSimulator/ClockJumpInjector (rg over crates/ tests/
apps/ excluding testkit → 0 hits). The actual recovery evidence lane
(tests/integration/first-vertical/tests/recovery_matrix.rs) drives
byte-mutated fixture files instead. ~650 lines of quality test infra are
reachable only from their own unit tests. Why it matters: the "recovery
testing" story of doc 03 §8 is delivered by a different mechanism; this kit
is decoration until a consumer lands. What correct looks like: wire one
recovery scenario to FlakySource or demote the doc claim.
Confidence: HIGH — Verification: read (rg usage sweep, 2 runs).

F-002 [S4]
Where: crates/testkit/src/faults.rs:43-50
Code:
//! *not* hot path and allocates freely. The measured-allocator proof of
//! this belongs in a dedicated integration binary — the house pattern of
//! `tests/alloc_count_e2e.rs` (the global allocator is process-global; the
//! lib unit-test binary cannot be exclusively serialized against the
//! crate's other unit tests). [PROPOSAL: add
//!   `crates/testkit/tests/faults_zero_alloc.rs` post-merge — Lead /
What is wrong: the "allocation-free per packet by construction" claim (L35)
stays unproven: the proposed `crates/testkit/tests/faults_zero_alloc.rs` does
not exist (ls crates/testkit/tests/ → alloc_count_e2e.rs only). Honest
deferral, never fulfilled.
Confidence: HIGH — Verification: read (ls).

F-003 [S4]
Where: crates/testkit/src/faults.rs:30-31
Code:
//! module knowing their packet shape. [handoff: those impls land with
//! 3.D1's own files; deliberately not written here — ownership]
What is wrong: the handoff never happened — no `impl PacketSource` exists
outside this file's test module (rg `impl PacketSource` → only
crates/testkit/src/faults.rs:714). Composition contract untested against any
real adapter.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/testkit/src/faults.rs:175-176
Code:
    /// End of stream — sticky, every later poll repeats it. `dirty`
    /// marks a mid-packet end (scripted or, one day, real).
What is wrong: "one day, real" — dirty EOF is never produced by any real
demuxer path through this kit (no adapter exists; see F-003). Aspiration, not
behavior.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PacketSource::next_packet (trait) | 61-67 | REAL | — | seam, no external impls (F-003) |
| PacketPayload::{corrupt_byte,truncate_payload} | 73-85 | REAL | — | no-panic OOB contract, tested L838-843/912-913 |
| Fault::at() | 150-163 | REAL | — | |
| FlakySource::new | 202-221 | REAL | — | stable sort by offset; test L1078-1096 |
| FlakySource::poll | 237-379 | REAL | — | hand-checked all arms; saturating arithmetic L297/321 |
| FlakySource::applied_faults/is_ended/next_offset | 381-401 | REAL | — | counters asserted in every test |
| DeviceLossSimulator::new/poll | 451-493 | REAL | — | one event per poll; exact-time test L1135-1177 |
| DeviceLossSimulator::is_lost/emitted/remaining | 495-517 | REAL | — | |
| ClockJump::{forward,backward} | 537-555 | REAL | — | const ctors |
| ClockJumpInjector::new/raw_now_ns/offset_ns/fired_jumps/into_inner | 578-631 | REAL | — | |
| VirtualClock::advance/now_ns impl | 633-651 | REAL | — | i128 intermediate, saturate-at-0, test L1224-1242 |
| 17 tests | 737-1261 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| drop_removes_exactly_the_scripted_packet | YES | exact id vector + counter + sticky-end re-poll |
| delay_parks_exactly_v_ticks_from_pull_time | YES | wake-time equality incl. pull-time anchor (t=5→7ms) and ns-early poll |
| corrupt_rewrites_the_exact_offset_byte_only | YES | byte-exact 0xA5→0x5A arrays; OOB no-op not applied |
| truncate_ends_clean_at_the_packet_boundary | YES | next_offset==5 proves never-pulled |
| truncate_mid_delivers_partial_packet_then_dirty_end | YES | 4-byte payload + sticky dirty |
| cut_shortens_payload_and_the_stream_continues | YES | cut≠EOF distinction pinned |
| clean_truncate_never_pulls_and_outranks_same_offset_faults | YES | moot Drop+Corrupt consumed, next_offset==4 |
| dropped_partial_packet_still_ends_dirty | YES | applied==2 (mid-cut + drop both counted) |
| mixed_script_composes_and_replays_identically | YES | full timeline trace + determinism replay |
| delayed_packet_can_also_be_the_mid_truncated_one | YES | pending+dirty interaction |
| unsorted_script_is_sorted_at_construction | YES | sort proof |
| edge_scripts_are_tolerated_dirty_world | YES | empty source / zero-tick / OOB mid-cut |
| device_loss_schedule_fires_in_order_at_exact_times | YES | exact ns boundaries, no double fire |
| unpaired_recovery_is_tolerated | YES | no-panic dirty-world |
| clock_jumps_fire_at_exact_raw_times_and_warp_exactly | YES | non-monotonic readout pinned |
| clock_jump_backward_saturates_at_zero_and_forward_recovers | YES | clamp, zero-advance fires |
| injector_is_a_virtual_clock_drop_in | YES | generic-callee composition + into_inner |

All 17 passed in baseline (cargo-test-workspace.txt:6208-6217+; 17 counted).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "driver behind the MP §13 scenario rows … device loss" | faults.rs:1-4 | FALSE (no consumer) | rg sweep 0 hits |
| "allocation-free per packet by construction" | faults.rs:35 | REAL-UNPROVEN | proposed proof file absent (F-002) |
| "No unsafe, no dependencies" | faults.rs:52 | REAL | rg unsafe → only the doc line; crate std-only |

## 7. Dead / duplicated / unreachable code
The entire public surface (3 types, 11 public fns) is dead outside this file
(see F-001); within the file nothing is dead.

## 8. Dependencies & flags
crate::VirtualClock/StepClock only; std-only; no feature gates; #[must_use]
on every observability fn. No #[ignore] in this file.

## 9. Open questions for the next agent
- Does any W17+ wave land a consumer (recovery scenarios via FlakySource)?
- Is `tick_ns` (u64 ns) consistent with the VirtualClock ns convention? (Yes
  by units, but nothing cross-checks it outside tests.)
