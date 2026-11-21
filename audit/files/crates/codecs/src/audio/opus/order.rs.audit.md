# AUDIT: crates/codecs/src/audio/opus/order.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 91 / 4 (1 fn + 3 tests + docs) |
| File verdict | REAL-UNPROVEN (5.1 row executed+verified via corpus differential in baseline; rows 3/4/5/7/8 shipped on documentation only — never semantically tested) |
| Reachable from shipped entry point? | YES — `vorbis_to_house` is called on the shipped Opus lane: `lumen open x.ogg/x.mkv` → Ogg/MKV demux (opus track) → CLI `pipeline_host.rs:276-282` seeded `OpusDecoder::new` → `decoder.rs:292-293` `if head.mapping_family == 1 { vorbis_to_house(head.channels) }`. `codec-opus` default-on. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "vorbis_to_house"` (single production call site decoder.rs:293, gated on mapping family 1); `rg -n "mapping_family\|family" opus/head.rs` (family-0 channel pin at parse time); baseline orchrun log:419-421 (3 in-file tests PASS) and :1151-1154 (`tests/opus_corpus_differential.rs … corpus_0077_surround_51 ... ok` — the 6-channel lane). |

## 1. One-paragraph verdict
The table's one corpus-verified row (5.1, `[0,2,1,5,3,4]`) is real — the baseline run executed the 0077 surround differential against ffmpeg-authored output and passed — and I independently re-derived every row against its documented Vorbis-source/house-target order: all eight are internally consistent permutations. But five of the eight rows (3/4/5/7/8) have never been semantically verified by anything in this repo — the module doc says so itself — and one of them (the 4-channel row) contradicts the doc's own convention anchor: "the WAV/WAVEFORMATEXTENSIBLE order" for 4 channels is quad FL FR BL BR, while the table targets FL FR FC BC. A wrong row is silent channel-swapped audio in shipped playback; today nothing would catch it.

## 2. What actually works (verified)
- Baseline orchrun log:1154: `corpus_0077_surround_51 ... ok` — the 6-row permutation verified end-to-end against real 6-channel Opus output (ffmpeg oracle), sample for sample.
- Baseline log:419-421: all 3 in-file tests pass.
- Every permutation re-derived by me against the documented orders (Vorbis RFC 7845 §5.1.1 source order → house target): 3: `[0,2,1]` ✓; 5: `[0,2,1,3,4]` ✓; 6: `[0,2,1,5,3,4]` ✓; 7: `[0,2,1,6,5,3,4]` ✓; 8: `[0,2,1,7,5,6,3,4]` ✓ — all match the stated semantics `out[i] = decoded[perm[i]]`.
- Call-site gating verified: family 1 only (decoder.rs:292-293); family 0 channel count pinned to 1–2 at head parse (head.rs:17), so the "family 0 is order-identity" claim is protected upstream.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/opus/order.rs:38-52
Code:
```rust
        // 3: Vorbis FL FC FR → house FL FR FC.
        3 => Some(&[0, 2, 1]),
        // 4: Vorbis FL FC FR BC → house FL FR FC BC.
        4 => Some(&[0, 2, 1, 3]),
