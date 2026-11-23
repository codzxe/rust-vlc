# AUDIT: crates/demux/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 551 / 20 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only shared module (compiled into crates/demux/tests/* integration binaries); by design for test infra |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cargo test -p demux --test fixtures_parent sha256_impl` → 1 passed (exit 0); `cargo test -p demux --test wav_aiff_fixtures md5_impl` → 1 passed (exit 0); `sha256sum tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → 1f56202d…; python3 hashlib cross-check of RFC vectors (exit 0) |

## 1. One-paragraph verdict
Real, correct test infrastructure. Both hand-rolled hashes are proven by execution:
the test-side SHA-256 is pinned (fixtures_parent.rs:30-35) against the recorded
corpus digest, which I independently reproduced with GNU sha256sum (identical
1f56202d7b113d38…, size 425,516 == recorded 425_516), and the test-side MD5 is
pinned against RFC 1321 A.5 vectors (wav_aiff_fixtures.rs:160-167) that I
cross-checked with python3 hashlib — all three digests match. The expectation/1
parser handles the actual committed TOML format (verified against
tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml). All defects are
test-infra hygiene: the parser's doc claims failures are `unreachable!` while
parse_u64 silently defaults to 0; one dead return value; substring-prone field
extraction; ~90% duplicated hash scaffolding.

## 2. What actually works (verified)
- `cargo test -p demux --test fixtures_parent sha256_impl` -> 1 passed — test-side
  SHA-256 == recorded digest == GNU sha256sum of the real fixture.
- `cargo test -p demux --test wav_aiff_fixtures md5_impl` -> 1 passed — RFC 1321
  vectors ("", "abc", alphabet) == python3 hashlib values.
- parse_expectation on the committed avi_mjpeg_pcm.avi.toml: multi-line
  video_sample_sha256 array (60 digests), `_`-separated numerics (2_000_000,
  425_516), inline tables, [tracks]/[seek]/[essence_v0] sections — all shapes
  consumed correctly (fixtures_parent suite passes in baseline; parser traced
  line-by-line against the real file).
- open_via_input composes the real input::FileAccess (used by mkv_corpus,
  ts_corpus, corpus_mp4, mp4_fixtures, mkv_attachments_chapters,
  mkv_extras_wiring suites).

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/common/mod.rs:349-351 vs 494-498
Code:
```
/// Parse an `expectation/1` TOML (subset: the AVI corpus fields). Test
/// code only — a wrong shape is a broken *fixture*, so failures are
/// `unreachable!` (the testkit `must` vocabulary).
```
vs
```
fn parse_u64(value: &str) -> u64 {
    let cleaned: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    cleaned.parse().unwrap_or(0)
}
```
What is wrong: the doc says malformed input fails loudly via the must/unreachable!
vocabulary, but numeric parse failures silently become 0 (and non-digit characters
are silently stripped, so "4a25516" parses as 425516). A corrupted expectation
number becomes 0 or a de-digitized value instead of an unreachable! explosion.
Downstream asserts usually fail loudly anyway, so blast radius is a confusing
failure mode, not a false pass. What correct looks like: parse with must().
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/tests/common/mod.rs:461-479 and 391-402
Code:
```
/// Harvest every complete quoted string in `line`; returns
/// `(strings, remainder_after_last_open_quote_or_close)` where the
/// remainder tracks whether the array is still open.
```
What is wrong: the documented remainder (the ".1" of the tuple) is dead — the
caller takes `.0` and does its own open-array tracking via `while !acc.contains(']')`
(common/mod.rs:393). The docstring describes a contract nothing uses; a digest
string containing ']' inside the accumulated text would also truncate the harvest
latent-only (hex digests cannot contain ']'). Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/common/mod.rs:503-509 (and 513-516)
Code:
```
fn extract_string(value: &str, field: &str) -> Option<String> {
    let needle = format!("{field} = \"");
    let start = value.find(&needle)? + needle.len();
```
What is wrong: substring field matching — a value containing "bitrate = 44100"
would satisfy the needle for field "rate", and "video_codec = \"x\"" would satisfy
"codec = \"". The committed corpus format never trips this (fields are exactly
codec/width/height/fps/rate/channels), so it is latent fragility of a hand parser,
not a live bug. Confidence: HIGH — Verification: read + traced against the real TOML.

