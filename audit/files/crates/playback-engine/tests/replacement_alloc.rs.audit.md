# AUDIT: crates/playback-engine/tests/replacement_alloc.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 502 / 1 test + marker harness |
| File verdict | REAL — the bounded-memory proof for source replacement: 51 opens / 50 switches with band/creep/churn metrics from the counting allocator, pinned constants with measured provenance, and anti-vacuity asserts |
| Reachable from shipped entry point? | N/A (test binary, one-test-per-binary allocator discipline) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

The "50 rapid switches with no leak" AC, measured. A **non-retaining**
sink (count + last marker only, :109-135 — the module doc explains why:
a retaining collector would attribute the test's own growth to the
engine) receives 51 epochs; after each switch a snapshot is taken; the
window (switches 5..=50, warm-up excluded) must satisfy: band ≤ 32 KiB
(steady-state oscillation — the dominant term is the A+V vs single-limb
queue preallocation ladder, explained :23-26), creep ≤ 1 KiB (the leak
detector: no monotone growth across 46 switches), churn ≤ 2048
allocation events per switch. The three bounds are pinned from
measurement with the provenance discipline stated ("re-measure and
re-pin honestly rather than weakening the assertion silently", :36-37),
and the suite asserts the soak actually ran: ≥ 51 frames, last marker
== 50, band > 0 (a flat-zero window means the soak did not run,
:501-502).

## 2. What actually works (verified)

— The marker harness is the replacement.rs design in non-retaining
form; `last_marker == 50` (:456-460) proves the final epoch's frames
were the last delivered (no stale tail).
— Creep is signed (`last − first`), so both growth AND shrink-then-
grow patterns are caught by the ≤ 1 KiB ceiling; a per-epoch retained
object (decoder cache entry, queue, bookkeeping) appears here.
— The churn metric separates allocation events (bounded per switch,
all freed) from retention — the honest decomposition of "no leak".
— Mutation reasoning (UNVERIFIED): a per-epoch retention of ≥ 1 KiB
cumulative fails creep; a queue-slot regression fails band; a
per-switch alloc regression fails churn.

## 3. Findings

### F-001 [S4] Pinned bounds are platform/allocator-shape dependent and the "re-pin honestly" instruction has no tripwire
Where: replacement_alloc.rs:79-91

Code:
```
const BOUND_BAND: isize = 32 * 1024;
```
What is wrong: the three constants carry measured maxima with ~2×
margin on "this suite's fakes" (:28-29) — different allocator
behavior (jemalloc vs system, 32- vs 64-bit slot sizes) could push a
legitimate build over a bound, and the only instruction is a comment
telling the developer to re-pin. There is no CI-distinguished
"measurement changed" signal (the assert failure message does include
the measured numbers, which is good).
What correct looks like: keep the asserts, but record the measured
values in a comment block with the measuring commit; or compute the
warm-up-derived expected band at runtime and pin only the margin.
Confidence: MEDIUM — reasoning; the constants themselves verified
present.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TallyConsumer` (non-retaining sink) | 109-135 | REAL | — | count + last marker only; the design's key honesty choice |
| marker fakes (MarkerDemuxer/Decoders/Provider) | 137-261 | REAL | — | content attribution, replacement.rs shape |
| builders + `switch_queues`/`switch_budget`/`Harness` | 262-427 | REAL | — | reused drain buffer (D-1) |
| `memory_stays_bounded_across_fifty_switches` | 431-502 | REAL | S4 | band/creep/churn with pinned bounds (F-001); anti-vacuity asserts |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `memory_stays_bounded_across_fifty_switches` | YES | any per-epoch retention fails creep; regression fails band/churn; vacuous run fails the three sanity asserts |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the AC's '50 rapid switches with no leak'" | :1-2 | REAL (per the test) | the three-metric window |
| "The three constants are pinned from measurement (D-6 …)" | :28-29 | REAL-UNPROVEN | the recorded measured values are described but not shown numerically in the file; UNVERIFIED (cannot run) |
| "a retaining collector would attribute the test's own growth" | :4-5 | REAL | TallyConsumer design |

## 7. Dead / duplicated / unreachable code

Marker fakes duplicated from replacement.rs in non-retaining form —
documented, purposeful.

## 8. Dependencies & flags

`playback_engine`, `commands`, `codec_api`, `media_clock`,
`media_types`, `packet_queue`, `plugin_api`, `testkit` (alloc_count).
No feature gates.

## 9. Open questions for the next agent

- Wave 16: `epoch_staleness_alloc.rs` is referenced (:13) but is NOT
  in my assignment list — whoever holds it, check the same pinned-
  bounds discipline.
