# AUDIT: crates/playlists/tests/hls_attrs.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 986 / 8 helpers + 20 tests |
| File verdict | REAL (strong evidence file: real ffmpeg fixtures, disk-derived truths, full RFC 3986 matrices, seeded fuzz-lite) |
| Reachable from shipped entry point? | Test file; the code under test IS reachable: playlists ← streaming (Cargo.toml:42) ← apps/cli (Cargo.toml:62) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | baseline citation (no cargo rerun needed); `cat` of the three fixture playlists; `ls -la` of fixture dirs; sed quote verification |

## 1. One-paragraph verdict
This is the rare test file that does what its header promises. The three ffmpeg fixtures exist, are genuine ffmpeg-shaped output (trailing-comma EXTINF lines, VERSION 3/4/7, BYTERANGE offsets), and the tests derive segment counts and byte totals from the files on disk rather than constants (verified: 8084+7708+7708+7708+4136 = 35344 = single_file/pl.ts actual size; init.mp4 = 764 bytes actual). The URI-joining tests are the complete RFC 3986 §5.4.1 normal matrix (23 cases) plus the §5.4.2 abnormal set. The fuzz-lite tests are seeded (reproducible), assert the fuzz is not vacuous (`parsed_count > 100`), and enforce the roundtrip law on every parseable mutant. All 20 tests pass in the orchestrator baseline. The one real defect is that the writer golden pins an EXTINF emission without the comma RFC 8216 §4.3.2.1 requires — a conformance bug in `playlists::write` blessed as expected output here.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt: `Running tests/hls_attrs.rs` → `test result: ok. 20 passed; 0 failed; 0 ignored` (line 4703-4707 area). Fixtures re-measured by me: `ls -la` shows pl0..pl4.ts = 8084/7708/7708/7708/4136 bytes, init.mp4 = 764, pl.ts = 35344 — exactly the values the tests derive at runtime (`fixture_len`), so the "on-disk truth" claims are true.

## 3. Findings
F-001 [S3]
Where: crates/playlists/tests/hls_attrs.rs:557-567 (golden) and crates/playlists/src/m3u.rs:370-377 (writer)
Code (hls_attrs.rs golden excerpt):
```
            "#EXT-X-BYTERANGE:8084@0\n",
            "#EXTINF:4\n",
            "media1.ts\n",
```
Code (m3u.rs writer):
```
            out.push_str("#EXTINF:");
            if let Some(d) = entry.duration {
                // Shortest round-trip decimal: `123.0` prints as `123`,
                // `123.5` as `123.5`; both re-parse exactly.
                out.push_str(&format!("{d}"));
            }
            if let Some(t) = &entry.title {
                out.push(',');
                out.push_str(t);
            }
```
What is wrong: the writer emits `#EXTINF:4` with NO comma when there is no title, and this test pins that as the canonical golden. RFC 8216 §4.3.2.1 fixes the tag format as `#EXTINF:<duration>,[<title>]` — the comma is mandatory even with an empty title; the ffmpeg fixtures this very suite uses as truth all carry it (`#EXTINF:2.043356,` in all three pl.m3u8 files, verified by cat). Why it matters: any playlist this crate rewrites and hands to a strict HLS consumer is out-of-spec on every duration-only entry; the test proves the code round-trips with itself, not that it emits spec text — and the file nowhere flags the divergence from its own ffmpeg ground truth. What correct looks like: writer emits the comma always; golden updated to `"#EXTINF:4,\n"`. Confidence: HIGH — Verification: read both files + fixtures.