```
What is wrong: the 3/4/5/7/8 rows are correctness-critical permutation data with zero semantic verification — the module doc admits it ("the 3/4/5/7/8 rows are the documented Vorbis↔WAV table entries … untested against a lane this corpus does not carry", lines 24-26), and the in-file tests only check bijection-structure, which any wrong-but-total permutation satisfies. Worse, the 4-row's target order contradicts the doc's own anchor: the doc defines house as "the WAV/WAVEFORMATEXTENSIBLE order" (lines 5-8), but the WAVEFORMATEXTENSIBLE 4-channel configuration is quad FL FR BL BR — not the FL FR FC BC the table targets (that is ffmpeg's `4.0` layout applied to Vorbis's FL FC FR BC source). The two conventions differ by which channels exist, not just their order, so "the same samples, a different interleave" (line 11-12) is not even true for the 4-row: house consumers expecting WAV-quad would relabel FC→BL and BC→BR.
Why it matters: a wrong or mis-anchored row is silent wrong-channel audio on shipped 3.0/quad/5.0/6.1/7.1 Opus playback — no error, no test failure, no fuzz catch (the fuzz harness checks panic-freedom, not channel identity). The 5.1 row only escaped this status because the corpus happened to carry a 0077 lane.
What correct looks like: (a) fix the doc — state that 4ch maps to the ffmpeg-4.0-style order, not WAV-quad, or fix the row to the WAV-quad convention the rest of the house uses; (b) add an ffmpeg-generated 3/4/5/7/8-channel fixture differential (the 0077 pattern, five more lanes) or at minimum a unit test deriving each row from named channel enums instead of raw indices.
Confidence: HIGH on the untestedness (doc-stated + test inventory), MEDIUM on the 4-row convention mismatch (WAVEFORMATEXTENSIBLE quad order asserted from the KS standard; ffmpeg's vorbis-4ch layout assignment not verifiable in this sandbox — no ffmpeg source, and the ruopus/symphonia sources are not vendored).

### F-002 [S4]
Where: crates/codecs/src/audio/opus/order.rs:32-55
Code:
```rust
pub(crate) fn vorbis_to_house(channels: u8) -> Option<&'static [u8]> {
    match channels {
```
What is wrong: the function's `None` return carries two unrelated meanings the doc conflates: "count has no defined row (pass through, honest)" — for family 1 a `None` at counts > 8 would be a spec violation arriving as silent passthrough (family 1 is 1..=8 by RFC 7845 §5.1.2), not an honest discrete case. Today unreachable if `head.rs`/`build_engine` refuse family-1 with >8 channels — neither gate is visible from this file.
Why it matters: the honest-passthrough contract only holds for family 255; a family-1 stream with 9 channels slipping through head parse would decode unpermuted with no typed complaint.
What correct looks like: make the family-1 call site treat `None` as a typed `opus.mapping.unsupported` (it knows the family), or document the invariant that family-1 channels are pre-clamped to 1..=8.
Confidence: MEDIUM — Verification: read; upstream clamping unverified (head.rs's family-1 channel bound not traced; decoder.rs is W07-A0x's file).
Note: kept S4 (not S3) because reachability requires a spec-violating head that two upstream layers would both have to accept.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-26 | REAL (honest on untestedness; inconsistent on the 4ch anchor) | — | Feeds F-001; the "same samples, different interleave" claim is false for the 4-row. |
| `vorbis_to_house` | 32-55 | REAL-UNPROVEN | S3/S4 | All 8 rows internally consistent (re-derived); 6-row corpus-verified; 5 rows semantically untested (F-001); `None` semantics conflation (F-002). |
| test `permutations_are_total_bijections` | 61-74 | REAL | — | Catches duplicate/out-of-range table entries; cannot catch swapped entries (still a bijection). |
| test `the_51_row_is_the_measured_vorbis_to_wav_permutation` | 76-83 | REAL | — | A regression pin (asserts the literal the comment derives) — genuine as a change-detector; the semantic truth is the 0077 corpus lane. |
| test `unknown_counts_have_no_row` | 85-90 | REAL | — | 0/9/255 → None pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `permutations_are_total_bijections` | Partially | Fails on non-total/duplicate rows; passes on any semantically wrong-but-total permutation. |
| `the_51_row_is_the_measured_vorbis_to_wav_permutation` | Yes (as a pin) | A mutation of the 6-row fails it; but it verifies the table against itself, not against an oracle — the oracle is the corpus differential (passes in baseline). |
| `unknown_counts_have_no_row` | Yes | Exact None asserts. |
| Gap | — | No test derives any row other than 6 from channel semantics — F-001's hole. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The 6-channel (5.1) row is corpus-verified (0077's differential asserts the mapped order matches ffmpeg's native output, sample for sample)" (22-24) | module doc | REAL | baseline log:1154 `corpus_0077_surround_51 ... ok`. |
| "the 3/4/5/7/8 rows are the documented Vorbis↔WAV table entries …, untested against a lane this corpus does not carry" (24-26) | module doc | REAL (honest) — and the defect it discloses is F-001. | Test inventory: no semantic test for those rows. |
| "house … is the WAV/WAVEFORMATEXTENSIBLE order" (5-8) | module doc | FALSE for the 4-row | WAV 4-channel config is quad FL FR BL BR; the table targets FL FR FC BC (F-001). |
| "family 0 (mono/stereo) is order-identical on both sides" (17-18) | module doc | REAL | RFC 7845 family 0 is 1–2 channels; head.rs:17 pins it at parse. |

## 7. Dead / duplicated / unreachable code
None — single purpose, single call site, consumed.

## 8. Dependencies & flags
Rides `codec-opus` (default-on; `ruopus` =0.1.2 pinned with `default-features = false, features = ["std"]` per the admission record's binding condition 1). This file itself is dependency-free table code. No unsafe.

## 9. Open questions for the next agent
- `opus/decoder.rs` (parts 1/2) + `opus/head.rs` agents: does anything refuse `mapping_family == 1` with `channels > 8` before the `vorbis_to_house` call (F-002's premise)? And does ruopus's MultistreamDecoder enforce it?
- Whoever owns the corpus: five more ffmpeg-generated surround lanes (3/4/5/7/8ch) would convert F-001 from "documented gap" to "verified" — the 0077 pattern already exists.