F-004 [S4]
Where: crates/demux/tests/common/mod.rs:69-110 vs 230-271
What is wrong: Sha256::update/finish and Md5::update/finish are ~90% identical
copy-paste (same buffering, same padding loop, only endianness and digest width
differ) — acknowledged std-only-by-hand design, but the scaffolding could be one
generic block. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sha256::new | 55-67 | REAL | — | FIPS 180-4 IV verified |
| Sha256::update | 69-93 | REAL | — | buffering traced; no partial-buffer path bug |
| Sha256::finish | 95-110 | REAL | — | pad-to-56 incl. wrap at 64 correct; bit_len precomputed |
| Sha256::compress | 112-159 | REAL | — | σ0/σ1/Σ0/Σ1/Ch/Maj verified; pinned by execution |
| sha256 | 164-168 | REAL | — | wrapper |
| sha256_hex | 172-179 | REAL | — | hex format matches corpus recording |
| Md5::new | 221-228 | REAL | — | RFC 1321 IV |
| Md5::update | 230-254 | REAL | — | dup of Sha256::update (F-004) |
| Md5::finish | 256-271 | REAL | — | little-endian length; correct |
| Md5::compress | 273-302 | REAL | — | F/G/H/I + g-index schedule verified; RFC vectors pass |
| md5_hex | 308-317 | REAL | — | ffmpeg -f md5 pin format |
| parse_expectation | 353-459 | REAL | S4 | silent-0 numerics (F-001); works on real corpus |
| harvest_strings | 465-479 | REAL | S4 | dead remainder (F-002) |
| split_key_value | 482-487 | REAL | — | ("","") fallback |
| parse_string | 490-492 | REAL | — | quote trim |
| parse_u64 | 495-498 | REAL | S4 | F-001 |
| extract_string | 503-509 | REAL | S4 | substring match (F-003) |
| extract_u32 | 513-526 | REAL | S4 | substring match (F-003) |
| extract_fps | 530-534 | REAL | — | composes extract_u32 |
| open_via_input | 546-550 | REAL | — | real FileAccess composition; must() on non-UTF8 path is test-only |

## 5. Test honesty
This file IS test plumbing; its own honesty: the sha256/md5 pinning tests are real
executed differentials against external truth (GNU sha256sum / RFC 1321 / hashlib
cross-checked by me). The suites consuming these helpers (fixtures_parent,
fixtures_class_b, adversarial_gaps, wav_aiff_fixtures, ps_fixtures, mkv/ts/mp4
corpora) assert concrete digests/counts/sizes — they would fail on regression.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the first test that uses it pins it against the committed corpus's recorded whole-file hash" | common/mod.rs:11-12 | REAL | sha256_impl_matches_the_recorded_file_digest executed; sha256sum cross-check |
| "pinned against the RFC's own A.5 vectors in the WAV/AIFF suite" | common/mod.rs:13-15 | REAL | md5_impl_matches_rfc1321_vectors executed; python3 hashlib match |
| "failures are unreachable!" (expectation parse) | common/mod.rs:349-351 | FALSE in detail | parse_u64 unwrap_or(0) silently defaults (F-001) |
| "the tests verify the recorded truth, never a transcribed copy" | common/mod.rs:17-18 | REAL | expectation TOMLs are generator-recorded (DO NOT EDIT header); parser reads them |

## 7. Dead / duplicated / unreachable code
- harvest_strings remainder return (F-002).
- #[allow(dead_code)] on every item — justified (suites include halves; verified:
  AVI suites use sha256/parse_expectation, WAV/AIFF uses md5_hex, corpora use
  open_via_input).

## 8. Dependencies & flags
No external deps: std + workspace media_types/plugin_api/testkit/input. Test-only
crate-external deps (input) via dev-dependency of the demux package.

## 9. Open questions for the next agent
- Who consumes [first_frame].hash_y and [audio_probe].first_samples_md5? The
  comment says "verified decoder-side by the codecs crate's AC suite" — codecs
  auditors should confirm those pins exist and run.
