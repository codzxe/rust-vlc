# AUDIT: crates/codecs/src/audio/aac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-30 |
| Lines / functions | 94 / 1 (+1 test fn) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the module's exports are the shipped AAC lane: `AacDecoder`/`decode_stream` re-exported here are constructed by apps/cli/src/pipeline_host.rs:288-292 (`"aac" \| "aac-lc"`); descriptors minted by MP4 esds (apps/cli/src/containers/mp4.rs:232), TS stream_type 0x0F (ts.rs:146), and the ADTS lane (containers/adts.rs via demux::adts). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::aac` -> 17 passed (exit 0); `audio::aac::tests` -> 1 passed (exit 0); `./target/debug/deps/aac_bitexact-1e99e0008b8875c2` -> 2 passed (exit 0); `./target/debug/deps/aac_corpus_differential-7843d789557f43b7` -> 1 passed (exit 0; ffmpeg/corpus absent in sandbox → fixture/skip path); rg/sed verification (exit 0) |

## 1. One-paragraph verdict
The module doc is a mostly honest map of a real, working, reachable AAC-LC decoder: engine = symphonia-codec-aac 0.6.1 (Cargo.toml:109, exact pin; vendored src/lib.rs:9 `#![forbid(unsafe_code)]` verified), ADTS parse/walk and contract path real (17 lib tests + bitexact pass), the four typed partial gates (`aac.he_sbr`, `aac.profile`, `aac.channel_config_pce`, `aac.channel_count`) all exist at decoder.rs:159-198. Two claims fail scrutiny: the "implicit ADTS HE ... the differential records the degradation (`lumen_rate × 2 = ffmpeg_rate`)" mechanism does not exist anywhere in code — the differential hard-asserts equal output length and would fail such a stream loudly, not record it; and `is_adts_stream` is production-dead (demux's AdtsDemuxer carries its own probe). Neither breaks playback; both are doc/dead-code debt.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 audio::aac` -> `test result: ok. 17 passed; 0 failed` (adts parse/walk/resync, descriptor gates, ASC sniffing, channel/profile refusals, contract configure/decode/flush).
- `./target/debug/deps/aac_bitexact-1e99e0008b8875c2` -> `2 passed` (committed LC fixtures, ffmpeg-dumped references).
- `./target/debug/deps/aac_corpus_differential-7843d789557f43b7` -> `1 passed` — in this sandbox ffmpeg is absent, so the corpus row ran the documented SKIP-with-message path (honest skip; the always-on fixture rows inside aac_bitexact carry the coverage).
- Gate codes verified by rg: `aac.he_sbr` (decoder.rs:159), `aac.profile` (165,177), `aac.channel_config_pce` (191), `aac.channel_count` (198) — the mod.rs doc's taxonomy is accurate.
- Engine facts verified: Cargo.toml:109 `symphonia-codec-aac = { version = "=0.6.1", optional = true }` behind `codec-aac` (default set — release lane includes it); vendored crate lib.rs:9 `#![forbid(unsafe_code)]` matches the mod.rs claim.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/aac/mod.rs:27-30 (claim) vs crates/codecs/tests/aac_corpus_differential.rs:64-69
Code (mod.rs:27-30):
```
//! ADTS HE signaling is undetectable from the header (the SBR
//! extension hides behind full SCE/CPE syntax) — such streams decode
//! as **half-rate LC**, and the differential records the degradation
//! rather than silently passing it. SBR/PS is doc 04's self-build M,
```
and the differential's actual behavior (aac_corpus_differential.rs:64-69): `assert_eq!(bytes.len(), reference.len(), "{label}: window mismatch — Lumen {} bytes, ffmpeg {} bytes", ...)`.
What is wrong: the claimed "records the degradation (`lumen_rate × 2 = ffmpeg_rate`)" mechanism is unimplemented — `lumen_rate` appears only in doc comments (mod.rs + decoder.rs:50-53); no row, field, or assert in the differential handles a rate mismatch. An implicit-HE stream (Lumen half-rate) would trip the equal-length assert and fail the suite — loud, but "recorded degradation" it is not. Why it matters: the module sells a degradation ledger that does not exist; nobody has ever exercised the implicit-HE path. What correct looks like: either a row class that accepts `lumen.len() * 2 == ffmpeg.len()` and records both rates, or delete the parenthetical. Confidence: HIGH on the absence of the mechanism — Verification: read + rg. UNVERIFIED: whether any corpus file is implicit-HE (corpus + ffmpeg absent in sandbox; the two .aac rows' recorded rates 88200/96000 with equal windows and ≤1 LSB delta indicate true high-rate LC, not HE).
F-002 [S4]
Where: crates/codecs/src/audio/aac/mod.rs:72-79
Code:
```
pub fn is_adts_stream(data: &[u8]) -> bool {
    adts::parse_adts_header(data).is_ok()
}
```
What is wrong: production-dead. Its only callers are its own tests (rg over crates+apps+fuzz: definition + 5 test asserts, nothing else). The shipped ADTS lane probes through `demux::adts::AdtsDemuxer` (apps/cli/src/containers/adts.rs:40), which has its own syncword logic — a duplicate probe. The worklog (docs/worklog/archive-2026-10.md:8914) says 5.C2b "needs `is_adts_stream`... provided `pub`" — the consumer never materialized using it. What correct looks like: delete it or make demux's probe the single source. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| is_adts_stream | 77-79 | REAL but UNREACHABLE | S4 | correct logic (full header validation), zero production callers (F-002) |
| tests::probe_accepts_validated_headers_only | 86-93 | REAL test | — | mutation-sensitive (layer bit, short buffer, wrong magic) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_accepts_validated_headers_only | YES | 5 distinct accepts/rejects incl. the layer-bit trap |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| engine = purity-pinned symphonia-codec-aac 0.6.1 | mod.rs:15-17 | REAL | Cargo.toml:109; vendored lib.rs:9 forbid(unsafe_code) |
| LC-only, mono/stereo; 4 typed refusals | mod.rs:21-26 | REAL | decoder.rs:159-198 codes verified; 0094 6-ch row asserts `aac.channel_count` |
| implicit HE → half-rate LC, "differential records the degradation" | mod.rs:27-30 | FALSE (second half) | F-001: no such mechanism; equal-length assert would hard-fail |
| "module never touches crates/demux" | mod.rs:17-19 | REAL | no demux imports in aac/* |
| differential records MD5 + max sample delta per file | mod.rs:53-60 | REAL | Row struct fields (differential:47-49); aac_bitexact passes |

## 7. Dead / duplicated / unreachable code
- `is_adts_stream` (F-002) — duplicated by demux/src/adts's own probe.

## 8. Dependencies & flags
symphonia-codec-aac =0.6.1 + symphonia-core =0.6.1, optional behind `codec-aac` feature (in default set — shipped). No other externals. Submodules adts/decoder/error are `pub(crate)`; fuzz is `pub`.

## 9. Open questions for the next agent
- aac/decoder.rs and aac/adts.rs (the real decode machinery, ASC mode, gates) are separate assignments — F-001's mechanism claim is repeated at decoder.rs:50-53; whoever audits that file should cross-link.
- Is any shipped matrix row's evidence string the implicit-HE degradation claim? Doc-audit territory.
