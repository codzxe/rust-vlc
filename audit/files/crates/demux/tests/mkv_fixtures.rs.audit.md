# AUDIT: crates/demux/tests/mkv_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 469 / 11 (2 oracle differentials, 2 lacing tests, 1 fuzz-lite family, 4 error-path tests, 1 probe test + builders) |
| File verdict | SPLIT: oracle differentials REAL (executed live, bit-exact); hand-built lacing fixture + its two tests certify **non-spec lacing semantics** (executed proof against ffmpeg) |
| Reachable from shipped entry point? | Test binary (CI lane); the code under test (`demux::mkv`) is reachable via `apps/cli/src/containers/mod.rs:56 "matroska" => mkv::open` |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux --test mkv_fixtures` → 9 passed / 0 failed (oracle present: ffmpeg 7.1.5 on PATH); ffprobe cross-check of the hand-built fixture bytes → "Error parsing frame sizes"; standalone demuxer probe (audit/scratch/probe5_mkv_lacing.rs) on ffmpeg-legal laced blocks → wrong frame counts, damage=0 |

## 1. One-paragraph verdict
This suite is two things. The ffmpeg-oracle half is excellent and REAL: live ffprobe packet-count differentials and per-stream `-c copy -f md5` payload digests over four committed ffmpeg-authored fixtures, all passing in this sandbox (ffmpeg 7.1.5 present — I ran them, nothing skipped), plus a loud-skip policy (`oracle_available`/`skip_no_oracle` with the `LUMEN_REQUIRE_ORACLE=1` strictness knob) and an honest event-storm bound on the 18 malformed derivatives. The hand-built lacing half is a self-consistent spec violation: the fixture writes the lace count byte as the literal frame count (3 = 3 frames), but Matroska defines it as frames−1 — ffmpeg, the repo's own chosen oracle, refuses the fixture bytes outright ("Error parsing frame sizes"), and when the demuxer is fed ffmpeg-legal laced blocks (count byte 2 = 3 frames) it emits the wrong number of packets with wrong sizes and **zero damage** — silent misparse. The two lacing tests therefore pin and protect the wrong behavior (fixing the demuxer to spec would make them fail), and the module doc's "spec-built" claim is false by the repo's own oracle.

## 2. What actually works (verified)
- `cargo test -p demux --test mkv_fixtures` → `test result: ok. 9 passed; 0 failed` — with ffmpeg on PATH, so the two oracle tests genuinely executed (not skipped): ffprobe stream/packet counts matched for all 4 fixtures (2/2/4/2 tracks), and every audio track's payload MD5 matched `ffmpeg -map 0:N -c copy -f md5 -`.
- `malformed_derivatives_are_typed_or_recover_never_panics`: 18 files, typed-open-refusal prefix check (`demux.*`), walk outcome + damage + `events < 1_000_000` storm bound — a real absence-of-panic sweep over committed derivatives.
- Error-path tests (unknown DocType, zero head, mid-payload truncation surfacing `demux.mkv.truncated`, EBML magic probe) assert exact codes and would fail if the paths regressed.
- The EBML builder (`el`, `uint_payload`, `laced_fixture`) produces byte-valid vints/element structure — ffmpeg parses the container skeleton fine (its refusal is exactly at lacing, nothing else).

## 3. Findings
F-001 [S1]
Where: crates/demux/tests/mkv_fixtures.rs:1-7 (claim), 258-280 (fixture bytes), 286-340 (the two tests)
Code:
```
//! … so the
//! card's fallback applies: a minimal spec-built laced SimpleBlock
//! set, Xiph + EBML + fixed, constructed byte-by-byte here), the
```
```
    // Xiph: 3 frames — two size bytes (4, 5), the last frame is
    // the payload remainder (6).
    let mut xiph_body = vec![3u8, 4, 5];
```
```
    let mut fixed_body = vec![3u8];
