# AUDIT: crates/codecs/src/audio/adpcm/ms.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 325 / 5 prod (header_bytes, ChannelState::expand, decode_ms_blocks, push_sample + tables) + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:298-302 `codec.starts_with("adpcm_") => self.seeded(track, config, AdpcmDecoder::new)` → adpcm/mod.rs:586 `ms::decode_ms_blocks(...)` → ms.rs:138 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | live differential: python3 transcription of this algorithm vs `ffmpeg 7.1.5` on tests/fixtures/adpcm_ms_mono.wav → BYTE-EXACT (MD5 `a95d2cbb66f65a4adcb36b57bc68e48e` = the suite's pinned digest); crafted loud-block probe vs ffmpeg → DIVERGENCE (F-001); `codecs-bae0f1756cc0a809 adpcm` → 55 passed; `adpcm_corpus_differential-b20f58c6c52dee82` → 2 passed; `adpcm_fuzz_survival-4bcf0cfa52427a86` → 4 passed; `adpcm_breadth_differential` → 4 passed |

## 1. One-paragraph verdict
REAL and genuinely bit-exact against live ffmpeg 7.1.5 — I verified it independently: a python transcription of `expand` + `decode_ms_blocks` reproduces ffmpeg's decode of the committed fixture byte-for-byte, and the fixture lane of the in-repo differential (pinned MD5) passes against the same ffmpeg version present in this sandbox. The block grammar (field-interleaved per-channel header, seed order, mono hi/lo packing) matches the reference. One proven defect: the delta ladder's upper clamp (`clamp(16, 32_767)`) diverges from ffmpeg on loud-but-legal content — ffmpeg lets `idelta` grow past 32767 (an `int` that wraps only past 2^31), while this code freezes it at 32767, producing audibly different samples; I proved the divergence with a crafted block decoded by live ffmpeg. The in-file comment justifying the clamp ("ffmpeg's `int` silently wraps there") is factually wrong at that magnitude, and the module-head claim "every packing decision pinned bit-exactly" is overstated for this regime.

