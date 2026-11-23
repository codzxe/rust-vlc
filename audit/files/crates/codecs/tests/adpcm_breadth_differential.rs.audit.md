# AUDIT: crates/codecs/tests/adpcm_breadth_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 516 / 7 helpers + 4 tests + MD5 impl (3 fns) |
| File verdict | REAL (bit-exact differential; 4/8 ffmpeg reference MD5s independently reproduced by me with the sandbox ffmpeg) |
| Reachable from shipped entry point? | N/A (test binary) — the decoder under test (`AdpcmDecoder`) is CLI-reachable via the `adpcm_*` arm of apps/cli/src/pipeline_host.rs (~line 300, `self.seeded(track, config, AdpcmDecoder::new)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | ls fixtures (12/12 present); ffmpeg forced-decoder reproduction ×4 (all match); baseline cite |

## 1. One-paragraph verdict
A real, strong differential: 8 ADPCM flavor/channel cases decoded through the wired contract path, each pinned to an ffmpeg 7.1.5 PCM MD5, plus a shape guard, an RFC-1321-pinned in-file MD5, and a 1920-run fuzz-lite (junk/truncation/bit-flip × 8 flavors) asserting no panic and bounded output. All 4 tests pass in the baseline (:921-928). I independently reproduced 4 of the 8 reference MD5s (the forced-decoder oki/ws cases) with the sandbox's own ffmpeg using the exact oracle commands from the header — all four match byte-for-byte — which validates both the methodology and the pinned constants; the remaining 4 (yamaha/swf/qt) reference container lanes whose source files are not committed, so their MD5 provenance rests on the header's authoring record plus the passing baseline (UNVERIFIED beyond that). The only defect is a doc/comment mismatch in the fuzz test's stated contract.

## 2. What actually works (verified)
- `ffmpeg -c:a adpcm_ima_oki -f mulaw -ar 8000 -ac 1 -i oki_mono.vox -f md5 -` → `MD5=60714fbfc3997f823b7b97dd90e958ed` (== CASES line 93); oki_st → `bad7f93b285386ba34d2e30ec6d6e5f3` (== :101); ws_mono → `cbbfd8928ee6a5b2c019330d3d5ff7fb` (== :109); ws_st → `ec91fc42a66bee03edf43a5e91b80932` (== :117). 4/4 exact.
- Baseline cargo-test-workspace.txt:919-928: `breadth_corpus_is_bit_exact_against_ffmpeg_reference`, `corpus_geometry_matches_the_reference_shapes`, `md5_implementation_matches_rfc1321_vectors`, `fuzz_lite_never_panics_and_stays_bounded` — 4/4 ok (1.19s).
- Fixture inventory: all 12 committed files present (8 payloads + 4 sizes JSONs); sizes tile the payloads by assertion (:217-222).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/adpcm_breadth_differential.rs:316-320 vs 384-389
Code:
```
/// real corpus through every flavor — the decoder must answer
/// `Emitted`/`Recovered`/typed errors, never panic, and its output
/// must stay inside the documented geometry bound.
```
vs
```
                Err(err) => unreachable!("{descriptor} fuzz round {round}: {err}"),
```
What is wrong: the doc comment permits "typed errors" as an acceptable fuzz outcome, but the match arm makes any `Err` a test failure (`unreachable!`). The enforced contract (all damage must surface as `Emitted`/`Recovered`/`NeedMoreInput`, never a typed error) is STRICTER than documented — the comment understates the invariant the test pins. Misleading for maintainers deciding whether a new typed error breaks the suite (it does). Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_bytes | 124-129 | REAL | — | crate-local breadth dir, typed read error |
| parse_sizes | 133-147 | REAL | — | flat-array JSON, typed parse errors |
| raw_facts | 151-160 | REAL | — | 16-byte WAVEFORMATEX skeleton (tag 0, channels, rate, 4-bit) |
| Capture::accept | 169-177 | REAL | — | audio-only, unreachable! on video — acceptable in test |
| packet | 181-192 | REAL | — | |
| decode_case | 197-266 | REAL | — | sizes must tile payload; per-packet Emitted{1}; flush Drained; frame count == packets |
| breadth_corpus_is_bit_exact... | 268-288 | REAL | — | whole-instant sanity + MD5 per case |
| fixture / corpus_geometry... | 290-314 | REAL | — | 18×1024, 35×1024, 9×4101, 173×68, four 512s — all verified against `ls` sizes (11764=173×68 ✓, 36909=9×4101 ✓, 18432=18×1024 ✓, 35840=35×1024 ✓) |
| fuzz_lite_never_panics... | 321-404 | REAL | S4 | 240×8 runs, LCG, 3 damage classes; doc mismatch F-001; PCM bound payload×8+64 |
| md5_hex / md5 | 413-491 | REAL | — | RFC 1321; padding + LE length correct; K/S tables standard |
| md5_implementation_matches_rfc1321_vectors | 495-516 | REAL | — | 7 canonical vectors (I recognize all as the RFC set; test passes in baseline) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| breadth_corpus_is_bit_exact_against_ffmpeg_reference | YES | any decode deviation flips the MD5; 4/8 references independently reproduced via ffmpeg this session |
| corpus_geometry_matches_the_reference_shapes | YES | exact count×size equalities (verified vs ls) |
| fuzz_lite_never_panics_and_stays_bounded | YES | panics/unbounded output/bad instants all fail it |
| md5_implementation_matches_rfc1321_vectors | YES | protects the digest primitive the differential rests on |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact PCM MD5 on every flavor/channel path" vs ffmpeg 7.1.5 | :1-3 | REAL | baseline pass + my 4/8 independent ffmpeg reproduction |
| oki/ws reference MD5s (forced-decoder oracle) | :21-24 | REAL | reproduced exactly (commands in §2) |
| yamaha/swf/qt reference MD5s | :17-20 | REAL-UNPROVEN | pinned constants match decoder output (baseline), but the ffmpeg authoring containers are not committed — cannot re-derive without them |
| "lettered adpcm_oki 32k/24k variants have no ffmpeg 7.1.5 encoder or decoder — typed Unsupported gate is the honest posture" | :33-36 | REAL | `ffmpeg -encoders` grep: no adpcm_oki encoder (my run: "Unknown encoder 'adpcm_ima_oki'" when misused as encoder); decoder gate is in the module (other agent's file) |
| fuzz "must answer Emitted/Recovered/typed errors" | :318-319 | PARTLY FALSE | code forbids Err entirely (F-001) |

## 7. Dead / duplicated / unreachable code
None. The hand-rolled MD5 is deliberate (no dev-deps, documented at :406-410) and RFC-pinned.

## 8. Dependencies & flags
Test-target only; uses codec-api/media-types/codecs + workspace testkit. No features.

## 9. Open questions for the next agent
- adpcm module (decoder under test) is another agent's file: the fuzz contract "never a typed error on any junk" is unusually strict — confirm the module truly guarantees Recovered for all garbage (the passing baseline suggests yes).
- If the yamaha/swf/qt authoring containers can be regenerated (ffmpeg enc → NUT/FLV/MOV), the remaining 4 MD5 provenance chains would close.
