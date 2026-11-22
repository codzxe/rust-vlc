# AUDIT: crates/packet-queue/src/limits.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 513 / 12 functions + 11 tests |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — `lumen` bin → run.rs open → apps/cli/src/host.rs:977-983 `queue_plan_from_product_limits` → playback-engine `QueuePlan` → `Pipeline` limb queues (BoundedQueue) → crates/packet-queue/src/queue.rs:401/443/462 `state.limits.binding_axis(...)` on every try-push/push-wait. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:4 |
| Commands run | `sed -n` quote verifications (limits.rs 30-36, 206-215, 237-252; queue.rs 392-470) — exit 0; `rg` enforcement-site search. No cargo (adaptation). |

## 1. One-paragraph verdict

The hostile question — "are caps enforced or advisory?" — answers **enforced**: every push path in queue.rs calls `binding_axis` and refuses with `PushOutcome::Full { item, binding }` (item handed back), and `push_wait` parks rather than bypassing. The type-level honesty is real (`NonZeroUsize` caps, `Duration::Unknown`/zero rejected typed). The one deliberate hole is the **single-item admission invariant**: an empty queue always admits one item regardless of weight, so the byte and duration caps are "cap + one item's weight", not hard bounds — a documented livelock fix (the measured 64 KiB / 2.73 s WAV packet vs the 1 s audio cap) that quietly converts doc 03 Q1's "whichever hits first refuses the next push" into a conditional claim. Plus a comment/code mismatch (the packets axis is summed in `usize` before widening, contrary to the overflow-safety claim), a `loop {}` hang-on-runtime-call in the const helper, and a `high ∧ low` watermark satisfiability edge.

## 2. What actually works (verified)

- Enforcement, not advice: queue.rs:401 `match state.limits.binding_axis(state.usage(), 1, size, duration)` → `PushOutcome::Full { item, binding }` (401-410); park-wait rechecks on every wake (443, 462). Read-verified end to end.
- Caps unconstructible as zero: `NonZeroUsize` fields + `QueueLimits::new` rejecting `Unknown`/zero duration typed (149-163) — pinned by `limits_reject_unknown_duration`, `limits_reject_zero_duration`.
- u128 axis checks for bytes/duration: `resident.bytes as u128 + add_bytes as u128` — wrap-proof for adversarial weights; pinned by `binding_axis_checks_in_u128_so_huge_weights_cannot_wrap` (usize::MAX/u64::MAX case).
- Watermark predicate (any-axis high, all-axis low) matches its doc; pinned by `watermark_flags_follow_the_predicate` including the neither-flag middle case.

## 3. Findings

### F-001 [S3] Single-item admission: the byte/duration caps are "cap + one item", not hard bounds
Where: crates/packet-queue/src/limits.rs:206-239
Code (234-239, verbatim):
```rust
        // The livelock invariant: a queue with nothing resident admits
        // one item unconditionally (see the method docs — the caps
        // bound accumulation, never a lone producer's admission).
        if resident == QueueUsage::EMPTY {
            return None;
        }
```
Code (206-212, verbatim):
```rust
    /// **Single-item admission (the livelock invariant, 6.FIX-pcm):**
    /// a queue with nothing resident always admits one item, whatever
    /// its weight. The caps bound *accumulation* — how much may pile
    /// up behind residents — never the admission of a lone item: a
    /// producer whose every item is heavier than a cap (measured: the
    /// wav demuxer's 64 KiB PCM packets carry 2.73 s at 8 kHz against
    /// the audio lane's 1 s duration cap) would otherwise be refused
    /// *into the empty queue* forever — a livelock that stalls the
```
What is wrong: nothing functionally — the trade is deliberate, measured, and documented. But the module's headline claim (limits.rs:3-5: "A queue is bounded on three axes at once … *whichever hits first* refuses the next push") is now conditional: the first item after any drain bypasses all three caps. Queue memory is bounded by `max_bytes + max(weight of one demuxed packet)`, and the duration axis likewise. A demuxer that emits one giant packet (file-controlled size) parks the queue above its byte cap until the consumer drains it — the cap no longer bounds the resident set at all times.
Why it matters: every consumer of `QueueLimits` that believes "max_bytes is the queue's memory bound" is wrong by one item; the W03 engine docs (session.rs queue-plan comments) make exactly that assumption when they talk about bounded memory soaks.
What correct looks like: an explicit `admit_oversize: bool` (or a per-item weight ceiling with a typed oversize outcome) so the carve-out is visible in the type, not in a method doc.
Confidence: HIGH — Verification: read (limits.rs + queue.rs push paths).
Severity: S3 — documented, deliberate, and the memory was allocated upstream regardless; but it contradicts the crate's own invariant Q1 wording.

