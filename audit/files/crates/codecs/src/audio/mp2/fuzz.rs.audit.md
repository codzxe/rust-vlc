# AUDIT: crates/codecs/src/audio/mp2/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 190 / 8 (1 harness fn + 2 structs + 3 tests + docs) |
| File verdict | REAL |
| Reachable from shipped entry point? | Library/test surface by design (not on the playback path). Executed by: 3 in-file smoke tests (baseline PASS, orchrun log:392-393,419), `tests/mp2_fuzz_survival.rs` (3/3 PASS, log:1104-1107), and the libFuzzer shim `fuzz/fuzz_targets/fuzz_codec_mp2.rs` — which exists but has NEVER been run (no campaign evidence; see F-002). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `ls fuzz/` (Cargo.toml, fuzz_targets/, corpus/ all EXIST); `cat fuzz/fuzz_targets/fuzz_codec_mp2.rs` (shim calls this module's `fuzz_mp2_stream`); `rg -n "libfuzzer" fuzz/Cargo.toml` (:43 `libfuzzer-sys = "0.4.13"`); `rg -n "fuzz" docs/feature-matrix.toml` + `docs/formats/audio-mp3-mp2.md:96`; baseline INDEX #18 (`fuzz_registry_check.sh` PASS — consistency only, no execution). |

## 1. One-paragraph verdict
The harness is real and does what it documents: one pass drives all four surfaces (walk, geometry+seek, full `decode_stream`, contract decoder over up to 64 walked frames) on arbitrary bytes with errors swallowed by contract. The in-crate smoke tests and the committed-fixture/seeded-mutation survival suite passed in baseline — that is genuine, deterministic hostile-input coverage. Two honesty defects: the module doc's "wiring status" paragraph is stale (it claims the `fuzz/Cargo.toml` scaffolding and `libfuzzer-sys` dependency haven't landed — both exist in-tree), and the registered libFuzzer target has never been executed by anyone: the green fuzz-registry gate verifies name/file/README/harness-fn consistency plus non-empty corpora, i.e. it passes while proving nothing about actual fuzzing.

