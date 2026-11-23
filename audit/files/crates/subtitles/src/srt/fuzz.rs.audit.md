# AUDIT: crates/subtitles/src/srt/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 47 / 1 (no in-file tests; backed by tests/srt_fuzz_survival.rs) |
| File verdict | REAL (harness + claims verified end-to-end, tests executed); one stale doc claim; crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (crate-level). Chain: `fuzz/fuzz_targets/fuzz_sub_srt.rs` (registered fuzz/Cargo.toml:244-245) -> `fuzz_sub_srt` here -> `srt::parse` + `srt::render::place_plain_text`; stable evidence = tests/srt_fuzz_survival.rs (4 tests, executed this session). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p subtitles --test srt_fuzz_survival` -> 4 passed, 0 failed (exit 0); `cat fuzz/fuzz_targets/fuzz_sub_srt.rs`; `rg fuzz_sub_srt fuzz/Cargo.toml` (line 244-245); `rg MAX_PROBE_PIXELS` (render.rs:50 = 1<<22 px); `sed -n` quote verification. |

## 1. One-paragraph verdict
This is what the ASS harness should have been: parse + render over every surviving cue, backed by a genuinely strong survival suite — 37 committed corpus files, a 2000-iteration seeded xorshift64* mutation loop (1-4 ops/iteration: bit flips, byte overwrites, truncations, junk insertions), and every prefix of four canonical parents — which I ran green. Every doc claim checks out except the wiring status: the module says the cargo-fuzz target is unwired ("Until the Lead wires it"), but the target file exists, is registered, and has committed corpus seeds — the doc describes an outdated state.

## 2. What actually works (verified)
— `cargo test -p subtitles --test srt_fuzz_survival` -> 4/4 pass (exit 0): `committed_corpus_survives` (37 fixtures incl. my next three assignment files, per-file invariants: damage cap, valid intervals, cues ≤ bytes, harness run), `canonical_parents_still_parse_exactly`, `seeded_mutation_loop_survives` (2000 iterations, UTF-16LE parent every 3rd), `truncations_never_beat_the_parser` (all prefixes × 4 parents).
— Wiring + bounds claims verified: target file delegates to this function; `MAX_PROBE_PIXELS = 1 << 22` px (render.rs:50) × 4 B = 16 MiB — the doc's "16 MiB" (line 31) is exact; `MAX_DAMAGE_EVENTS` cap asserted by the suite (line 34).
— Harness body (lines 40-47) has no panic paths of its own: `parse` returns a document, `place_plain_text` per cue with results discarded, fixed `TextMetrics(40,20)` and extent `(1920,1080)`.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/srt/fuzz.rs:13-23
Code:
```rust
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_srt.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
//! decision (AGENTS.md §3.2, C5); the `fuzz/` tree is outside this
//! card's file scope. Until the Lead wires it, the harness logic is
//! proven on stable by the deterministic survival runs in
//! `crates/subtitles/tests/` (committed mutated/truncated corpus +
//! seeded LCG mutation loop).
```
What is wrong: the wiring it says is pending EXISTS: `fuzz/fuzz_targets/fuzz_sub_srt.rs` is present (body: `subtitles::srt::fuzz::fuzz_sub_srt(data);`), registered in fuzz/Cargo.toml:244-245, with committed corpus seeds (`fuzz/corpus/fuzz_sub_srt/seed_srt_flip2.srt`, `seed_srt_trunc2.srt`). "Until the Lead wires it" is stale — a reader auditing fuzz coverage would under-count the wired targets (nightly campaign build remains Lead-owned, which is the true remaining gap). The target file's own header carries the same stale paragraph.
What correct looks like: update the paragraph to "target committed at fuzz/fuzz_targets/fuzz_sub_srt.rs; the nightly campaign build is the remaining Lead-owned item".
Confidence: HIGH — Verification: read + `cat`/`rg` of the target, manifest, and corpus dir (outputs pasted in §2).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_sub_srt | 40-47 | REAL | - | parse + render-per-cue; single fixed geometry (1920x1080, 40x20 metrics) — scaling math only partially probed, acceptable for a survival harness |

## 5. Test honesty
No in-file tests. Backing suite (all ran green):
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_corpus_survives | YES | real asserts per file: cap, interval validity, cues ≤ bytes, harness run; ≥30-file count guard |
| canonical_parents_still_parse_exactly | YES | is_clean() + non-empty cues |
| seeded_mutation_loop_survives | YES | 2000 mutants × real invariants |
| truncations_never_beat_the_parser | YES | every prefix of 4 parents × invariants |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the render path (box + placement) over every surviving cue" | srt/fuzz.rs:6-7 | REAL | lines 44-46 loop all cues through place_plain_text |
| "proven on stable by ... committed mutated/truncated corpus + seeded LCG mutation loop" | srt/fuzz.rs:21-23 | REAL | tests/srt_fuzz_survival.rs ran green (4/4) |
| "damage ledger is capped (256 events)" | srt/fuzz.rs:27-28 | REAL | suite asserts `damages.len() <= MAX_DAMAGE_EVENTS` |
| "a transient probe bitmap of at most MAX_PROBE_PIXELS pixels (16 MiB)" | srt/fuzz.rs:30-31 | REAL | render.rs:50 `1 << 22` px × 4 B = 16 MiB exactly |
| "Until the Lead wires it" (target unwired) | srt/fuzz.rs:20-21 | FALSE (stale) | F-001: target + registration + corpus exist |

## 7. Dead / duplicated / unreachable code
None. (Crate-level unreachability as per segment.rs audit header.)

## 8. Dependencies & flags
Crate-internal `srt::parse` + `srt::render` only. No unsafe, no features.

## 9. Open questions for the next agent
- The corpus-generation script ("systematic truncations at 16-byte strides, bit flips at 8/24-byte strides" — suite doc lines 20-23) is NOT in-tree (UNVERIFIED: no script found in tests/fixtures/fuzz/); if it ever mattered for regenerating the corpus, it lives only in history. The fixtures themselves are committed, so reproducibility of the RUN is intact.
- srt/render.rs `place_plain_text` bounds (MAX_PROBE_PIXELS clamp path) — owner of render.rs should confirm the clamp is exercised by a fixed test, not only by the fuzz suite.
