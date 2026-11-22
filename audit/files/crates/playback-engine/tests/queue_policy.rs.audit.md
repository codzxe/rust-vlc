# AUDIT: crates/playback-engine/tests/queue_policy.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 353 / 6 tests + PRNG/helpers |
| File verdict | REAL — the queue-plan property suite: seeded accept/reject domain, exact boundaries, a schema tripwire against the real product.schema.json, lane totality, doc-03 defaults, threshold shape, error Display |
| Reachable from shipped entry point? | N/A (test binary). The subject (`QueuePlan::from_product_limits`) is the shipped plan constructor consumed by the product-config path |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep + verified product.schema.json:192-193 and product.toml:203-204 contents |

## 1. One-paragraph verdict

Six genuine tests, no tautologies. The property test drives
`from_product_limits` over 2 000 seeded pairs per seed and asserts the
full accept/reject decision table including the typed axis and exact
bound figures on every rejection (:62-138), with unreachable! arms
that fail if the decision disagrees with the ranges (:129-134) — a
clamp would be caught as an in-range reject. The boundary test hits
floor, ceiling, ±1, and the shipped product figures 1024/512
(:143-183). The constants test reads the real `product.schema.json`
and pins the engine's duplicated constants against it (:192-234) — I
verified the schema rows exist with exactly 64/8192 and 64/4096
(product.schema.json:192-193) and product.toml carries 1024/512
(:203-204). The lane test proves totality over `WiredKind` and the
typed `UnwiredKind` refusal for data (:242-277).

## 2. What actually works (verified)

— `from_product_limits_accepts_in_range_and_rejects_typed`: exhaustive
match over (result, audio_ok, video_ok) — every arm asserts payload
equality; `LUMEN_PROP_SEEDS` env allows extra seeds (honest fuzzing
hook).
— `schema_constants_pin_the_engine_copies`: std-only line-grep parse
of the schema; the fragility (one-line row shape) is documented as the
point (:188-191 "exactly the tripwire for that shape changing").
Verified against the real file: rows exist with the expected literals.
— `thresholds_make_packets_the_starvation_axis` pins the never-binding
byte/duration low marks and the ¾ high marks — the exact policy the
pipeline audit (F-004 family) discussed.
— Mutation reasoning (UNVERIFIED): clamping instead of rejecting fails
the unreachable! arm; a threshold inversion fails `stall < resume`; a
constant drift fails the schema pin.

## 3. Findings

### F-001 [S4] The schema tripwire parses the schema by line-shape, and its own copy of the ranges (:51-56) is a third duplication
Where: queue_policy.rs:51-56 and :198-220

Code:
```
fn schema_ranges() -> [(QueuePlanKind, u64, u64); 2] {
    [
        (QueuePlanKind::Audio, 64, 8192),
        (QueuePlanKind::Video, 64, 4096),
    ]
}
```
What is wrong: the boundary test's expected ranges are yet another
hand copy (schema → engine constants → test table). The schema pin
test guards the engine copy; nothing guards `schema_ranges()` itself
except reading it — if the schema changed, the pin test fails on the
engine constants while `schema_ranges` silently disagrees with the
boundary test's own expectations. Small, self-contained, but the
"must never drift" claim (:48-50) covers only two of the three copies.
What correct looks like: derive `schema_ranges()` from the engine
constants (the pinned pair) instead of re-listing literals.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `XorShift`/`seeds` | 23-46 | REAL | — | env-extensible seeds, `| 1` nonzeroing |
| `from_product_limits_accepts_in_range_and_rejects_typed` | 62-138 | REAL | — | full decision table, typed payloads, unreachable! on disagreement |
| `from_product_limits_accepts_the_exact_boundaries` | 143-183 | REAL | S4 | uses the third copy of the ranges (F-001) |
| `schema_constants_pin_the_engine_copies` | 192-234 | REAL | — | verified vs the real schema file |
| `for_kind_is_exhaustive_and_honest` | 242-277 | REAL | — | totality + UnwiredKind refusal + round-trips |
| `default_keeps_the_doc03_library_figures` | 282-305 | REAL | — | 200/300 packets, 1 s/2 s, subtitle 128/16 MiB/10 s |
| `thresholds_make_packets_the_starvation_axis` | 311-336 | REAL | — | never-binding low marks; ¾ highs |
| `queue_plan_errors_display_the_axis_and_bound` | 340-353 | REAL | — | Display names axis + bound |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all six | YES | exact payloads/boundaries; decision-table arms; real-file pin |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the schema is the authority; the engine's copies must never drift" | :6-7 | REAL | pin test + my file verification (product.schema.json:192-193) |
| "the shipped product figures (product.toml:199-200)" | :180 | STALE-LINE | product.toml now carries them at :203-204 (values 1024/512 correct) — the comment's line ref drifted |
| "every rejected input carries the typed axis and the violated bound" | :3-4 | REAL | decision-table assertions |

## 7. Dead / duplicated / unreachable code

`schema_ranges()` third copy (F-001).

## 8. Dependencies & flags

`playback_engine`, `packet_queue`, `media_types`, `testkit`. Reads
`product.schema.json` via `CARGO_MANIFEST_DIR` (committed file — no
network). No feature gates.

## 9. Open questions for the next agent

- Wave 11 (config): does anything validate product.toml's
  `queue_packets_*` against the schema before it reaches
  `from_product_limits` (or is the engine's typed rejection the only
  guard — which would make a bad product.toml an open-time failure
  rather than a config-load failure)?
