# AUDIT: crates/playback-engine/tests/queue_policy.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 353 / 12 (7 tests + 5 helpers) |
| File verdict | REAL (seeded-domain property suite + schema tripwire; 7/7 executed pass) |
| Reachable from shipped entry point? | N/A (test file) — the code under test (`QueuePlan::from_product_limits`, used by hosts to build `SessionConfig.queues`) is reachable: apps/cli constructs QueuePlans via playback-engine exports |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/queue_policy-6541a72ced5fd915` (prebuilt binary; cargo rebuild blocked — disk 100% full) → `7 passed; 0 failed` in 0.00s; `rg queue_packets product.schema.json` → lines 192–193; `rg queue_packets product.toml` → lines 203–204 |

## 1. One-paragraph verdict
A tight, genuinely adversarial config-validation suite: 4 seeded XorShift domains × 2000 pairs asserting that `QueuePlan::from_product_limits` accepts exactly the in-range inputs (with the documented derived byte/duration/subtitle/debounce figures and the 25%/75% threshold arithmetic) and rejects everything else with the typed axis, value, and exact violated bound — never a clamp. The boundary test pins floor/ceiling/±1 edges and the shipped product figures; the constants test parses `product.schema.json` itself (std-only line scan) and pins the engine's duplicated constants against it — a real drift tripwire, and I verified the schema rows exist in the assumed single-line shape (`"queue_packets_audio": { "type": "integer", "minimum": 64, "maximum": 8192 }`, product.schema.json:192). The lane test pins totality + the typed Data refusal. All 7 tests pass (executed via the prebuilt binary after the sandbox disk filled). Only defect: a stale comment line reference.

## 2. What actually works (verified)
- `./target/debug/deps/queue_policy-6541a72ced5fd915` → `test result: ok. 7 passed; 0 failed; 0 ignored` (executed).
- Schema tripwire verified against the real file: product.schema.json:192–193 carries both ranges on single lines matching the parser's assumptions; engine constants (QUEUE_PACKETS_FLOOR=64, ceilings 8192/4096) match — the passing `schema_constants_pin_the_engine_copies` proves the parse found the rows (its `unreachable!`s fire otherwise).
- Drift-closure reasoning (no edits): if `schema_ranges()` (the local copy) drifted up, `floor - 1` in `from_product_limits_accepts_the_exact_boundaries` would still be ≥ the real floor → the must-reject assert fails; if it drifted down, `ceiling + 1` stays in range → the must-reject assert fails. The local copy cannot silently drift.
- The seeded test's match is exhaustive over (result, audio_ok, video_ok): the two `unreachable!` arms prove accept/reject EXACTLY agrees with the independent range recomputation — a mutation of the range constants or the validation logic fails within 2000 pairs per seed.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/queue_policy.rs:178–180
Code:
```
    // The shipped product figures (product.toml:199-200).
    QueuePlan::from_product_limits(1024, 512)
        .unwrap_or_else(|e| unreachable!("the shipped figures must accept: {e}"));
```
What is wrong: the comment cites product.toml:199-200; the shipped figures actually live at product.toml:203-204 (`queue_packets_audio = 1024`, `queue_packets_video = 512` — verified). The VALUES asserted are correct; only the line reference is stale (comment drift after an edit above it). Confidence: HIGH — Verification: read + rg executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `XorShift::next` | 24–32 | REAL | — | repo-convention xorshift* |
| `seeds` | 34–44 | REAL | — | 4 fixed seeds + `LUMEN_PROP_SEEDS` env extension hook (documented; every failure prints its seed) |
| `schema_ranges` | 49–57 | REAL | — | local copy; drift-closed by the boundary + constants tests (reasoning in §2) |
| `from_product_limits_accepts_in_range_and_rejects_typed` | 63–137 | REAL | — | 2000 pairs/seed; accept figures (bytes 1/4 MiB, durations 1/2/10 s, subtitle 128, debounce) + typed reject payloads; pins audio-checked-first order (documented contract) |
| `from_product_limits_accepts_the_exact_boundaries` | 143–185 | REAL | — | floor/ceiling/±1 both axes + shipped figures; S4 comment drift F-001 |
| `schema_constants_pin_the_engine_copies` | 190–241 | REAL | — | std-only line scan of product.schema.json; verified shape + values; brittleness is the disclosed tripwire purpose |
| `for_kind_is_exhaustive_and_honest` | 248–283 | REAL | — | lane totality, distinct postures, typed Data refusal (UnwiredKind), round-trips |
| `default_keeps_the_doc03_library_figures` | 288–312 | REAL | — | 200/300/128 packets, byte/duration figures, `new` defaults for unspecified lanes |
| `thresholds_make_packets_the_starvation_axis` | 318–341 | REAL | — | low = 25% packets (min 1), high = 75%, byte/duration lows at caps (never binding), stall < resume |
| `queue_plan_errors_display_the_axis_and_bound` | 346–353 | REAL | — | Display names axis + bound; Box<dyn Error> non-empty |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 7 | YES | mutation reasoning: clamping instead of rejecting fails the typed-reject arms; a drifted constant fails the schema pin; wrong threshold arithmetic fails the exact 25%/75% equalities; a silent audio fallback for Data fails the UnwiredKind assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the schema is the authority; the engine's copies must never drift" | queue_policy.rs:6–8 | REAL | passing schema-pin test over the real file (executed); schema rows verified at product.schema.json:192–193 |
| "product.toml:199-200" (shipped figures location) | queue_policy.rs:178 | FALSE (stale line ref) | actual lines 203–204; values themselves correct (F-001) |
| "every rejected input carries the typed axis and the violated bound — never a clamp" | queue_policy.rs:5–7 | REAL | the seeded test's unreachable arms + typed payload asserts (executed) |

## 7. Dead / duplicated / unreachable code
- None. The `schema_ranges` duplication is drift-closed (§2).

## 8. Dependencies & flags
- Test-only; deps packet-queue, media-types, playback-engine, testkit. Std-only schema parse (no JSON dep) — deliberate, disclosed. No #[ignore]. NOTE (environment): cargo rebuild is currently blocked by a full disk (`No space left on device`, exit 28 during `cargo test -p playback-engine --test queue_policy`); I executed the prebuilt test binary directly instead — same tests, same results.

## 9. Open questions for the next agent
- `QueuePlan::from_product_limits`/`thresholds` implementation lives in playback-engine/src/limits.rs (not my assignment) — the pinned figures here are the contract side.
- Does any shipped host actually call `from_product_limits` with product.toml's values (config wiring), or are the product figures only aspirational? Check apps/cli + settings-schema wiring (host-side injection of `SessionConfig.queues`).