### F-002 [S4] Packets-axis sum is computed in `usize` before widening — comment claims u128 safety
Where: crates/packet-queue/src/limits.rs:224-226 (comment) vs 240 (code)
Code (224-226, verbatim):
```rust
    /// The byte and duration sums are computed in `u128` so the
    /// *check* can never overflow even for adversarial weights; only a
    /// fitting item's weight is added to the resident sums.
```
Code (240, verbatim):
```rust
        let packets = (resident.packets + add_packets) as u128;
```
What is wrong: the packets addition happens in `usize` and is cast afterwards — the exact pattern the comment says cannot happen. Overflow needs `resident.packets + add_packets > usize::MAX`, unreachable while pushes add 1 packet each (`add_packets` is always 1 at the call sites, queue.rs:401/443/462), so this is a latent inconsistency, not a live bug.
What correct looks like: `resident.packets as u128 + add_packets as u128` — one edit restores the documented invariant.
Confidence: HIGH — Verification: read.

### F-003 [S4] `cap()`'s failure arm is an infinite loop if ever reached at runtime
Where: crates/packet-queue/src/limits.rs:30-36
Code (verbatim):
```rust
#[allow(clippy::empty_loop)]
const fn cap(literal: usize) -> NonZeroUsize {
    match NonZeroUsize::new(literal) {
        Some(n) => n,
        None => loop {},
    }
}
```
What is wrong: `const fn` is callable at runtime; a future runtime call `cap(0)` hangs the thread with no diagnostic instead of panicking. Today the only call sites are `const` items (40, 43, 47) where the const-evaluator turns the loop into a build failure, exactly as documented (15-25). Unreachable now; a hang is a worse failure mode than the `panic!` the lint suggested.
What correct looks like: `const_panic!`-style or keep the loop but make `cap` `const`-only by convention documented at the signature (it is, mostly) — acceptable as-is, recorded for the record.
Confidence: HIGH — Verification: read.

### F-004 [S4] `high` and `low` watermarks can both be true
Where: crates/packet-queue/src/limits.rs:323-331
Code (verbatim):
```rust
    pub fn evaluate(usage: QueueUsage, marks: &WatermarkThresholds) -> Watermark {
        let high = usage.packets >= marks.high.packets
            || usage.bytes >= marks.high.bytes
            || usage.duration.0 >= marks.high.duration.0;
        let low = usage.packets <= marks.low.packets
            && usage.bytes <= marks.low.bytes
            && usage.duration.0 <= marks.low.duration.0;
```
What is wrong: for a 1-packet cap (`from_limits` with max_packets=1: high.packets=1, low.packets=1) a single resident packet with small bytes/duration is both `high` and `low` — Primed and Underrun evidence at once. `WatermarkThresholds::new` is `pub` and accepts any thresholds, so custom configs can construct arbitrary overlap. The engine-side consumer resolves by signal priority (session.rs tick signal order: primed before starved), so behavior is defined, but the type advertises two booleans with no documented mutual-exclusion and no assert.
What correct looks like: document the overlap, or return a three-state enum (Low/Mid/High).
Confidence: HIGH — Verification: read + boundary arithmetic.