F-002 [S4]
Where: crates/playlists/tests/hls_attrs.rs:853-859
Code:
```
        // Splice 0-3 random interesting bytes into the line.
        for _ in 0..rng.below(4) {
            let pos = rng.below(out.len() as u64 + 1) as usize;
            out.insert(
                pos,
                INTERESTING[(rng.below(INTERESTING.len() as u64)) as usize],
            );
        }
```
What is wrong: `hls_soup` documents "splice ... into the line" but inserts into the WHOLE accumulated buffer `out`, so bytes land in previously emitted lines, not the current template line; also the comment says "0-3" while `rng.below(4)` yields 0-3 — that part is right. The cross-line splice is harmless for a no-panic fuzz (arguably broader coverage) but the comment misdescribes the mechanism. Test-only. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_dir | 32-36 | REAL | — | CARGO_MANIFEST_DIR-based |
| total_duration | 39-41 | REAL | — | f64 sum, used with 0.01 tolerance |
| must_parse | 44-49 | REAL | — | let-else + unreachable (test-appropriate) |
| assert_roundtrip | 52-56 | REAL | — | parse(write(p))==p — model-level, honestly named |
| fixture_len | 59-64 | REAL | — | disk truth, unreachable! on missing fixture |
| ffmpeg_hls_ts_vod_matches_ffprobe_truth | 71-120 | REAL | — | derives segment count from read_dir; pins ffmpeg EXTINF verbatim |
| ffmpeg_hls_single_file_byteranges_match_disk_truth | 126-177 | REAL | — | ranges tile the file exactly (cursor == 35344) |
| ffmpeg_hls_fmp4_map_matches_ffprobe_truth | 183-217 | REAL | — | MAP stamped on every entry; init.mp4 764 bytes |
| byterange_forms_and_malformed_tolerance | 222-270 | REAL | — | 4 bad lines → BadExtXTag warnings, entries survive |
| discontinuity_flags_next_segment_only | 273-296 | REAL | — | stateful flag, junk on valueless tag = damage |
| key_recorded_statefully_and_none_clears | 299-352 | REAL | — | quoted-URI commas, hex IV case tolerance, NONE clears; KEY emitted per change (count==3) |
| unknown_enumerated_key_method_is_recorded | 355-366 | REAL | — | Other("FUTURE-METHOD") preserved |
| malformed_keys_skip_line_and_keep_state | 369-406 | REAL | — | 13 hostile KEY lines, prior state kept |
| map_with_quoted_byterange | 409-456 | REAL | — | stateful MAP + 3 bad MAP shapes |
| endlist_positions_and_dangling_rules | 459-479 | REAL | — | dangling detection honest |
| quoted_attribute_contents_survive_awkward_chars | 482-493 | REAL | — | commas/@/= inside quotes |
| write_hls_canonical_golden_pin | 496-573 | REAL | S3 | pins comma-less EXTINF (F-001); delta encoding covered |
| hls_dirty_corpus_roundtrips | 576-614 | REAL | — | 12 hostile corpus inputs incl. 100-attr line |
| resolve_uri_rfc3986_normal_examples | 619-650 | REAL | — | full §5.4.1 matrix, 23 cases (counted) |
| resolve_uri_rfc3986_abnormal_examples | 653-679 | REAL | — | §5.4.2 set incl. `http:g` |
| resolve_uri_hls_shaped_cases | 682-729 | REAL | — | CDN shapes, scheme-relative, query/fragment rules |
| resolve_uri_windows_posix_and_local_bases | 732-766 | REAL | — | drive/UNC verbatim, local-root clamp |
| resolve_uri_on_parsed_ffmpeg_playlists | 769-790 | REAL | — | end-to-end on real fixtures |
| SplitMix64::{new,next_u64,below,byte} | 797-813 | REAL | — | standard splitmix64 constants |
| hls_soup | 823-863 | REAL | S4 | comment/code splice mismatch (F-002) |
| fuzz_lite_hls_soup_and_mutations_no_panic | 866-920 | REAL | — | 10k iters, mutation of real fixture, roundtrip enforced, vacuity guards |
| fuzz_lite_resolve_uri_no_panic_and_total | 923-986 | REAL | — | 5k iters, bounded output, absolute-base fixed point |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 20 | YES | every test asserts concrete parsed/typed values against fixed vectors, RFC matrices, or live disk measurement; fuzz tests assert non-vacuity counters; mutation of the parser, writer, or resolve_uri breaks at least one assertion (e.g., drop BYTERANGE offset handling → ffmpeg_hls_single_file fails at `br.offset == Some(cursor)`; break dot-segment removal → resolve_uri_rfc3986_abnormal fails) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Generated this session with the card's recipe (ffmpeg 7.1.5.5...)" | hls_attrs.rs:5-8 | REAL-UNPROVEN (fixture CONTENT consistent with ffmpeg output: trailing-comma EXTINF, VERSION 3/4/7, exact TS sizes; ffmpeg itself not runnable here — UNVERIFIED that this exact binary produced them) | cat of fixtures; ls -la sizes match pins |
| "The tests re-derive segment counts and byte totals from the committed files themselves" | hls_attrs.rs:19-20 | REAL | read_dir at 77-83; fixture_len at 59-64 |
| "duration 9.046543, 5 segments" / "pl.ts = 35344 bytes" / "init.mp4 = 764 bytes" | hls_attrs.rs:15-17 | REAL | EXTINF sums verified by test; file sizes verified by ls |
| "Roundtrip law: `parse(write(p)) == p`" | hls_attrs.rs:51 | REAL (accurately scoped — model equality, NOT byte-level ffmpeg reproduction) | helper at 52-56 |

## 7. Dead / duplicated / unreachable code
None. All helpers used. Note: `assert_roundtrip` proves self-consistency, not spec-conformant output (see F-001).

## 8. Dependencies & flags
playlists crate API only (parse_bytes/parse_detailed/write/resolve_uri/types) + std. No features. Fixtures embedded via include_str! and read at runtime — both paths exercised.

## 9. Open questions for the next agent
- Does any shipped code path ever CALL `playlists::write` (rewrite playlists)? If yes, F-001's comma omission becomes user-visible (S2). If no, it stays S3. UNVERIFIED here — audit of callers belongs to the playlists-crate source auditor.
- Who consumes the fuzz seeds' determinism guarantee (CI)? No assertion pins exact parsed_count, only >100 — deliberate loose coupling, fine.
