# AUDIT: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 411 / 12 (2 tests + 10 helpers) |
| File verdict | REAL (one false invariant: the DVB damage-cap assertion contradicts the decoder it tests — proven by execution) |
| Reachable from shipped entry point? | N/A (integration test; runs via `cargo test` — baseline audit/runs/baseline/cargo-test-workspace.txt:6060-6064 both tests ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rustc probe build+run (exit 0; output below); rg/sed quotes (exit 0); baseline cite |

## 1. One-paragraph verdict
A genuine deterministic fuzz-survival suite: in-code seed corpora (DVB display sets in 2-bit/8-bit/stream grammar, VobSub idx+sub with real PES/SPU framing), systematic truncation sweeps, and 400 LCG mutation rounds per seed asserting damage-cap, RGBA-stride, and panic-freedom through the real decoders and the real `fuzz_sub_dvb`/`fuzz_sub_vobsub` harnesses. I reproduced the seed grammar against the freshly built crate and confirmed the seeds DO compose regions/cues, so the stride invariants are not vacuous. But the DVB damage-cap invariant is written against a contract the decoder does not implement: the decoder APPENDS a `DamageOverflow` marker beyond the cap (len = 65), while the test asserts `len <= 64` and `marker ⇒ len == 64`. My probe proves an input exists (70 unknown-type segments) where the correct-running decoder violates the test's assertion — the suite survives only because 1200 mutation rounds never reach the cap. The VobSub half asserts the correct `cap + 1` contract for the identical design, proving the DVB half is the typo'd sibling.

## 2. What actually works (verified)
- Probe (`audit/runs/w09a01_probe_dvb.rs`, rustc-built against fresh rlibs) output, exit 0:
```
seed0-2bit: regions=1 damage=1 first_damage=Some("RleOverrun { region_id: 3, object_id: 9 }") stride_ok=true
seed1-8bit: regions=1 damage=0 first_damage=None stride_ok=true
seed2-stream: regions=1 damage=1 first_damage=Some("RleOverrun { region_id: 3, object_id: 9 }") stride_ok=true
flood: damage.len()=65 MAX=64 overflow_marker_present=true (test asserts len<=MAX)
```
  — clean seeds compose 1 region each (stride invariant non-vacuous); vobsub clean-pass asserts `cues.len() >= 2` in-test (L368-371).
- Baseline: `test dvb_fuzz_survival_mutations_and_truncations ... ok`, `test vobsub_fuzz_survival_mutations_and_truncations ... ok` (cargo-test-workspace.txt:6063-6064).
- Honest disclosure block (L22-31) about why no ffmpeg differential corpus exists — verifiable claim shape, no fake evidence strings.

## 3. Findings
F-001 [S3]
Where: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:14-15 (doc), 195-206 (assertions) vs crates/subtitles/src/dvb/mod.rs:374-380
Code:
```
//! * the DVB damage ledger respects its cap (64) with the overflow
//!   *counted*, never stored;
```
```
    assert!(
        update.damage.len() <= MAX_DAMAGE_EVENTS,
        "{name}: DVB damage ledger over cap ({} > {MAX_DAMAGE_EVENTS})",
        update.damage.len()
    );
    assert!(
        update.damage.iter().all(|d| {
            !matches!(d, DvbDamage::DamageOverflow { .. })
                || update.damage.len() == MAX_DAMAGE_EVENTS
        }),
        "{name}: DamageOverflow stored inside the cap"
    );
```
What is wrong: the decoder's actual semantics (dvb/mod.rs:374-380) are: store up to 64, count the rest, then APPEND `DamageOverflow{dropped}` — final len 65. The test doc claims "counted, never stored" (false — the marker IS stored) and the assertions demand len ≤ 64 and marker⇒len==64, both violated by correct decoder behavior on cap-hitting input. Why it matters: the first seed or mutation that actually exercises the overflow path fails the suite while the decoder works as designed — a false invariant that will burn the next contributor; worse, it documents the wrong cap contract. The vobsub half of THIS file asserts the correct `VS_MAX + 1` (L294), proving the inconsistency is in the DVB half. What correct looks like: `update.damage.len() <= MAX_DAMAGE_EVENTS + 1` + marker⇒len==MAX+1 (mirror vobsub), and fix the doc line. Confidence: HIGH — Verification: ran (probe `flood:` line above). Reachability of the bug by the current suite: latent (400×3 rounds never hit 65 — verified green in baseline, reasoning: seeds are ~150-500 B, damage-per-pass bounded well below 65).

F-002 [S4]
Where: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:134-135 (doc "realistic 2-bit display set ... the shape the mutator mangles") vs probe output
What is wrong: the "synthetic-but-valid" 2-bit seed is not valid: the clean pass emits `RleOverrun { region_id: 3, object_id: 9 }` damage (probe: seed0/seed2 `damage=1`), i.e. the un-mutated baseline already overruns the 24×12 region — the mutation sweep measures damage deltas from an already-dirty baseline. Harmless for panic-survival (the contract), misleading for the "valid" claim. Confidence: HIGH — Verification: ran (probe). Fix: widen the region or shorten the RLE lines so the clean pass is damage-free (seed1-8bit already is).

F-003 [S4]
Where: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:405-406
Code:
```
            let at = lcg.below(mi.len());
            mi[at] = b"0123456789abcdefx:,#\n\t\xff"[lcg.below(21)];
```
What is wrong: the literal is 23 bytes; `below(21)` indexes 0..=20 — `\t` and `\xff` are unreachable mutation characters (dead alphabet tail). Trivial coverage nit. Confidence: HIGH — Verification: read (byte count: 10 digits + 6 hex + `x:,#\n\t\xff` = 23).

F-004 [S4]
Where: crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:314-323
What is wrong: the DVB clean pass asserts nothing about composition (no `regions.len() >= 1`), unlike the vobsub clean pass (L368-371). If a decoder regression made every seed compose zero regions, this suite would stay green with a vacuous stride invariant. Today non-vacuous only because the seeds happen to compose (my probe). Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Lcg::{next_u32, below}` | 40-57 | REAL | — | Knuth LCG; `% n` bias irrelevant |
| `seg`/`page_seg`/`region_seg`/`clut_seg`/`object_seg` | 64-132 | REAL | — | correct EN 300 743 framing (be lengths, 0x0F sync) |
| `dvb_seed_2bit` | 136-161 | REAL | F-002 | composes 1 region but 1 RleOverrun on clean pass |
| `dvb_seed_8bit` | 165-180 | REAL | — | clean: 1 region, 0 damage (probe) |
| `dvb_seed_stream` | 184-190 | REAL | F-002 | crosses display sets |
| `dvb_invariants` | 192-215 | REAL (buggy invariant) | F-001, F-004 | runs decoder + fuzz harness |
| `vobsub_build_spu`/`wrap_pes`/`vobsub_seed_sub` | 221-289 | REAL | — | real SPU command sequence + PES framing; 0x100 filepos honored |
| `vobsub_invariants` | 291-306 | REAL | — | correct cap+1 contract |
| `dvb_fuzz_survival_mutations_and_truncations` | 314-360 | REAL | — | 3 seeds × (clean + truncation sweep + 400 mutation rounds); OOB-safe indexing verified by read |
| `vobsub_fuzz_survival_mutations_and_truncations` | 362-410 | REAL | F-003 | sub + idx mutation; clean assert ≥2 cues |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dvb_fuzz_survival_mutations_and_truncations | YES for panics/stride/cap-regressions below 64; NO for the overflow path (F-001: assertion false vs. design, path unreached); NO for zero-region regression (F-004) | invariants real but two blind spots, both proven |
| vobsub_fuzz_survival_mutations_and_truncations | YES | asserts clean ≥2 cues + cap+1 + stride; 400 sub-mutations + 400 idx-mutations + truncation sweeps |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "absence of panic/hang/unbounded allocation" contract | L10-12 | REAL | baseline ok + my probe runs |
| DVB "overflow *counted*, never stored" | L14-15 | FALSE | dvb/mod.rs:374-380 appends marker; probe flood len=65 |
| VobSub "Overflow event is appended" | L294 comment | REAL | vobsub/mod.rs:202 |
| "seed corpus is BUILT IN CODE (zero committed bytes)" | L6-7 | REAL | no fixture files for this suite (tests/fixtures/fuzz/ holds only .srt) |
| no-ffmpeg-corpus honesty note | L22-31 | REAL-UNPROVEN | ffmpeg refusal quote plausible; no ffmpeg run in sandbox (UNVERIFIED: no ffmpeg binary) |

## 7. Dead / duplicated / unreachable code
None dead. Duplication note: seed-grammar helpers duplicate the unit-suite grammar (per L60-61 "the same grammar the unit suite uses") — accepted test-scaffold duplication, self-contained by design.

## 8. Dependencies & flags
Test crate path-deps only (`subtitles`); no features. Runs on stable (no libfuzzer needed — the in-crate `fuzz` harnesses are the target bodies, per dvb/fuzz.rs wiring note).

## 9. Open questions for the next agent
- Who owns `src/dvb/mod.rs`: was the append-marker cap design (65) the intended contract, or should `DamageOverflow` replace the 64th slot? The test and the vobsub sibling disagree with the DVB assertions, not the code.
- The SPU 128 KiB cap claim (L19-20) is "re-asserted via the Ok shape" only — no direct assert in this suite; the vobsub unit suite presumably pins it.
