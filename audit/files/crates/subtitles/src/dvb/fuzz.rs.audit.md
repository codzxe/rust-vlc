# AUDIT: crates/subtitles/src/dvb/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 75 / 1 prod (fuzz_sub_dvb) |
| File verdict | UNREACHABLE from shipped apps; REAL as a test-side harness (executed via the committed survival suite, 2/2 pass) |
| Reachable from shipped entry point? | NO — called only from `crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:33,214` (a test binary) and the not-yet-existing cargo-fuzz target; the `subtitles` crate's sole tree-wide dependent is `fuzz/Cargo.toml:50` (non-member) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/dvb_vobsub_fuzz_survival-bc4f65a1d8582a31` → `dvb_fuzz_survival_mutations_and_truncations ... ok`, `vobsub_fuzz_survival_mutations_and_truncations ... ok`, 2 passed 0 failed, 0.82s (exit 0); `ls fuzz/fuzz_targets/` → fuzz_sub_{srt,ass,vtt}.rs exist, no fuzz_sub_dvb/vobsub/pgs/cc; `rg 'fuzz_sub' fuzz/Cargo.toml` → 3 bins only |

## 1. One-paragraph verdict
The harness does what it says: sync-split at `0x0F 0x10`, per-chunk decode with increasing PTS, a truncation sweep on the single-chunk path, a reset + re-feed, and a final cue build — all outputs swallowed per the documented absence-of-panic contract, which I verified by executing the committed survival suite (2/2 pass, 0.82s, includes LCG-mutated and truncated corpora). The bounds arithmetic in the harness itself is safe (start ≤ len-2 invariant on the windows loop; `data[..cut]` cuts ≤ len). Two gaps: the cargo-fuzz target the doc registers (`fuzz/fuzz_targets/fuzz_sub_dvb.rs`) does not exist — srt/ass/vtt got theirs, dvb/vobsub/pgs/cc did not, so no coverage-guided campaign will ever reach this harness; and the claimed "cue render path" stops at cue CONSTRUCTION — `DvbCueBuilder::cues` never calls `place_cue`, so the placement arithmetic (where my layout.rs audit found real defects) is outside every fuzz surface.

## 2. What actually works (verified)
— verified by `./target/debug/deps/dvb_vobsub_fuzz_survival-bc4f65a1d8582a31` → `test result: ok. 2 passed; 0 failed` (both the DVB and VobSub survival passes; the committed corpus asserts stride/ledger invariants around each `fuzz_sub_dvb(payload)` call, tests/dvb_vobsub_fuzz_survival.rs:205-214).

## 3. Findings
F-001 [S4]
Where: fuzz.rs:20-28 (claim), fuzz/fuzz_targets/ + fuzz/Cargo.toml (reality)
Code: (verbatim)
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_dvb.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
```
What is wrong: the target does not exist and is not registered. `ls fuzz/fuzz_targets/` shows `fuzz_sub_srt.rs`, `fuzz_sub_ass.rs`, `fuzz_sub_vtt.rs` — the sibling pattern was executed for the text formats but not for DVB (nor VobSub/PGS/CC); `fuzz/Cargo.toml` registers only those three `fuzz_sub_*` bins. Doc 04's registry (docs/04-codec-format-roadmap.md:355) lists `fuzz_sub_dvb` as the DVB row's fuzz target, and the in-file doc is honest that it is unwired ("Until the Lead wires it") — but the result is that this harness is exercised ONLY by the deterministic in-crate survival runs (seeded LCG mutations, no coverage feedback, no campaign). A declared, tracked gap — hygiene severity.
Confidence: HIGH — Verification: ran (ls + rg, outputs above).
F-002 [S4]
Where: fuzz.rs:11-12, fuzz.rs:71-74; dvb/mod.rs:876-943
Code: (verbatim)
```
//! end-of-display-set composition, and the cue render path over
//! whatever survived. **The harness contract is
```
```
    // The render path over the LAST update whatever it was.
    let update = decoder.decode_pes(&[], None);
    let builder = DvbCueBuilder::new(update.display_extent);
    let _ = builder.cues(&update, Some(1_000_000), 0);
```
What is wrong: "the cue render path" is cue CONSTRUCTION: `DvbCueBuilder::cues` (dvb/mod.rs:876) only builds `CompositedCue` values via `layout_for` + `CompositedCue::new`; nothing calls `compositing::layout::place_cue` or `SubtitleCompositor::compose`. The placement/clamp/scale arithmetic — where my layout.rs audit found an unimplemented centering rule and an unguarded `reference` — is fuzzed by NO harness in the tree. The doc's "every decode surface" overstates the render coverage.
Confidence: HIGH — Verification: read (cues() body traced; no place_cue call).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_sub_dvb | 41-75 | REAL | S4 | bounds-safe splitter (start ≤ len-2); truncation sweep incl. len-0/1 degenerate cuts; executed via survival suite |

## 5. Test honesty
No tests inside this file; the harness IS the test subject of `dvb_fuzz_survival_mutations_and_truncations` (executed, passes, and asserts stride/ledger invariants around each call — it would fail on a panic or a broken invariant).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-PES pass through every decode surface" | fuzz.rs:4-11 | PARTLY FALSE | F-002: placement math never called |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:12-13 | REAL (panic/allocation) | survival suite passes; "hang" relies on the outer test timeout, no in-harness watchdog |
| "Until the Lead wires it, the harness logic is proven on stable by the deterministic survival runs" | fuzz.rs:25-28 | REAL | executed: 2/2 pass |
| "output ≤ O(input) + the region pixel-buffer cap" | fuzz.rs:32-33 | UNVERIFIED | would need allocation instrumentation; cap const exists in dvb/mod.rs |

## 7. Dead / duplicated / unreachable code
- None in-file. Context: doc 04:355 marks the DVB row status "missing" while a full decoder + this harness + survival tests exist in-tree — the doc UNDERSTATES implemented capability (matrix auditors' territory).

## 8. Dependencies & flags
- `crate::dvb::{DvbCueBuilder, DvbDecoder}` only; no unsafe; no features; harness allocates the chunk Vec (O(chunks)) + whatever the decoder caps.

## 9. Open questions for the next agent
- Whoever owns fuzz/: three of six subtitle formats have libFuzzer shims (srt/ass/vtt); dvb/vobsub/pgs/cc harnesses exist but are orphaned — is a Lead decision recorded anywhere for the missing four?
