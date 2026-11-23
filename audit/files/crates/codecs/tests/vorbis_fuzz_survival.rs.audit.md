# AUDIT: crates/codecs/tests/vorbis_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 105 / 2 helpers + 1 test |
| File verdict | SHALLOW (the survival loops are real and deterministic, but the harness's contract-path packet slicing degenerates to a constant ~79-byte prefix for Ogg inputs, so most mutated audio bytes never reach the contract decode — the test's blast-radius claim overstates coverage) |
| Reachable from shipped entry point? | N/A (test binary; harness exercises `codecs::audio::vorbis` — CLI-reachable) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sed reads (test + harness cross-read fuzz.rs:80-116); ls fuzz targets; baseline cite |

## 1. One-paragraph verdict
The loops run and count honestly: 240 deterministic byte-flip mutants (60 × 4 committed fixtures, single+burst flips) + 80 truncations + 4 pristine passes through the real `fuzz_codec_vorbis` harness, with exact count assertions (baseline ok). The corpus lane (0085, +40 mutants) is properly conditional with a message (absent here). But cross-reading the harness exposes a coverage hole the test's own doc misstates: `fuzz_codec_vorbis`'s pass-3 contract path slices the packet as `data[..cut]` where `cut = data[0] % (len+1)` (fuzz.rs:89) — for every Ogg fixture the first byte is `'O'` (0x4F), so the packet is ALWAYS the first ~79 bytes, a partial identification page. A byte flipped at offset 2000+ (most of the file) is never fed through the contract decode at all; it reaches only pass 2's single `decode_block` attempt over the whole-buffer-as-one-packet. The test's comment "container bytes, header-chain bytes and packet bytes all fall in the blast radius — the harness routes them through whichever surface they parse as" is therefore false for the contract surface on mid-file mutations. The 0.00s baseline duration (vs mp3_fuzz_survival's 12.98s for the same pattern) corroborates the shallowness.

## 2. What actually works (verified)
- Baseline :1370-1376: `mutated_and_truncated_inputs_never_panic ... ok` (1 passed; 0.00s).
- Determinism: LCG with fixture-name-derived seeds; count assertions (`mutants == 240 + 40·lane`, `truncations == 80`) prove the loops ran — they would fail if the corpus lane misreported.
- All four committed fixtures exist (ls: 8422/3458/4431/7564 bytes).
- The libFuzzer shim exists and wires to the same harness (fuzz/fuzz_targets/fuzz_codec_vorbis.rs, `fuzz_target!` → `fuzz_codec_vorbis`) — the harness module's "proposed" wiring status is stale in the favorable direction.
- Harness internal hostile-input unit tests (fuzz.rs:131-174: empty, short prefixes, all-0xFF/all-0x00 4096, legal-prefix+0xFF tail) exist and passed in the baseline (in-crate tests).

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/vorbis_fuzz_survival.rs:49-52 vs crates/codecs/src/audio/vorbis/fuzz.rs:89,106
Code:
```
        // bytes, header-chain bytes and packet bytes all fall in the
        // blast radius — the harness routes them through whichever
        // surface they parse as.
```
vs (harness):
```
    let cut = usize::from(data.first().copied().unwrap_or(0)) % (data.len() + 1);
```
What is wrong: the claimed blast radius is not achieved. For Ogg inputs `data[0] == 'O'` (0x4F), so `cut` is a constant 79: the contract-path packet (pass 3, both clean and discontinuity) is always `data[..79]` — the Ogg page header plus the start of the identification header. Any mutation past byte 79 — which is nearly the whole file for a 3.4-8.4 KB fixture — never flows through the contract decode; it is exercised only by pass 2's one `decode_block` call over the entire buffer as a single packet (which parses one block at most, and only if the mutated setup header still validates). The test's doc comment asserts coverage the harness does not deliver. Why it matters: a panic lurking in the packet-decode path on mid-stream audio bytes would survive this suite; the fuzz suite's purpose (survival of mutated real streams) is only met at ~2% depth. What correct looks like: derive `cut` from a hash of multiple input bytes (or the LCG in this test passing an explicit cut), or feed the contract path the full mutated buffer as packet data. Confidence: HIGH — Verification: read both files; 0.00s baseline duration vs the mp3 sibling's 12.98s corroborates.

F-002 [S4]
Where: crates/codecs/tests/vorbis_fuzz_survival.rs:64-68
Code:
```
        for k in 0..20 {
            let at = (k + 1) * pristine.len() / 21;
            fuzz_codec_vorbis(&pristine[..at]);
        }
```
What is wrong: the truncation points are evenly spaced but the first is at ~4.8% of the file — every truncation retains the full header chain (Vorbis headers live in the first ~1-3 KB of these fixtures), so the survival suite never tests a truncated-HEADER input through passes 2/3 (a cut inside the setup header). Combined with F-001 the deep-tail/truncated-setup territory is untested. Confidence: HIGH — Verification: read + header sizes (first pages of an Ogg Vorbis file carry id+comment+setup).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg | 23-35 | REAL | — | deterministic; `below` panics on n=0 (never called with 0 here) |
| mutated_and_truncated_inputs_never_panic | 38-105 | REAL (loops) / SHALLOW (coverage) | S3/S4 | F-001/F-002; count asserts honest; corpus lane conditional |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_and_truncated_inputs_never_panic | YES for panics reachable at ≤79 bytes (contract path), header-chain bytes (pass 1), or one-block decode (pass 2); NO for panics deeper in packet decode (F-001) | survival-only assertion + count checks |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all three decode surfaces: header chain, free block, contract path" | :3-5 | TRUE (surfaces exist) but contract path sees only a 79-byte prefix for Ogg | fuzz.rs:89 + F-001 |
| "container bytes, header-chain bytes and packet bytes all fall in the blast radius" | :49-52 | FALSE for the contract surface | F-001 |
| "This suite's assertion is the survival itself" | :9-10 | TRUE | no outcome assertions, counts only |
| corpus 0085 lane "skip-with-msg otherwise" | :72-73 | TRUE | corpus absent → eprintln + count math adjusts |

## 7. Dead / duplicated / unreachable code
None intra-file. The corpus lane is environment-conditional by design.

## 8. Dependencies & flags
Test-target; depends on `codecs::audio::vorbis::fuzz::fuzz_codec_vorbis` (pub harness), tests/vorbis/mod.rs helpers (another file), tests/common. The libFuzzer campaign needs nightly wiring (fuzz/README) — honest status in the shim.

## 9. Open questions for the next agent
- src/audio/vorbis/fuzz.rs auditor (my F-001 evidence lives in that file): the same constant-cut weakness applies to the libFuzzer campaign when it runs — a fuzzer-found mid-buffer mutation still only reaches pass 2.
- vorbis_vlc_differential.rs (next in my queue is vorbis_bench; the vlc differential belongs to another agent): does it share the pending-VLC honesty of the flac one?
