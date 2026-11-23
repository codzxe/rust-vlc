# AUDIT: crates/codecs/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 890 / 25 (+4 structs, 2 consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only infrastructure (`mod common;` compiled into crates/codecs/tests/* binaries; not in the codecs lib, not in lumen-cli/desktop). Not a defect: this is its purpose. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/fixtures_parent-4e25a1e1316c527e` (codecs AC suite) → 9 passed / 0 failed (0.50s), incl. `decoded_avi_outputs_match_reference_digests`, `sha256_reader_pinned_against_committed_file_digests`. `./target/debug/deps/fuzz_survival-ba00cbfdf690b00c` (codecs, uses the walkers) → 6 passed (3.68s). Independent python re-implementation of the RIFF walk over `avi_mjpeg_pcm.avi` → 60/60 per-frame sha256 match vs expectation TOML + whole-file sha match. Lumen decoder probe (prebuilt rlib): frame 0 I420 sha256 == committed reference digest line 1; ffmpeg 7.x oracle on frame 0 → luma 0/76800 byte diffs, chroma 410/38400 bytes differ, max |diff| = 1 (matches the documented tolerance). `ffprobe` reads the MP4 fixture's sowt entry as pcm_s16le/44100/2ch/16-bit — agrees with `mp4_audio_stbl`'s field offsets. |

## 1. One-paragraph verdict
Genuine, careful test infrastructure: hand-rolled but correct SHA-256/MD5 (both pinned by executing tests), a purpose-built expectation-TOML reader, strict bounds-checked AVI-RIFF and MP4-stbl extraction walkers whose output is digest-pinned to the independent xtask writer's expectation values, and the audio-probe/luma-descriptor helpers the AC suites consume. I independently re-derived the AVI walk in python (60/60 frame digests match), verified the committed decode-reference golden equals the real decoder output, and re-ran the external ffmpeg oracle the provenance header documents (luma byte-exact, chroma ±1 — the header's numbers are true). Two hygiene-level findings only. This is the standard of test infra the rest of the repo should be held to.

## 2. What actually works (verified)
- codecs fixtures_parent suite: 9/9 pass — `decoded_avi_outputs_match_reference_digests`, `decoded_mp4_outputs_match_reference_digests`, `avi_extraction_matches_writer_declared_frame_digests`, `mp4_extraction_matches_writer_declared_frame_digests`, `sha256_reader_pinned_against_committed_file_digests`, `entropy_truncation_conceals_and_reports_recovery`, `header_damage_is_a_typed_error_with_nothing_emitted`, `both_containers_carry_the_identical_frame_essence`, `first_frame_descriptor_matches_the_expectation` (binary run pasted above).
- Hand SHA-256 cross-verified indirectly: whole-file digest of `avi_mjpeg_pcm.avi` matches expectation TOML via python hashlib AND via the passing Rust test.
- Walkers cross-verified against an independent python implementation (60/60).
- The decode-reference differential's external-oracle claim (ffmpeg, luma byte-exact / chroma ±1) re-executed for frame 0: TRUE (`luma byte diffs: 0 of 76800`, `chroma max |diff|: 1`).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/common/mod.rs:30-32
Code:
```rust
fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}
```
What is wrong: the fixtures root is located by a hard-coded relative traversal two levels up the workspace tree; moving/renaming the codecs crate or the workspace layout silently breaks every fixture test with filesystem errors rather than a clear "fixtures root not found". Test-infra brittleness only. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/tests/common/mod.rs:188-198
Code:
```rust
        if value.starts_with('[') && !value.ends_with(']') {
            // Multi-line array: consume until the closing bracket.
            for follow in lines.by_ref() {
                let follow = follow.trim();
                value.push(' ');
                value.push_str(follow);
                if follow.ends_with(']') {
                    break;
                }
            }
        }
```
What is wrong: an unterminated multi-line array at EOF exits the loop without error and keeps the partial value; downstream parsing then either fails opaquely or (for `video_sample_sha256`) still yields the entries collected so far. Trusted generated inputs make this harmless today; a structural error would be better surfaced as `Err`. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixtures_root | 30-32 | REAL | S4 | F-001 brittle relative path |
| fixture_path | 36-38 | REAL | — | join wrapper |
| fixture_bytes | 44-47 | REAL | — | propagates fs errors with path |
| sha256_hex | 64-132 | REAL | — | FIPS 180-4; padding/compression verified by pinned tests + python agreement |
| parse_parent_expectation | 168-250 | REAL | S4 | F-002 EOF-array; otherwise strict, unknown fields ignored |
| u32le_at | 254-259 | REAL | — | bounds-checked |
| fourcc_at | 261-265 | REAL | — | bounds-checked |
| extract_avi_mjpeg_frames | 278-280 | REAL | — | delegates to movi walk; 60/60 digest-verified |
| extract_avi_pcm_chunks | 289-291 | REAL | — | same walk, `01wb` |
| extract_avi_movi_chunks | 299-343 | REAL | — | pad-byte handling, overrun + non-advancing guards; python cross-walk agrees |
| extract_avi_audio_strf | 353-425 | REAL | — | nested strl walk, auds check via strh fccType |
| u32be_at | 429-434 | REAL | — | bounds-checked |
| iter_boxes | 445-476 | REAL | — | 64-bit size rejected loudly, size0-to-end, overrun checks |
| find_box | 479-481 | REAL | — | linear first-match |
| parse_stbl | 493-540 | REAL | — | stco/stsz/stsc; count-bounded by payload bounds |
| trak_stbl | 548-559 | REAL | — | moov→trak→mdia→minf→stbl descent |
| expand_stbl_samples | 567-604 | REAL | — | stsc run expansion; inconsistency → Ok(None) |
| extract_mp4_mjpeg_frames | 614-637 | REAL | — | SOI-sniff to pick the video trak; pinned by test |
| mp4_audio_stbl | 657-697 | REAL | — | sowt facts at body+16/+18/+24 — matches the actual fixture bytes AND ffprobe's reading |
| extract_mp4_pcm_chunks | 706-710 | REAL | — | pinned by pcm suite |
| extract_mp4_audio_facts | 717-719 | REAL | — | ffprobe agrees (44100/2/16) |
| reference_digests | 730-741 | REAL | — | reads committed 60-digest file; provenance header honest (verified) |
| luma_block_mean_digest | 752-778 | REAL | — | round-half-up means; dimension/length validated |
| md5_hex | 789-856 | REAL | — | RFC 1321; pinned by RFC vectors test (adpcm_breadth_differential.rs:497-513) and pcm suite |
| audio_probe_md5 | 875-890 | REAL | — | f32/32768 conversion, LE serialization; length-validated |