```
What is wrong: the lace count byte is written as the literal frame count (3 = three frames). Matroska's Block lacing defines this byte as **number of frames − 1** (ffmpeg's demuxer implements this; it is the repo's designated oracle everywhere else in this suite). Executed proof chain: (a) `ffprobe` on the exact fixture bytes (replicated byte-for-byte, audit/scratch/laced_fixture.mkv) → `[matroska,webm @ …] Error parsing frame sizes.` — the fixture is not a spec-valid laced block; (b) the same file with count byte 2 instead of 3 → ffprobe parses the lace (remaining errors are only the fake Opus payloads); (c) standalone probe (audit/scratch/probe5_mkv_lacing.rs) feeding the ffmpeg-legal variants to the real `MkvDemuxer`: fixed count=2 (3 frames of 2B) → `packets=2 sizes=[3, 3] damage=0`; xiph count=2 (frames 4/5/6) → `packets=2 sizes=[4, 12] damage=0` — the demuxer splits laces at wrong boundaries with zero damage signal, and the repo-convention count=3 gives the "expected" 3×[2,2,2]. Why it matters: the two tests `hand_built_laced_fixture_decodes_all_three_codings` and `laced_frame_payloads_are_byte_exact` lock in non-spec semantics — they will FAIL when someone correctly fixes `demux::mkv` to frames−1 — and every real-world laced MKV (mkvmerge/libmatroska lace audio tracks by default; ffmpeg itself reads them) is silently misparsed by the shipped demuxer. The "spec-built" doc claim is false by the repo's own oracle. What correct looks like: fixture count bytes 2/2/2 (three frames), tests expecting 9 frames, and — in the demuxer — count+1 semantics; the escalation for `crates/demux/src/mkv/*` (not my file) is an S1-grade misparse of common valid input. Confidence: HIGH — Verification: ran (ffprobe cross-check + demuxer probe, outputs above); the in-repo tests pass only because fixture and demuxer are wrong in the same direction (goldens generated by the code under test).

F-002 [S4]
Where: crates/demux/tests/mkv_fixtures.rs:38-72
Code:
```
fn probe_packets(path: &std::path::Path) -> Vec<(String, u64)> {
    let out = Command::new("ffprobe")
```
What is wrong: hand-rolled JSON field scraping (`text.split("\"index\":")` + substring hunting) instead of a JSON parse — works for the fixed ffprobe output of these four fixtures, but any ffprobe output reorder/new field silently yields 0s or miscounts, and `.must()` panics if the ffprobe binary disappears between the `oracle_available()` check and the call (TOCTOU, test-only). Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/mkv_fixtures.rs:100-105
Code:
```
                let i = p.track.handle() as usize;
                while keyframes.len() <= i {
                    keyframes.push((0, 0));
                }
```
What is wrong: `walk_all` silently grows per-track counters for any track ordinal seen in packets — a demuxer emitting packets for an undeclared track ordinal would pass this helper without complaint (the count differential in the first test would only catch it if ffprobe's total disagreed). Minor: the helper trusts track ordinals instead of asserting them against `start()`'s track list. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (test functions)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture / probe_packets / ffmpeg_stream_md5 | 30-92 | REAL | S4 | oracle helpers; live ffmpeg, MD5 ground truth |
| walk_all | 94-113 | REAL | S4 | trusts ordinals (F-003) |
| valid_fixtures_match_ffprobe_packet_counts | 115-145 | REAL | — | executed live, exact per-stream counts |
| audio_payload_is_bit_exact_vs_ffmpeg | 147-195 | REAL | — | executed live, per-track MD5 vs `-map 0:N -c copy -f md5` |
| el / uint_payload / laced_fixture | 201-284 | REAL code, FALSE claim | S1 | byte-valid EBML, spec-invalid lacing counts (F-001) |
| hand_built_laced_fixture_decodes_all_three_codings | 286-298 | FALSE-as-conformance | S1 | pins non-spec semantics (F-001) |
| laced_frame_payloads_are_byte_exact | 300-340 | FALSE-as-conformance | S1 | same; byte-exact against the wrong split |
| malformed_derivatives_are_typed_or_recover_never_panics | 344-407 | REAL | — | 18 derivatives, storm bound, typed prefix check |
| unknown_doctype / zero_head / truncated_cluster / probe | 411-465 | REAL | — | exact error codes |
| open | 467-469 | REAL | — | real input stack (`open_via_input`) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| valid_fixtures_match_ffprobe_packet_counts | YES | live ffprobe oracle, per-stream counts (skips loudly without ffmpeg) |
| audio_payload_is_bit_exact_vs_ffmpeg | YES | live ffmpeg MD5 oracle per audio track |
| hand_built_laced_fixture_decodes_all_three_codings | INVERTED | fails if the demuxer is FIXED — certifies the bug (F-001) |
| laced_frame_payloads_are_byte_exact | INVERTED | same — byte-exact against non-spec boundaries |
| malformed_derivatives… | PARTIAL | absence-of-panic + typed-prefix + storm bound; no specific verdicts asserted (by design) |
| unknown_doctype / zero_head / truncated_cluster / probe | YES | exact codes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg-authored fixtures … all < 32 KiB committed" | mkv_fixtures.rs:9-10 | REAL | files exist in tests/fixtures/generated/valid; suite passes |
| "ffmpeg's muxer writes zero laced blocks in every file we generate — measured" | mkv_fixtures.rs:2-4 | REAL-UNPROVEN | plausible (their generated files); I did not re-measure muxer output |
| "a minimal spec-built laced SimpleBlock set" | mkv_fixtures.rs:5-6 | FALSE | ffprobe rejects the fixture bytes at lacing (F-001, executed); count byte is frames−1 per spec/ffmpeg, the fixture writes frames |
| "ffprobe's per-stream packet count (live measurement…)" | mkv_fixtures.rs:36-37 | REAL | ran live here |

## 7. Dead / duplicated / unreachable code
- The EBML builder in this test duplicates builder logic that exists in demux/src/mkv tests (mkv_editions_tags.rs builds with the same helpers via ebml module) — acceptable for a cross-integration suite, noted.

## 8. Dependencies & flags
- Test-only: testkit (oracle helpers), common (md5_hex, open_via_input), demux, media-types, plugin-api. No feature gates. Uses `unreachable!` in test code only (house-rule compliant).

## 9. Open questions for the next agent
- ESCALATION for the `demux/src/mkv/*` owner: my executed evidence (probe5_mkv_lacing.rs) shows laced blocks with spec/ffmpeg count semantics misparse with **zero damage events** — that is an S1-grade silent-corruption defect on common real-world files (mkvmerge laces audio by default). The demuxer-side finding belongs in the mkv files' audit; this test file must be rewritten together with the fix (count bytes 2/2/2, expectations unchanged at 9 frames).
- Did the corpus differential (tests/fixtures corpus, .oracle.sha256 files) ever include a real mkvmerge-authored laced file? If one exists it should have caught this — worth checking how the corpus oracle digests are produced.