## 2. What actually works (verified)
— verified by python3 model vs live ffmpeg: `match /256: True` on adpcm_ms_mono.wav (12216 samples, 6 blocks × 2036); `ffmpeg ... | md5sum` → `a95d2cbb66f65a4adcb36b57bc68e48e`, equal to the digest pinned in tests/adpcm_corpus_differential.rs:26.
— verified by `codecs-bae0f1756cc0a809 adpcm` → 55 passed (all 5 ms.rs unit tests green).
— damage model verified: out-of-table predictor skips the block counted (test ran); short block prefix-decodes with `concealed_tail`; empty input no-ops (test ran).

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/adpcm/ms.rs:104-105 (with the false justification at 98-103)
Code:
```rust
        let delta = (i64::from(self.delta) * i64::from(ADAPTATION[usize::from(nibble)])) / 256;
        self.delta = delta.clamp(16, 32_767) as i32;
```
What is wrong: the 32_767 upper clamp is NOT ffmpeg parity — it is a divergence on legal loud input. A legal header delta of 32767 plus one nibble-8 byte takes the ladder to 98301 in ffmpeg (its `int` wraps only past 2^31, 65,000× higher); this code pins it at 32767, so subsequent small-nibble predictors compute different values. Proven: crafted block (predictor 0, delta 32767, s1/s2 32767, nibbles `88 11 11`) → live ffmpeg decodes `32767,32767,-32768,-32768,32767,32767,32767,32767`; this algorithm produces `32767,32767,-32768,-32768,-1,29438,32767,32767` (python model, no upper clamp matches ffmpeg exactly). The justification comment — "ffmpeg's `int` silently wraps there; we bound. Clean streams never reach it" — is false: wrapping needs delta > 2^31 (≈21 consecutive ×3 steps), while the clamp bites at one step past 32767, and loud transients (full-scale clicks, saturated material) reach delta > 32767 routinely. The module head's "every packing decision pinned bit-exactly against the ffmpeg 7.1 reference" (ms.rs:4-5) is therefore overstated. Why it matters: loud streams decode with compressed dynamics relative to the reference decoder — the exact defect class the differential suite exists to catch, and it cannot, because no fixture/corpus row carries such content. What correct looks like: clamp at the i64/i32 boundary (panic-freedom) and keep exact parity below it, or document the divergence as a deliberate soft-limiter with its own test. Confidence: HIGH — Verification: ran (live ffmpeg probe).
F-002 [S3]
Where: crates/codecs/src/audio/adpcm/ms.rs:209-214 (stereo lane)
Code:
```rust
            _ => {
                for byte in nibble_bytes {
                    push_sample(out, states[0].expand(byte >> 4));
                    push_sample(out, states[1].expand(byte & 0x0F));
                }
```
What is wrong: not "wrong" per se — the hi=ch0/lo=ch1 stereo interleave matches the MS-ADPCM spec — but the stereo lane has NO executable verification in this environment: ms.rs's own tests are mono-only, the breadth differential covers yamaha/swf/qt/oki/ws stereo but NOT ms stereo (tests/adpcm_breadth_differential.rs CASES list, no `adpcm_ms` row), and the only stereo MS row (corpus 0102, pinned MD5 `a1de073a928ed1209422852ff5b99a28`) skips loudly because /tmp/my-project/corpus-v4 is absent. Stereo MS is REAL-UNPROVEN in this sandbox: it rests on a digest pinned in a past session. What correct looks like: commit a stereo MS fixture (the corpus row already proved the recipe — ffmpeg-encode, pin the digest). Confidence: HIGH on the coverage gap, UNVERIFIED on the stereo decode itself.
F-003 [S4]
Where: crates/codecs/src/audio/adpcm/ms.rs:155
Code:
```rust
        let mut states = Vec::with_capacity(channels);
```
What is wrong: a per-block heap allocation of the channel-state vec (≤64 entries, mod.rs:320 caps channels at 64) inside the block loop; blocks are ≥7·channels bytes and typically 512–1024, so this is a small constant alloc per ~1 KiB of input — measurable but bounded. The `out` vec is caller-owned and reused, so this is the only per-block allocation. Hygiene/perf nit. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `header_bytes` | 68-70 | REAL | — | 7·channels; channels ≤ 64 upstream |
| `ChannelState::expand` | 81-107 | REAL | S2 | /256 trunc = ffmpeg parity (verified live); delta clamp divergence F-001 |
| `decode_ms_blocks` | 138-219 | REAL | S3/S4 | bounds-checked, no panic; stereo lane unproven here (F-002); per-block states alloc (F-003) |
| `push_sample` | 224-226 | REAL | — | native-endian; honest doc note that LE is assumed by the differential |
| tables `STANDARD_COEFFICIENTS`/`ADAPTATION` | 48-61 | REAL | — | match the published pairs; ADAPTATION = the standard 230..768 table; pinned by test |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `standard_table_matches_the_published_pairs` (240) | YES | exact tuple equality |
| `mono_block_decodes_the_pinned_ladder` (257) | YES | full ladder vector [-226, 367, 239, -145, 863, 3278] — I re-derived every value by hand; correct |
| `predictor_out_of_table_skips_the_block_typed` (282) | YES | skipped count + empty output |
| `block_smaller_than_header_is_skipped_not_panicked` (296) | YES | includes block_align=0 empty-input no-hang |
| `negative_header_delta_heals_to_the_floor` (307) | YES | floor-16 semantics |
| Gap | — | no loud-delta test (F-001 invisible to the suite); no stereo test (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every packing decision pinned bit-exactly against the ffmpeg 7.1 reference on the real corpus" | ms.rs:4-5 | SHALLOW/overstated | true for the committed fixtures and (past-session) corpus 0102; false for loud-delta content (F-001, live-ffmpeg probe) |
| block grammar "as the corpus bytes actually carry it" (field-interleaved header, seed order, mono hi/lo) | ms.rs:7-24 | REAL | live ffmpeg byte-exact match on the fixture |
| "predictor = (samp1·coef1 + samp2·coef2) / 256 (trunc → 0)" | ms.rs:29 | REAL | verified against live ffmpeg 7.1.5 (python transcription byte-equal) |
| "delta = max(delta · ADAPTATION[nibble] / 256, 16)" | ms.rs:32 | PARTIAL | floor 16 = ffmpeg; the code ALSO clamps above at 32767 — undocumented in the formula, documented (wrongly justified) at 98-103 |
| "a predictor index above 6, or a block shorter than its own header, is block-skipping damage … never panics" | ms.rs:35-40 | REAL | unit tests ran; fuzz survival 4/4 |
| `MsFacts.coefficients` "the file's own coefficient table (7 pairs, validated equal to STANDARD_COEFFICIENTS range-wise)" | ms.rs:128-130 | misleading | mod.rs:407 refuses any file table ≠ STANDARD_COEFFICIENTS (exact equality, not "range-wise") — the "file's own" table is always the standard one; open question for the mod.rs owner |

## 7. Dead / duplicated / unreachable code
None. All functions consumed by adpcm/mod.rs (`TAG_WAVE_FORMAT` :351, `STANDARD_COEFFICIENTS` :407, `header_bytes` :417, `MsFacts` :429, `decode_ms_blocks` :586).

## 8. Dependencies & flags
In-house, zero external deps (unconditional module per audio/mod.rs:37). No unsafe. S16 native-endian output.

## 9. Open questions for the next agent
- adpcm/mod.rs owner: confirm the coefficient-table gate (mod.rs:407) refusing spec-legal custom tables is the intended behavior (ffmpeg decodes them with the file's table) — a capability divergence hiding behind a "range-wise" doc.
- Whoever owns the corpus: re-run the 0102 stereo row to re-prove the MS stereo lane (F-002).
