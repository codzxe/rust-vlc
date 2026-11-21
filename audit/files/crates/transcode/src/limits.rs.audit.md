# AUDIT: crates/transcode/src/limits.rs (whole file, 264 lines)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 264 / 8 fns + 1 type + 10 consts + 4 tests |
| File verdict | UNREACHABLE (real, self-validating, test-pinned — the transcode crate has zero consumers) |
| Reachable from shipped entry point? | NO — same orphan-crate verdict as job.rs (no workspace member depends on `transcode`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "max_tracks\|max_filter_stages" crates/transcode/src/runner.rs crates/transcode/src/graph.rs` (enforcement points exist: runner.rs:359, graph.rs:318/324/353); `rg -n "slots\|bytes" crates/transcode/src/queue.rs` (two-axis capacity real: queue.rs:120-121); `rg -n "limit_zero\|limit_over_budget" crates/transcode/src/error.rs` (codes exist :79-95); `sed -n` on quoted ranges; baseline INDEX #3 (transcode tests green). |

## 1. One-paragraph verdict
A small, disciplined resource-envelope type: private fields, a single validating constructor (zero refused, ceilings refused, first-offender order pinned by test), defaults that are in-construction constants inside the ceilings, and a doc table whose every "enforced at" cell I verified against the enforcing code. The unit tests are honest and would catch regressions on all four boundaries. The only defects are contextual: five positional `usize` parameters invite transposition at the call site, and — decisively — the whole crate is dead code in the shipped product (no dependent crate, no CLI/tauri wiring, `product.toml:165`'s `transcode` key read by nothing).

## 2. What actually works (verified)
— Validation loop traced: :98-105 refuses zero first (`transcode.limits.zero`, error.rs:79-86) then over-ceiling (`transcode.limits.over-budget`, error.rs:88-96), in declaration order; test `over_ceiling_fields_are_refused_typed_first_offender` (:203-245) pins the order (slots before bytes), every axis individually, and the exact `ResourceLimit{budget, attempted}` payload. Green in baseline `cargo test --workspace` (INDEX #3, 2026-11-21).
— Enforcement claims verified by grep: `queue_slots`/`queue_bytes` → `PacketStageQueue::can_accept` (queue.rs:120-121, two axes); `event_slots` → `JobEventHub::new` (job.rs:233); `max_tracks` → runner.rs:359 (after `Demuxer::start`) and graph.rs:353 (MP4-remux configs); `max_filter_stages` → graph.rs:318 plus graph.rs:324 `filter_unimplemented()` for any non-empty filter list (the doc's "v1 refuses every one of them" is real).
— Defaults ≤ ceilings: 64≤1024, 1 MiB≤16 MiB, 32≤1024, 64≤4096, 4≤16; `exact_ceilings_are_legal` (:248-263) pins the boundary as inside the contract.

## 3. Findings

### F-001 [S4] Five positional `usize` parameters — transposition is silent
Where: crates/transcode/src/limits.rs:84-90
Code:
```
    pub fn new(
        queue_slots: usize,
        queue_bytes: usize,
        event_slots: usize,
        max_tracks: usize,
        max_filter_stages: usize,
    ) -> Result<JobLimits, EngineError> {
```
What is wrong: every parameter is the same type; `JobLimits::new(64, 32, 1024, 4, 64)` (bytes↔event-slots, tracks↔filters swapped) compiles and validates fine — the bug surfaces later as a queue that deadlocks at 32 bytes or a track table that refuses at 4.
Why it matters: the type system is this crate's whole safety story ("bounded memory is a contract"); the constructor undercuts it. The validation cannot catch it because every value is individually legal.
What correct looks like: a builder with named setters, or `JobLimits { queue_slots: 64, … }` plus a private `validate()` — i.e. make the wrong shape unexpressible rather than merely unlikely.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `MAX_*` (5) | 52-68 | REAL | — | match the module-doc table's ceiling column exactly |
| consts `DEFAULT_*` (5) | 72-76 | REAL | — | match the table's default column; all ≤ ceilings |
| `JobLimits::new` | 84-113 | REAL | S4 | F-001; validation complete, first-offender order as documented |
| accessors (5) | 118-148 | REAL | — | trivial `#[must_use]` |
| `Default` | 151-162 | REAL | — | in-construction constants, cannot fail (documented, true) |
| test `defaults_are_the_documented_posture` | 170-177 | REAL | — | pins the table |
| test `zero_fields_are_refused_typed` | 180-200 | REAL | — | all 5 axes + code/stage/kind asserted |
| test `over_ceiling_fields_are_refused_typed_first_offender` | 203-245 | REAL | — | order + payload + every axis |
| test `exact_ceilings_are_legal` | 248-263 | REAL | — | boundary inclusive |

## 5. Test honesty
All four tests assert exact values/codes/payloads and would fail under the mutations I reasoned through: dropping the zero check fails :180; reordering the axis array fails the first-offender assertion (:221-232); off-by-one on a ceiling (`>=` instead of `>`) fails :248 (1025 slots would refuse) — wait, `>=` would make the exact-ceilings test fail since it uses MAX_QUEUE_SLOTS=1024 — yes, caught. No gaps for this file's own surface.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| bounds table (5 axes: default/ceiling/enforced-at) | limits.rs:7-13 | REAL | constants match; enforcement verified at queue.rs:120, job.rs:233, runner.rs:359, graph.rs:318/353 |
| "Built only through `JobLimits::new`, which refuses zero values and values above the hard ceilings" | limits.rs:35-38 | REAL | private fields :41-47; loop :98-105 |
| "v1 refuses every [filter] request within [the ceiling] too" | limits.rs:66-67, :143-144 | REAL | graph.rs:324 `filter_unimplemented()` |
| "decoded frames are written through to the sink frame-by-frame (the WAV writer's only buffer is a reused conversion scratch of at most one frame)" | limits.rs:17-19 | UNVERIFIED | that is wav.rs's behavior — not this file; owner of wav.rs must confirm (cross-ref) |
| 7.E1 "bounded resources … AC: job model tests; no unbounded queues" | docs/13-agent-wave-program.md:733 | REAL-UNPROVEN + UNREACHABLE | tests green (baseline #3); crate has zero shipped consumers |

## 7. Dead / duplicated / unreachable code
Whole crate orphaned (see job.rs audit §7 for the full evidence: no dependent Cargo.toml, no CLI/taui wiring, `ui-dialog-convert` matrix row `not_started`/empty evidence, `product.toml:165 transcode = false` unread). `JobLimits` is re-exported at lib.rs:171 with no external consumer.

## 8. Dependencies & flags
`crate::error::{limit_zero, limit_over_budget}`, `media_types::EngineError`. No feature gates, no unsafe. Workspace build clean (baseline #1), tests green (baseline #3).

## 9. Open questions for the next agent
- wav.rs owner: the "reused conversion scratch of at most one frame" claim (limits.rs:17-19) is yours to prove or refute — it is this file's only unverified doc statement.
- If the crate is ever wired up (7.E3), F-001's constructor should be restructured first — config bugs there are undetectable until runtime.