### F-005 [S4] Default thresholds are `[PROPOSAL]` numbers, not product config
Where: crates/packet-queue/src/limits.rs:283-303 — "the quarter is this crate's default; doc 03 assigns the real policy to product.toml" (286-287). The doc admits the policy is unclaimed; product.toml wiring does not exist for it. Recorded as paper-vs-code distance, not fraud (the admission is in the doc comment).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `cap` (const) | 30-36 | REAL | S4 | loop{} arm (F-003); const-only call sites |
| `LimitAxis` + Display | 50-68 | REAL | — | |
| `QueueUsage::EMPTY` | 86-93 | REAL | — | all-zero const |
| `QueueLimits::new` | 149-163 | REAL | — | typed rejection of Unknown/zero duration |
| `QueueLimits::audio_default` | 168-175 | REAL | — | 200 pkts / 1 s; bytes from caller |
| `QueueLimits::video_default` | 180-187 | REAL | — | 300 pkts / 2 s |
| `QueueLimits::subtitle_default` | 194-201 | REAL | — | 128 pkts / 10 s; `[PROPOSAL]` figures |
| `QueueLimits::binding_axis` | 227-252 | REAL | S3/S4 | single-item admission (F-001); usize-first packets sum (F-002); pub(crate), enforced by queue.rs |
| `WatermarkThresholds::new` | 278-281 | REAL | — | pub, accepts overlapping thresholds (F-004) |
| `WatermarkThresholds::from_limits` | 289-303 | REAL | S4 | quarter floor 1; `[PROPOSAL]` (F-005) |
| `Watermark::evaluate` | 323-331 | REAL | S4 | any-high/all-low; overlap possible (F-004) |
| tests (11) | 339-513 | REAL | — | see §5 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| limits_accept_known_positive_duration | YES (reasoned) | field equality |
| limits_reject_unknown_duration / _zero_duration | YES (reasoned) | typed error + message content |
| audio/video/subtitle_default_matches_doc03 | YES (reasoned) | exact figures (200/1s, 300/2s, 128/10s) |
| binding_axis_reports_the_first_violated_cap | YES (reasoned) | per-axis + fits case; note it also pins the EMPTY-admits-oversize behavior (line 437: EMPTY + weight 100 > bytes cap 10 → None) — the invariant F-001 is test-pinned, i.e. intentional |
| binding_axis_checks_in_u128_so_huge_weights_cannot_wrap | YES (reasoned) | usize::MAX/u64::MAX resident + add → Bytes (would wrap to a false "fits" on a 32-bit usize build for the packets axis — the bytes axis is the one actually protected) |
| watermark_default_high_is_capacity_low_is_a_quarter | YES (reasoned) | exact thresholds incl. quarter arithmetic |
| watermark_flags_follow_the_predicate | YES (reasoned) | all four predicate regions incl. neither-flag |
| config_error_is_a_std_error | WEAK (reasoned) | only checks non-empty Display — but it pins the trait impl, its whole purpose |

No tautologies; no mocks; none `#[ignore]`. Mutation run not possible (no cargo): reasoned, UNVERIFIED at runtime.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "whichever hits first refuses the next push (doc 03 §4.4(b), invariant Q1)" | limits.rs:3-5 | SHALLOW | enforced on every push (queue.rs:401+) BUT the empty-queue admission bypasses all axes (F-001) — the invariant holds only for queues with ≥1 resident |
| "an unbounded configuration cannot be expressed" | limits.rs:5-9 | REAL | NonZeroUsize + typed duration rejection read-verified |
| "the check can never overflow even for adversarial weights" | limits.rs:224-226 | FALSE for the packets axis | `(resident.packets + add_packets) as u128` sums in usize first (F-002); unreachable overflow, but the claim as written is wrong |
| Watermark high/low predicate | limits.rs:261-267 | REAL | evaluate() matches doc; overlap corner unannotated (F-004) |

## 7. Dead / duplicated / unreachable code

- None. `subtitle_default` has no production consumer yet (subtitle limb wiring is card 3.A3 — but the CLI's pipeline does construct subtitle queues via QueuePlan? UNVERIFIED; the QueuePlan construction in playback-engine/pipeline.rs is another agent's file) — flagged as open question instead.

## 8. Dependencies & flags

- Depends only on media-types time types. No feature gates. Linked into every playback build via playback-engine (W00: release PASS).

## 9. Open questions for the next agent

- Does any demuxer emit a packet whose byte weight exceeds the configured `max_bytes` in one item (making F-001's "+ one item" the dominant memory term)? Candidates: demux/src/wav (64 KiB packets, documented), MP4 sample sizes (wave 5).
- Is `subtitle_default` wired into the shipped QueuePlan, or unused vocabulary? File: crates/playback-engine/src/pipeline.rs (QueuePlan), apps/cli/src/demux_host.rs (queue_plan_from_product_limits).
- The 1024/512 packet caps the CLI passes (host.rs:977-983, "product posture") vs the 200/300 doc defaults here — which one actually runs, and does product.toml exist to back the "product limits" name? File: apps/cli/src/demux_host.rs.
