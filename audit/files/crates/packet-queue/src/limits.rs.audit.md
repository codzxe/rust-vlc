# AUDIT: crates/packet-queue/src/limits.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 513 / 22 (10 prod items + 12 unit tests) |
| File verdict | REAL — typed three-axis boundedness, overflow-safe checks (one axis exception, see F-001), watermark predicates, all exercised by unit tests (baseline-passed; my own re-run blocked by disk-full) |
| Reachable from shipped entry point? | YES. `QueueLimits::audio_default`/`video_default` → crates/playback-engine/src/pipeline.rs:437-438 (the default `QueuePlan` every shipped session builds); `binding_axis` → queue.rs:401/443/462 (every push); `Watermark::evaluate` → queue.rs:294 (every watermark read). packet-queue is a production internal dep of playback-engine (boundaries.toml:296). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p packet-queue` -> BLOCKED: "No space left on device" (sandbox disk full — environment limit, not a repo defect); baseline cited instead: audit/runs/baseline/cargo-test-workspace.txt (packet_queue lib section): `17 passed; 0 failed` incl. all 12 limits tests + `oversized_item_admits_alone_and_bounds_accumulation` (backpressure section) |

## 1. One-paragraph verdict
This is solid boundedness engineering: three simultaneous caps (packets/bytes/duration) with whichever-hits-first refusal, `NonZeroUsize` making zero-caps unrepresentable, a validating constructor that rejects unknown/zero duration caps, u128 check arithmetic on the byte and duration axes, and a measured livelock fix (single-item admission for the empty queue — the wav 64 KiB PCM packet case, 6.FIX-pcm) that is honestly documented and tested. Two real defects: the packets axis adds in `usize` BEFORE casting to u128 (contradicting the module's own no-overflow claim — practically unreachable but a doc-code mismatch), and `subtitle_default` is a dead constructor no production code calls. My own test run was blocked by a full disk; the baseline shows all 12 limits tests plus the integration proofs passing.

## 2. What actually works (verified)
- verified by baseline audit/runs/baseline/cargo-test-workspace.txt (packet_queue unittests): all 12 limits::tests ok — typed rejections, doc-03 default figures, per-axis first-violated reporting, u128 no-wrap proof, watermark predicate matrix; and backpressure tests `oversized_item_admits_alone_and_bounds_accumulation`, `duration_cap_refuses_pushes` ok.
- verified by rg: audio_default/video_default consumed at pipeline.rs:437-438; binding_axis at queue.rs:401/443/462; Watermark::evaluate at queue.rs:294 — the shipped push path runs this code every packet.
- Hand-checked `cap()` const trick: a zero literal fails const evaluation at build time (loop-in-const never yields), replacing a runtime `unreachable!` — sound.

## 3. Findings
F-001 [S4]
Where: crates/packet-queue/src/limits.rs:224-226 vs 240
Code:
```
    /// The byte and duration sums are computed in `u128` so the
    /// *check* can never overflow even for adversarial weights; only a
    /// fitting item's weight is added to the resident sums.
```
vs
```
        let packets = (resident.packets + add_packets) as u128;
```
What is wrong: the packets axis adds in `usize` and casts AFTER; the bytes and duration axes cast before adding (`resident.bytes as u128 + add_bytes as u128`). A `resident.packets` near `usize::MAX` would wrap before the comparison and admit an over-cap item, contradicting the quoted invariant. Practically unreachable: `binding_axis` refusal keeps resident counts ≤ cap, and reaching ~2^64 resident packets is impossible on real hardware — but the doc claims total no-overflow and the code does not deliver it on this axis.
What correct looks like: `let packets = resident.packets as u128 + add_packets as u128;`
Confidence: HIGH — Verification: read + arithmetic checked.

F-002 [S3]
Where: crates/packet-queue/src/limits.rs:195-201 (+ Grep over crates/)
Code:
```
    pub fn subtitle_default(max_bytes: NonZeroUsize) -> QueueLimits {
        QueueLimits {
            max_packets: SUBTITLE_DEFAULT_PACKETS,
            max_bytes,
            max_duration: Nanoseconds(10_000_000_000),
        }
    }
```
What is wrong: `subtitle_default` is never called outside its own definition and unit test (Grep over crates/: only limits.rs:195/392/395). No production code wires a subtitle queue with these caps (the subtitle limb is unwired — see tracks.rs audit); the constructor is library-public dead code whose passing test suggests a capability that does not exist.
Confidence: HIGH — Verification: Grep sweep.

F-003 [S4]
Where: crates/packet-queue/src/limits.rs:283-303
Code:
```
    /// Default thresholds derived from the limits: `high` fires exactly
    /// at capacity (any axis at its cap), `low` at one quarter of each
    /// cap, floored at one unit so a cap of 1 still yields a usable low