## 2. What actually works (verified)
- Baseline orchrun log:392-393,419: `a_layer_iii_frame_is_refused_not_decoded`, `a_header_only_buffer_is_survived`, `empty_and_garbage_inputs_never_panic` — all `ok`; log:1104-1107: `corpus_dirty_and_garbage_pools_survive`, `the_fuzz_harness_survives_the_same_mutations`, `mutated_fixtures_never_panic_and_settle_typed` — all `ok` (the survival suite drives this exact harness).
- Harness bounds re-derived: pass 4 caps at 64 frames (`take(64)`, line 130); every slice access via `data.get(..).unwrap_or(&[])`; `CountingSink` saturating; no unbounded allocation (pool bounded at 4 slots; output ≤ walked_frames×1152×2×4 as the doc claims).
- Shim wiring verified: `fuzz/fuzz_targets/fuzz_codec_mp2.rs` calls `codecs::audio::mp2::fuzz::fuzz_mp2_stream(data)` — the registry's harness-fn check is satisfiable.

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/audio/mp2/fuzz.rs:20-28
Code:
```rust
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exactly as doc 04 §8.5
//! registers it; building it needs the `libfuzzer-sys`
//! dev-dependency plus the `fuzz/Cargo.toml` scaffolding — both
//! Lead-owned dependency decisions (AGENTS.md §3.2, C5) — so until
//! they land the harness logic is proven by the in-crate
```
What is wrong: the scaffolding HAS landed — `fuzz/Cargo.toml` exists with `libfuzzer-sys = "0.4.13"` (fuzz/Cargo.toml:43), `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exists (shim present, calls this harness), and seed corpora exist under `fuzz/corpus/`. The "proposed … until they land" paragraph describes a pre-scaffolding tree.
Why it matters: a doc that says required infrastructure is absent when it is present misdirects the next maintainer (they would not know the shim is already wired), and it hides the real remaining gap — which is only the nightly campaign wiring (the shim's own comment at fuzz_codec_mp2.rs:9-12 states it correctly).
What correct looks like: update the paragraph to the shim's status: scaffolding landed, `cargo check` on stable works, campaign blocked on the nightly Lead decision (fuzz/README.md:21-23).
Confidence: HIGH — Verification: read + `ls fuzz/` + `cat` of the shim and Cargo.toml.

### F-002 [S3]
Where: crates/codecs/src/audio/mp2/fuzz.rs:1-2 (registry claim context) + baseline INDEX #18
Code:
```rust
//! The MP2 codec fuzz harness (card 5.J3 `+FB`; doc 04 §8.5 registry
//! name `fuzz_codec_mp2`): one call = one untrusted byte buffer
```
What is wrong: the registered fuzz target has never been executed. The only executed coverage of this harness is deterministic: 3 smoke tests and the survival suite's committed fixtures + seeded byte-flip mutants (`tests/mp2_fuzz_survival.rs`: "80 byte-flip mutants per fixture"). No libFuzzer/coverage-guided campaign has ever run against `fuzz_codec_mp2` — the campaign needs nightly, which is a pending Lead decision (fuzz/README.md:21-23), and no baseline run or log shows one. Meanwhile the gate that touches this surface, `scripts/fuzz_registry_check.sh` (baseline INDEX #18, PASS, "35 targets, diff empty"), verifies only registry-vs-implementation consistency (bins = files = README rows = harness fn exists) + non-empty seed corpora — it passes green while proving nothing about fuzzing ever having happened.
Why it matters: "35 fuzz targets" reads as a fuzzing capability that does not exist in executed form; the security posture of every parser in this repo rests on deterministic sweeps, not coverage-guided fuzzing, and nothing in the green gate distinguishes the two.
What correct looks like: the gate or README should carry an explicit "0 targets ever run; campaigns pending nightly wiring" line so the registry's 35 rows cannot be mistaken for executed coverage (the module and format docs are otherwise honest: `docs/formats/audio-mp3-mp2.md:96` says "fuzz-lite + family survival; mp2 340+ mutants" — accurate naming).
Confidence: HIGH — Verification: read of fuzz/README.md, the registry script's checks, and baseline INDEX #18; absence of any campaign run in `audit/runs/baseline/` (grep "fuzz" in INDEX).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc claims | 1-38 | REAL (stale wiring para) | S4/S3 | F-001 stale; F-002 campaign-never-run; bounds math checked (output ≤ frames×1152×2×4; frame_size ≥ 17 floor conservative — real LSF floor is 24). |
| `CountingSink` | 49-59 | REAL | — | Saturating count; swallows nothing (always Ok). |
| `FuzzOutcome` | 62-73 | REAL | — | 4-field digest, all asserted by tests. |
| `fuzz_mp2_stream` | 78-152 | REAL | — | 4 passes in dependency order; early typed-return on walk/geometry failure; `let _ =` swallows are the documented harness contract; contract pass bounded `take(64)`; no panic path found (all indexing via `get`); generation handling via `GenerationId::FIRST.next()`. |
| test `empty_and_garbage_inputs_never_panic` | 158-174 | REAL | — | Exact-outcome assert on empty; frames==0 on 0xFF×4096. |
| test `a_header_only_buffer_is_survived` | 176-181 | REAL | — | Header-without-body edge. |
| test `a_layer_iii_frame_is_refused_not_decoded` | 183-189 | REAL | — | Layer gate through the full harness. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 3 in-file smoke tests | Yes | Exact `FuzzOutcome` equality asserts; a walk that hallucinated frames or a decode that ran on garbage would flip fields. |
| Survival suite (external, drives this harness) | Yes | Seeded byte-flip mutants assert typed-settling and zero panics — mutation-sensitive by construction. |
| Gap | — | Nothing exercises the harness under memory/time pressure measurement (the "unbounded allocation" clause of the contract is unverified by any assertion — argued only in docs). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the harness logic is proven by the in-crate deterministic smoke tests here and by the committed-fixture + seeded-mutation survival run" (25-27) | module doc | REAL | Baseline passes (log:392-393,419,1104-1107). |
| "building it needs the `libfuzzer-sys` dev-dependency plus the `fuzz/Cargo.toml` scaffolding … until they land" (22-25) | module doc | FALSE (stale) | Both exist in-tree (F-001). |
| "one call = one untrusted byte buffer through every decode surface this module owns" (2-3) | module doc | REAL | Read of `fuzz_mp2_stream`: walk, geometry, both seek directions, single-header parse, `decode_stream`, contract path. |
| `fuzz/README.md` registry implies 35 live fuzz targets | registry | PAPER-ONLY (campaign) | Zero executed campaigns (F-002); gate #18 checks consistency only. |

## 7. Dead / duplicated / unreachable code
None in-file. The libFuzzer shim is "dead" only in the never-run sense (F-002), which is a campaign-status issue, not a code defect.

## 8. Dependencies & flags
Rides `codec-mp2` (default-on). The shim's `libfuzzer-sys` 0.4.13 lives in the separate `fuzz/` workspace (not a member of the main workspace — verified `rg -n "fuzz" Cargo.toml` → absent), so the shipped build carries none of it.

## 9. Open questions for the next agent
- Does any wave-8+ agent own `fuzz/README.md` or doc 04 §8.5 as a claim surface? F-002's "registry implies executed coverage" finding belongs there more than here.
- The survival suite's mutant count backs the "340+ mutants" claim in `docs/formats/audio-mp3-mp2.md:96` — verify the arithmetic when auditing that test file (80 flips × N fixtures ≥ 340?).
