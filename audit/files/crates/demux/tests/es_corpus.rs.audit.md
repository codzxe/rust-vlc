# AUDIT: crates/demux/tests/es_corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 248 / 4 tests + 3 helpers |
| File verdict | REAL-UNPROVEN (genuine ffprobe differential; corpus absent here — all 4 tests are skip-noops in this environment) |
| Reachable from shipped entry point? | N/A (integration test). Demuxers under test are reachable: `lumen open` → probe (`AnnexbSignature`/`Mp3Signature`/`Mp2Signature`, signature/mod.rs:142-144) → containers/mod.rs:63-66 (`"h264"`/`"hevc"`/`"mp3"`/`"mp2"` arms). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `target/debug/deps/es_corpus-07e45cdf211543d9` → 4 passed in 0.01s (SKIP-noops, corpus absent); `LUMEN_CORPUS=/nonexistent …` → still 4 passed in 0.00s (per-file silent skips — contrast adts_corpus, which fails on a bogus override); corpus dir `/tmp/my-project/corpus-v4/media` verified absent |

## 1. One-paragraph verdict
Same env-gated design as adts_corpus.rs: real ffprobe-measured truths (H.264 packet geometry
pinned for ALL 26 packets, HEVC for the first 8 + count + whole-file byte coverage, MP3 frame
counts + exact sample-clock arithmetic, AAC counts cross-consistent with adts_corpus's TRUTHS),
with documented divergences (recovery-point SEI, ID3 attached-pic) pinned by assertions. But the
corpus is absent in this sandbox, so all four tests are green skip-noops here, and — worse than
its ADTS sibling — a bogus `LUMEN_CORPUS` value also skips silently (per-file `is_file()`
checks), so a typo'd override can never fail. The header also overstates "exact AU boundary
geometry for the two video lanes": the HEVC lane pins 8 of 56 sizes. Real code, environment-
conditional evidence, honestly labeled as skip-with-message but counted green regardless.

## 2. What actually works (verified)
- Skip behavior verified by execution: default run 4 passed / 0.01 s; corpus dir absent
  (demuxing two video lanes + 4 audio lanes with per-packet size asserts cannot run in 0.01 s).
- Assertion inventory (read): 26/26 H.264 packet sizes vs ffprobe's size column (117-123),
  `pts == Missing` + `duration == None` per packet (124-125), IDR-only keyframe policy with the
  packet-19 divergence pinned (130-132), start-code byte check (135), 8/56 HEVC sizes + 56 count
  + total == file_len contiguity (156-163), all-IRAP flags (164-167), MP3 576-tick ladder on a
  1/11025 base (192-198), AAC 346/376 counts matching adts_corpus's TRUTHS (229-241).
- Cross-suite corroboration: the AAC packet counts (346, 376) and file names are identical to
  adts_corpus.rs's TRUTHS — the two suites pin the same corpus instance independently.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/es_corpus.rs:52-58 (pattern at 99-107, 140-148, 172-175, 225-234)
Code:
```rust
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
        return Some(PathBuf::from(dir));
    }
    let default = PathBuf::from("/tmp/my-project/corpus-v4/media");
    default.is_dir().then_some(default)
}
```
What is wrong: environment-conditional evidence plus a silent override failure mode. (a) In any
environment without the corpus (this sandbox; baseline log carries no es-corpus output) all four
tests return early and count green. (b) Unlike adts_corpus.rs — which `assert!(path.is_file())`
once the dir is configured — this suite checks `is_file()` per lane and eprintln-skips, so
`LUMEN_CORPUS=/nonexistent` still yields 4 green passes (verified by execution). A typo'd
override silently converts the differential into a no-op. What correct looks like: assert on a
configured-but-missing corpus (the adts pattern). Confidence: HIGH — Verification: ran both.
F-002 [S4]
Where: crates/demux/tests/es_corpus.rs:9-11 vs 94-95
Code:
```rust
//! - **exact AU boundary geometry** for the two video lanes (packet
//!   sizes must equal ffprobe's measured `size` column — the walk is
//!   contiguous, so equal sizes ⇒ equal boundaries);
```
```rust
/// ffprobe measured packet sizes for `0044-raw-video.265` (56).
const HEVC_0044_SIZES_HEAD: [u32; 8] = [19745, 13800, 14114, 14191, 14292, 14034, 14743, 14929];
```
What is wrong: the header claims exact AU boundary geometry for BOTH video lanes; the HEVC lane
pins only the first 8 of 56 sizes (plus the packet count and whole-file byte total — which
constrains, but does not pin, the remaining 48 boundaries). Overstated claim; the H.264 lane
(26/26) does honor it. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/tests/es_corpus.rs:40-58 vs crates/demux/tests/common/mod.rs
Code:
```rust
/// Open a corpus file through the real access stack (card 2.A1):
/// the shared platform-aware `SourceUri::from_file_path` constructor
/// (POSIX outputs byte-exact; a non-local path is the constructor's
/// typed refusal, propagated — never a mis-built URI).
fn open_via_input(path: &Path) -> Result<Box<dyn SourceStream>, EngineError> {
```
What is wrong: this file has NO `mod common;` — it re-defines `open_via_input` (and `corpus_dir`)
as verbatim copies of the shared helpers in tests/common/mod.rs (identical doc comment text),
while sibling suites (adts_corpus.rs:44-46) import them. Copy-paste duplication that can drift.
Confidence: HIGH — Verification: read + diff of doc comments.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_via_input / corpus_dir | 44-58 | REAL | S4 | duplicated from common (F-003); skip source (F-001) |
| drain | 64-85 | REAL | — | packets+flags+terminal; Rec/Disc tolerated |
| H264_0019_SIZES / HEVC_0044_SIZES_HEAD | 89-95 | REAL-UNPROVEN | S4 | constants from an absent corpus instance; HEVC partial (F-002) |
| corpus_h264_lane_matches_ffprobe_packet_geometry | 97-136 | REAL-UNPROVEN | — | 26/26 sizes + timing vocabulary + IDR policy; not executed here |
| corpus_hevc_lane_matches_ffprobe_packet_geometry | 138-168 | REAL-UNPROVEN | — | 8/56 + count + total; not executed here |
| corpus_mp3_lanes_match_ffprobe_packet_counts | 170-221 | REAL-UNPROVEN | — | 79/6 counts, 576-tick ladder, ID3 skip; not executed here |
| corpus_aac_lanes_match_ffprobe_packet_counts | 223-248 | REAL-UNPROVEN | — | 346/376 (cross-consistent with adts_corpus); not executed here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 (corpus present) | YES | exact sizes/counts/flags/timing assertions (§2 inventory) |
| all 4 (corpus absent — THIS environment, incl. bogus LUMEN_CORPUS) | NO | early return, green regardless — F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exact AU boundary geometry for the two video lanes" | es_corpus.rs:9-11 | FALSE-for-HEVC | only 8/56 sizes pinned (F-002) |
| "absent corpus ⇒ skip-with-message" | es_corpus.rs:28-29 | REAL | executed: skip path taken (and it skips silently on bogus overrides too — F-001) |
| "resolution differential is cited, not re-run here" (layer rule) | es_corpus.rs:20-25 | REAL-UNPROVEN | codecs::tests::h264_parser_corpus existence not verified by me (next agent: confirm it is green and actually pins SPS vs ffprobe) |
| divergences (recovery-point SEI packet 19; 0145 attached-pic) | es_corpus.rs:14-18 | REAL-UNPROVEN | pinned in asserts at 131 and 213-214; not executed here |

## 7. Dead / duplicated / unreachable code
- The duplicated `open_via_input`/`corpus_dir` (F-003). Nothing else.

## 8. Dependencies & flags
- `demux::es::{annexb, mp3, aac}`, `input` (dev), `testkit`, `plugin_api`. Env: `LUMEN_CORPUS`
  (silently ineffective when wrong — F-001). No features.

## 9. Open questions for the next agent
- Verify `codecs::tests::h264_parser_corpus` exists, runs green, and actually pins SPS geometry
  vs ffprobe (this file leans on it for the 0019 resolution truth by layer rule).
- If the corpus is ever re-instantiated, re-derive H264_0019_SIZES / HEVC_0044_SIZES_HEAD and
  the MP3/AAC counts with the sandbox ffmpeg (7.1.5 present — `which ffmpeg` verified).