```
What is wrong: with any cap of 1 (or caps 1..3), the quarter floor makes `low == high` on that axis, so a usage at capacity evaluates BOTH `high` and `low` true simultaneously (Watermark::evaluate, 323-331) — an ambiguous state every consumer must disambiguate. Deliberate and documented, but the ambiguity is real. Adjacent prior finding: audit/files/crates/playback-engine/src/pipeline.rs.audit.md flags the degenerate-limits primed-on-empty trap (directly-constructed `QueueLimits` with 0-ns duration) — the validating constructors here never mint those, shipped paths safe.
Confidence: HIGH — Verification: read + cross-cited prior audit.

F-004 [S4]
Where: crates/packet-queue/src/limits.rs:165-166, 286-287
Code:
```
    /// Doc 03 §4.4(b) audio-queue defaults `[PROPOSAL]`: 1.0 s /
    /// 200 packets, hard-capped in bytes by the memory budget the
```
What is wrong: every figure (200/300/128 packets, 1 s/2 s/10 s, quarter watermark) is a `[PROPOSAL]` constant not wired to product.toml — config surface promised, not delivered (tree-wide pattern).
Confidence: HIGH — Verification: read; no product.toml queue keys exist.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cap (const fn) | 30-36 | REAL | — | zero literal = const-eval build failure; scoped allow justified |
| AUDIO/VIDEO/SUBTITLE_DEFAULT_PACKETS | 38-47 | REAL | — | const-evaluated via cap |
| LimitAxis + Display | 50-68 | REAL | — | Packets/Bytes/Duration |
| QueueUsage + EMPTY | 70-93 | REAL | — | packets/bytes/duration resident sums |
| QueueLimits (struct) | 102-110 | REAL | — | pub fields (struct-literal misuse possible — see F-003 cross-ref) |
| QueueConfigError + Display + Error | 114-138 | REAL | — | UnknownDuration/ZeroDuration typed |
| QueueLimits::new | 149-163 | REAL | — | validates duration > 0 known; packet/byte zero unrepresentable |
| QueueLimits::audio_default | 168-175 | REAL | — | 200 pkts/1 s; production default (pipeline.rs:437) |
| QueueLimits::video_default | 180-187 | REAL | — | 300 pkts/2 s; production default (pipeline.rs:438) |
| QueueLimits::subtitle_default | 194-201 | DEAD | S3 | F-002 — no production caller |
| QueueLimits::binding_axis | 227-252 | REAL | S4 | single-item admission (livelock fix) + u128 checks; F-001 packets-axis cast order |
| WatermarkThresholds::new | 278-281 | REAL | — | |
| WatermarkThresholds::from_limits | 288-303 | REAL | S4 | high=capacity, low=quarter floored at 1 (F-003 overlap) |
| Watermark + evaluate | 310-331 | REAL | — | high eager (any axis), low conservative (all axes) — matches doc |
| tests (12) | 334-513 | REAL | — | constructor rejections, default figures, per-axis violation order, u128 wrap proof (bytes/duration), watermark matrix |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| limits_accept/reject_* (3) | YES | typed error variants asserted exactly |
| audio/video/subtitle_default_matches (3) | YES | figures pinned to doc 03 |
| binding_axis_reports_the_first_violated_cap | YES | all three axes + fit case |
| binding_axis_checks_in_u128_so_huge_weights_cannot_wrap | YES (bytes/duration) | usize::MAX/u64::MAX weights — but does NOT cover the packets-axis wrap (F-001) |
| watermark_default_high_is_capacity_low_is_a_quarter / flags_follow_the_predicate | YES | threshold derivation + all four predicate quadrants |
| config_error_is_a_std_error | YES (trivially) | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A queue is bounded on three axes at once — whichever hits first refuses" | limits.rs:3-5 | REAL | binding_axis order Packets→Bytes→Duration; backpressure tests (baseline) |
| "An unbounded queue is unconstructible" | limits.rs:98-99 | REAL | NonZeroUsize + validating new(); caveat: pub fields allow struct-literal bypass of the duration check (prior pipeline.rs audit finding) |
| "the *check* can never overflow even for adversarial weights" | limits.rs:224-226 | FALSE (packets axis) | F-001: usize add before cast |
| single-item admission livelock fix | limits.rs:206-222 | REAL | baseline backpressure `oversized_item_admits_alone_and_bounds_accumulation` ok |

## 7. Dead / duplicated / unreachable code
- `subtitle_default` (F-002): dead constructor, test-only consumer.

## 8. Dependencies & flags
- Deps: media-types (Duration/Nanoseconds) only + testkit (dev). No features. Shipped in every build (packet-queue is a production dep of playback-engine).

## 9. Open questions for the next agent
- queue.rs (the BoundedQueue itself, push/pop/close paths that call binding_axis) is in another agent's domain — the single-item admission means a queue can exceed a cap by one item's weight; the memory-accounting tests (bounded_alloc.rs) should be checked against that +1 headroom.
- Disk-full blocked my re-run (`No space left on device`, target/ incremental dir) — environment limit recorded; baseline cited.