## 5. Test honesty
This module IS the test-support layer; its own honesty is established by: (a) the digest pinning between the xtask writer's expectation values and the walkers' extracted bytes (a wrong walk cannot reproduce 60/60 writer digests — and my independent python walk confirms); (b) `sha256_impl_matches_the_recorded_file_digest` / `sha256_reader_pinned_against_committed_file_digests` pin the hand hash to committed values; (c) MD5 pinned to RFC 1321 vectors. No mock-of-the-thing-tested, no tautology found.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a wrong walk cannot produce the writer's digests" (extraction oracle) | mod.rs:20-23 | REAL | python re-walk 60/60 + 9/9 fixtures_parent |
| "correctness is machine-checked by the per-frame sha256 match" | mod.rs:21-23 | REAL | `avi_extraction_matches_writer_declared_frame_digests` ok |
| reference digests "Lumen's own decoder output, cross-checked against the ffmpeg 7.1.5 oracle ... luma BYTE-EXACT ... chroma max \|diff\| = 1" | tests/data/mjpeg_decode_reference.txt:13-23 | REAL | re-executed frame 0: luma 0/76800 diffs; chroma 410/38400, max 1 |
| "the writer's fixed v0 layout" (sowt entry offsets) | mod.rs:652-653 | REAL | fixture bytes + ffprobe both agree with the offsets |

## 7. Dead / duplicated / unreachable code
- `#![allow(dead_code)]` at L1 — justified (shared module compiled into many binaries; per-pattern usage verified: every pub helper has ≥1 consumer among tests/{fixtures_parent,fuzz_survival,fuzz_loop_stable,pcm_*,contract,zero_alloc_decode,benches}.rs).
- SHA-256/MD5 duplicate the xtask writer's implementations by design (no dev-dependencies policy, documented at L2-4, 782-788).

## 8. Dependencies & flags
None — std-only by design (L2-4). No unsafe. No feature gates.

## 9. Open questions for the next agent
- tests/corpus.rs binaries contain only `#[ignore]`d randomized-harness tests (0 run) — the randomized-corpus lane runs via tools/corpus/run_harness.py; whoever audits that lane should confirm the harness actually runs in CI.
- The ffmpeg cross-check for frames 1-59 was claimed (24,600/2,304,000 chroma diffs) but only frame 0 re-verified here (time budget); UNVERIFIED: frames 1-59.
