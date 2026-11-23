# AUDIT: crates/subtitles/tests/srt_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 149 / 6 (1 helper + 1 rng impl, 4 tests) |
| File verdict | REAL (test) — genuinely runs 37 committed mutants + 2000 seeded mutants + ~6k truncation prefixes, all pass; but one documented invariant is NOT checked |
| Reachable from shipped entry point? | N/A (integration test) — runs in the cargo test gate; verified by direct execution |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | prebuilt integration binary `./target/debug/deps/srt_fuzz_survival-8c37ba1040decd00` -> 4 passed / 0 failed, 0.05s (exit 0); `ls fixtures/fuzz | wc -l` -> 37; `du -sb` -> 4803 bytes; rg MAX_DAMAGE_EVENTS -> 256; sed quote verification |

## 1. One-paragraph verdict
This is a real fuzz-survival suite, not theater: it executes the committed 37-file mutated corpus (count and 4803-byte size match the doc's "37 files / 4.7 KB" exactly), 2000 deterministic xorshift-mutants of two canonical parents (UTF-8 + UTF-16LE, 1-4 ops each: high-bit flips, random bytes, truncation, junk insertion incl. BOM/surrogate bytes), and every prefix of four canonical files — all through the real parse + render harness, 4/4 passing in 0.05s. The damage: the module docs promise a pinned invariant this suite never checks — "cue text ... no residual markup bytes (`<`/`>` only from literals, never from un-stripped tags)" — `invariants()` checks only the damage cap, interval validity, and cue-count sanity, and the harness's own contract is panic-only. A regression that leaks raw `<b>` into cue text sails through this AC.

## 2. What actually works (verified)
— verified by `./target/debug/deps/srt_fuzz_survival-8c37ba1040decd00` -> `test result: ok. 4 passed; 0 failed` (exit 0, 0.05s). Corpus size/count verified on disk: 37 files, 4803 bytes (doc: "37 files / 4.7 KB" — exact). Damage cap constant is 256 (srt/damage.rs:17). The cargo-fuzz target fuzz/fuzz_targets/fuzz_sub_srt.rs:19 exists and delegates to the same harness.

## 3. Findings
F-001 [S2]
Where: context — the crate under test is wired to nothing (apps/cli/Cargo.toml has no `subtitles` dep; only fuzz/Cargo.toml:50)
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50)
What is wrong: the survival proof protects a parser no shipped binary can invoke. The test is real; the product surface it guards is dead code (see tags.rs F-001). Confidence: HIGH — Verification: ran (rg).
F-002 [S3]
Where: crates/subtitles/tests/srt_fuzz_survival.rs:17-18 vs 31-52
Code:
```
//! * cue text is well-formed UTF-8 with no residual markup bytes
//!   (`<`/`>` only from literals, never from un-stripped tags).
```
vs the actual checks:
```
    for cue in &doc.cues {
        assert!(
            cue.interval.start_us() <= cue.interval.end_us(),
            "{name}: invalid interval in a survived cue"
        );
    }
```
What is wrong: no assertion anywhere in this suite inspects cue text for residual markup; the harness (`fuzz_sub_srt`, src/srt/fuzz.rs:40-46) explicitly swallows outcomes ("the harness contract is absence-of-panic ... nothing else"). Mutation reasoning: delete the markup-stripping from srt/tags.rs and this AC still passes green. The doc's "pinned invariants per run" list overstates what is pinned. What correct looks like: assert no `<`/`>` in Plain cue text except documented literal cases, or delete the bullet. Confidence: HIGH — Verification: read + ran (suite passes with no text inspection possible).
F-003 [S4]
Where: crates/subtitles/tests/srt_fuzz_survival.rs:84-96
Code:
```
/// xorshift64* — deterministic, std-only.
struct Lcg(u64);
```
What is wrong: the struct is named `Lcg` but implements xorshift64* (three shifts + a multiply — not a linear congruential generator). The comment is right; the name lies. src/srt/fuzz.rs's doc repeats the error ("seeded LCG mutation loop"). Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/subtitles/tests/srt_fuzz_survival.rs:7-13 vs 31-52
Code:
```
//! cues, never a crash, never a mid-file abort of the surviving
//! cues. Pinned invariants per run:
//!
//! * the damage ledger respects its cap (256) with the overflow
//!   *counted*, never stored;
```
What is wrong: "never a mid-file abort" is not asserted anywhere (no cue-count-vs-expectation checks), and the "overflow *counted*, never stored" half of bullet 1 is invisible to a `len() <= cap` check (a ledger that DROPS overflow events silently also passes). The assertions prove less than the contract prose claims. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| invariants | 31-52 | REAL | S3 | cap + interval + cue-count checks + harness; missing markup check (F-002) |
| committed_corpus_survives | 54-71 | REAL | — | >=30 files guard (37 actual), sorted, per-file invariants |
| canonical_parents_still_parse_exactly | 73-82 | REAL | — | is_clean + non-empty for out1/2/3 |
| Lcg (xorshift64*) | 85-96 | REAL | S4 | name lies (F-003); seed nonzero, deterministic |
| seeded_mutation_loop_survives | 98-137 | REAL | — | 2000 iters, UTF-16LE parent every 3rd, 1-4 ops, real junk table |
| truncations_never_beat_the_parser | 139-149 | REAL | — | every prefix of 4 parents incl. UTF-16BE |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_corpus_survives | YES (panic/cap/interval bugs) / NO (markup leakage) | runs real mutants; text never inspected (F-002) |
| canonical_parents_still_parse_exactly | YES | is_clean is a strict ledger assertion |
| seeded_mutation_loop_survives | YES (panic/hang/alloc) / NO (markup) | 2000 real mutants through parse+render |
| truncations_never_beat_the_parser | YES (panic on prefix) / NO (markup) | exhaustive prefixes, real parse |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "37 files / 4.7 KB — well under the 200 KB budget" | srt_fuzz_survival.rs:2-3 | REAL | ls+du: 37 files, 4803 bytes |
| "cue text ... no residual markup bytes" pinned per run | srt_fuzz_survival.rs:17-18 | FALSE | F-002: no such assertion exists |
| "damage ledger respects its cap (256) with the overflow *counted*" | srt_fuzz_survival.rs:12-13 | PARTIAL | cap length checked; counted-overflow not observable by this check |
| corpus "generated by a deterministic script (truncations at 16-byte strides, bit flips at 8/24-byte strides, BOM grafts, CRLF rewrites)" | srt_fuzz_survival.rs:20-23 | UNVERIFIED | generator script not in my assignment; fixture names (flip2_XXX, trunc2_XXX, bom_*, crlf2) are consistent with it |
| "all through the `subtitles::srt::fuzz` harness" | srt_fuzz_survival.rs:4-5 | REAL | invariants() calls fuzz_sub_srt per input (line 51) |
| srt/fuzz.rs "Until the Lead wires it" (no cargo-fuzz target) | src/srt/fuzz.rs wiring-status | STALE | fuzz/fuzz_targets/fuzz_sub_srt.rs:19 exists and delegates |

## 7. Dead / duplicated / unreachable code
None in-file. (The rng struct name is misleading, not dead.)

## 8. Dependencies & flags
Integration test of crate `subtitles`; dev-deps testkit (Must). Reads fixtures via CARGO_MANIFEST_DIR concat. No features, no unsafe. Runs on stable without cargo-fuzz — by design (harness doc), and the cargo-fuzz target now exists too.

## 9. Open questions for the next agent
- Where is the corpus generator script? If uncommitted, "reproducible offline" applies to RUNNING the corpus, not REGENERATING it — worth a doc tweak by the srt owner.
- srt/fuzz.rs render probe uses fixed TextMetrics::new(40, 20) and (1920, 1080) — one geometry only; styled-cue rendering (CueText::Styled) coverage inside place_plain_text is the srt/render.rs auditor's question.
