# AUDIT: crates/codecs/tests/aac_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 122 / 5 (Lcg, Outcome, classify, 2 tests) |
| File verdict | REAL (deterministic survival suite, executed; mutants demonstrably bite) |
| Reachable from shipped entry point? | N/A (integration test); runs in the codecs suite — I executed the prebuilt binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c --nocapture` -> 2 passed / 0 failed, 0.82s, tally "32 decoded, 48 typed refusals; codes: [aac.adts.param_switch, aac.frame.decode, aac.stream.no_frames]" |

## 1. One-paragraph verdict
A tight, honest fuzz-lite survival suite: 60 seeded byte-flip mutants (1 or 4 flips) plus 20 deterministic head/mid/tail truncations of the committed 2.5 KB AAC-LC fixture through the real `fuzz_aac_stream` harness, with the refusal codes re-derived through the public `decode_stream` surface and asserted to be stable `aac.*` codes; plus a sync-storm (8 KB of 0xFF) and garbage-interleave walk. Everything ran green in 0.82s with a real tally (32 decoded / 48 typed across three distinct codes — the mutants genuinely exercise both outcomes). Two nits: nothing asserts that refusals ever occur (a decoder that accepted every mutant would still pass, since `codes.iter().all(...)` is vacuously true on an empty list — only the eprintln records the tally), and the module doc calls the libFuzzer target "proposed" when `fuzz/fuzz_targets/fuzz_codec_aac.rs` exists and is registered in fuzz/Cargo.toml (bin 202) — only the nightly campaign wiring is pending.

## 2. What actually works (verified)
— `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c --nocapture` -> **2/2 pass**: `mutated_fixtures_never_panic_and_settle_typed` (80 mutants, tally printed, pristine-decode asserted) and `sync_storms_and_garbage_interleaves_never_panic` (8192×0xFF prefix + 512 suffix, 32 random byte inserts).
— Determinism: LCG seeds are fixed constants (`0x5A3_2026_1027`, `0xC0FF_EE01`); any failure reproduces bit-for-bit.
— Cross-checked the harness's honesty claims: `fuzz/fuzz_targets/fuzz_codec_aac.rs` is a real thin `fuzz_target!` shim over the same `fuzz_aac_stream`, registered in fuzz/Cargo.toml.

## 3. Findings
F-001 [S4]
Where: aac_fuzz_survival.rs:96-103
Code:
    // The honest invariants: nothing panics (the test reaching here
    // proves it), every refusal carries a stable aac.* code, and the
    // pristine base decodes.
    assert!(codes.iter().all(|c| c.starts_with("aac.")));
What is wrong: the "settle typed" half of the test's name is never enforced to occur — if every mutant decoded (codes empty), the `all()` passes vacuously. A regression that made the decoder swallow all damage would keep this green. The printed tally shows 48 refusals today, so the suite is informative in practice; a `assert!(!codes.is_empty())` (or `refused > 0`) would pin it. Confidence: HIGH — Verification: ran + read.
F-002 [S4]
Where: aac_fuzz_survival.rs:4-7
Code:
//! through `codecs::audio::aac::fuzz::fuzz_aac_stream` — the harness
//! that the proposed `fuzz/fuzz_targets/fuzz_codec_aac.rs` target
//! will wrap (wiring status honestly documented in the harness's
//! module docs).
What is wrong: "proposed … will wrap" is stale — the target file exists (a real `fuzz_target!` shim) and is registered in fuzz/Cargo.toml ([[bin]] name = "fuzz_codec_aac", line 202). What is actually pending is the nightly campaign wiring (the target's own header says so accurately). Confidence: HIGH — Verification: cat target file + rg Cargo.toml.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg | 23-35 | REAL | — | Knuth constants, reproducible |
| Outcome / classify | 37-53 | REAL | — | double-decode to recover the typed code; honest comment about the harness swallowing errors |
| mutated_fixtures_never_panic_and_settle_typed | 55-104 | REAL (ran) | S4/F-001 | 80 mutants, code-prefix + pristine asserts |
| sync_storms_and_garbage_interleaves_never_panic | 108-122 | REAL (ran) | — | survival-only by design (no asserts beyond not panicking) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_fixtures_never_panic_and_settle_typed | PARTIAL | panic/hang: YES (reaching the end proves it); non-aac.* codes: YES; pristine regression: YES; "damage always swallowed" regression: NO (F-001) |
| sync_storms_and_garbage_interleaves_never_panic | YES (survival only) | any panic in the walk fails the test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the contract is absence-of-panic/hang … never a crash" | aac_fuzz_survival.rs:9-11 | REAL | both tests ran; 80 mutants + storms survived |
| "the proposed fuzz/fuzz_targets/fuzz_codec_aac.rs target will wrap" | aac_fuzz_survival.rs:5-6 | STALE (S4/F-002) | target exists + registered; only nightly campaign pending |
| "committed bytes stay under the card's 200 KB budget" | aac_fuzz_survival.rs:11-13 | REAL | base fixture 2.5 KB; mutants in memory only |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
codecs aac fuzz harness, media-types; `mod aac` fixture helper (fixture_bytes). No features, no unsafe.

## 9. Open questions for the next agent
- codecs/src/audio/aac/fuzz.rs: the harness "swallows the typed error" (per classify's comment) — audit whether the fuzz harness's own counters (decoded/refused tallies it must keep) are exposed anywhere or lost; the survival test re-derives codes via a second full decode, implying the harness surface is lossy for diagnostics.
