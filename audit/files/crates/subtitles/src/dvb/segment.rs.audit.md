# AUDIT: crates/subtitles/src/dvb/segment.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-23 |
| Lines / functions | 642 / 11 (+9 tests) |
| File verdict | REAL-UNPROVEN at product level / UNREACHABLE from shipped entry points (crate not linked by any app) |
| Reachable from shipped entry point? | NO — no Cargo.toml of lumen-cli/desktop/desktop-tauri depends on `subtitles`; only `fuzz/Cargo.toml:50` does (outside workspace members). In-crate chain: `dvb/mod.rs:343` `DvbDecoder::decode_pes` -> `walk_segments`/`parse_*` -> `dvb/fuzz.rs:58` `fuzz_sub_dvb` + tests only. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p subtitles --lib dvb::segment` -> 9 passed, 0 failed (exit 0); `sed -n` quote verification (all quotes below verbatim); baseline `audit/runs/baseline/cargo-test-workspace.txt:5741-5760` (267 lib tests ok). |

## 1. One-paragraph verdict
This is a genuine, dense, EN 300 743 §7.2 segment grammar with bit layouts that match the standard and ffmpeg's `dvbsubdec` (verified entry-by-entry below), plus honest typed-degradation behavior. The one real defect: `parse_display_def` does unchecked `u16` arithmetic on parsed fields (`+1` and `max-min+1`) that overflows on malformed input — panic in debug profiles (the profile every test and the committed fuzz-survival lane runs in), silent wrap in release. Nothing in the repo links this crate into a shipped binary, so user-facing blast radius today is zero; the in-repo exposure is the cargo-test lane.

## 2. What actually works (verified)
— `cargo test -p subtitles --lib dvb::segment` -> all 9 tests pass (exit 0): header walk + truncation accounting, bad sync, page region refs, region 2/4/8-bit fills, 12-bit object positions incl. character-coded 8-byte entries, invalid-depth fallback, CLUT full+limited-range unpack with exact bit patterns, object field lengths, display definition.
— Bit layouts cross-checked against EN 300 743: page region entry = id/reserved/x16/y16 (line 116-120); region object type = top 2 bits of byte 2, 12-bit x/y packed (lines 201-203); limited-range CLUT = Y6/Cr6/Cb6/T2 MSB-packed over 3 bytes, `<<2` scaling (lines 291-294) — test at lines 582-597 pins the exact bits. All match the standard.

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/dvb/segment.rs:371-381
Code:
```rust
    let width = u16::from_be_bytes([data[1], data[2]]) + 1;
    let height = u16::from_be_bytes([data[3], data[4]]) + 1;
```
and (lines 377-381)
```rust
        let x = u16::from_be_bytes([data[5], data[6]]);
        let wx = u16::from_be_bytes([data[7], data[8]]);
        let y = u16::from_be_bytes([data[9], data[10]]);
        let wy = u16::from_be_bytes([data[11], data[12]]);
        (x, y, wx - x + 1, wy - y + 1)
```
What is wrong: four unchecked u16 arithmetic ops on attacker-controlled parsed bytes. `65535 + 1` (width/height max value) and `wx - x` when `wx < x` (min>max, trivially authored) overflow. In debug/test profiles (`overflow-checks` on — the profile `cargo test` and the committed `tests/dvb_vobsub_fuzz_survival.rs` lane use) each is a panic: `attempt to add with overflow` / `subtract with overflow`. In release they wrap; `dvb/mod.rs:549` (`w > 0 && h > 0 && w <= 8192 && h <= 8192`) then rejects most wrapped values, but a wrapped width of exactly e.g. 4000 from an inverted window is accepted as truth.
Why it matters: `parse_display_def` sits directly on the fuzzed PES path (`dvb/fuzz.rs:58` -> `decode_pes` -> `dvb/mod.rs:537`). The fuzz harness's own doc (dvb/fuzz.rs:39-40) promises "Never panics" — this is a reachable panic contradicting that contract; the committed survival test only survives because its seeds never author min>max or 0xFFFF sizes. `display_def_parse` (line 625) tests only well-formed windows — the underflow case has zero coverage.
What correct looks like: checked arithmetic (`checked_add`/`checked_sub` returning `None`, damage-typed) or u32 widening before the +1/-operations, plus a truncation-style unit test for min>max and 0xFFFF.
Confidence: HIGH — Verification: read (Rust overflow semantics; debug panic / release wrap); panic not executed (could not add a test: source is read-only). UNVERIFIED-by-execution only for the literal panic backtrace.

F-002 [S4]
Where: crates/subtitles/src/dvb/segment.rs:157-165 (with 143-150)
Code:
```rust
    let depth_code = (data[6] >> 2) & 7;
    let depth = 1u16 << (depth_code & 7);
    let (fill_color, objects) = if depth == 8 {
        (data[8], &data[10..])
    } else if depth == 4 {
        (data[9] >> 4, &data[10..])
    } else {
        ((data[9] >> 2) & 3, &data[10..])
    };
```
What is wrong: `fill_color` is extracted at the RAW depth, but `RegionSegment::depth()` (lines 143-150) falls back to 4 when the raw depth is invalid (codes 0, 4-7). For those already-damaged streams the fill index is a 2-bit extraction (range 0-3) applied to a region rendered as 4-bit. `dvb/mod.rs:444` records `RegionDepthInvalid` damage, so the input is flagged, but the fill value and the render depth disagree.
Why it matters: cosmetic wrong fill on invalid input; damage ledger already admits the stream is broken.
What correct looks like: extract `fill_color` using the post-fallback depth (single source of truth for depth).
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/subtitles/src/dvb/segment.rs:457-492
Code:
```rust
    fn region_parse_4bit() {
```
(then, line 484) `assert_eq!(r.depth(), 2);`
What is wrong: the test name says 4-bit; the data encodes depth code 1 (=> 2 bits) and the body asserts depth 2. The name lies about what is pinned.
What correct looks like: rename to `region_parse_2bit` (the 4-bit extraction branch `data[9] >> 4` then has NO direct unit coverage — only 8-bit (via `object_parse_fields`? no) — actually no test exercises the `depth == 4` fill branch at all).
Confidence: HIGH — Verification: read + ran the test (passes; it pins 2-bit, not 4-bit).
This is also a coverage gap, slightly above pure naming: the 4-bit fill branch is untested. Kept S4 because damage-typed and narrow.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SegmentKind::of | 26-37 | REAL | - | total 8-way map incl. Unknown(u8); exercised by tests |
| walk_segments | 51-73 | REAL | - | bounds-checked (`p+6<=len`, `data_start+len>len` breaks); truncation returned via `used`; no overflow (len<=65535, usize) |
| parse_page | 86-96 | REAL | - | len>=2 guard; version/state nibble math per §7.2.2 |
| PageSegment::region_refs | 111-121 | REAL | - | `as_chunks::<6>` drops partial tail; damage accounted upstream (mod.rs:419-425) |
| RegionSegment::depth | 143-150 | REAL | - | `1<<code`, fallback 4 |
| parse_region | 153-177 | REAL | F-002 | len>=10 guard; all indexes < 10 in-bounds |
| RegionSegment::object_refs | 196-224 | REAL | - | 6/8-byte entries, `<8 break` guard before fg/bg index; matches ffmpeg layout |
| parse_clut | 234-243 | REAL | - | len>=2 guard |
| ClutSegment::entries | 249-301 | REAL | - | zero-depth-flags and truncation stop the walk (break); limited-range unpack bit-exact (test 582-597) |
| parse_object | 317-349 | REAL | - | len>=3/len>=7 guards; `top_len+bottom_len` cannot overflow usize; coding!=0 returns typed-empty (mod.rs records ObjectCodingUnsupported) |
| parse_display_def | 364-393 | REAL (defective) | F-001/S2 | len>=5 / len>=13 guards correct; arithmetic overflow sites |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_walk_and_truncation | YES | asserts used=15 and exact kinds/data of 2 segments |
| bad_sync_stops_walk | YES | asserts empty + used=0 |
| page_region_refs_parse | YES | asserts both refs' id/x/y + partial-entry drop |
| region_parse_4bit | YES (misnamed) | asserts depth 2, fill, object fields — but name promises 4-bit |
| region_object_12bit_positions | YES | pins 0xABC/0x123 and char-coded fg/bg |
| region_invalid_depth_fallsback_to_4 | YES | pins fallback |
| clut_entries_full_and_limited | YES | exact bit patterns incl. limited-range scaling |
| object_parse_fields | YES | pins field slices + truncation Nones |
| display_def_parse | PARTIAL | pins valid parses and truncated-window None; does NOT cover F-001's overflow inputs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "page region entries are 6 bytes (id, reserved, x:16, y:16)" | segment.rs:6-8 | REAL | lines 116-120; EN 300 743 table 5 layout |
| "region object entries are 6 bytes (+2 for character-coded) with object type in top 2 bits of byte 2 and 12-bit x/y" | segment.rs:8-9 | REAL | lines 201-211 |
| "Entries with a zero depth-flags byte stop the walk (invalid per EN 300 743 — the reference logs and skips the rest)" | segment.rs:246-248 | REAL | lines 259-261 break |
| "Limited range: Y(6) Cr(6) Cb(6) T(2) packed MSB-first over 3 bytes" | segment.rs:279-281 | REAL | lines 282-294 + test 582-597 bit-exact |
| fuzz harness "Never panics" (path through this file) | dvb/fuzz.rs:39-40 | FALSE for malformed display definitions | F-001: overflow panic in debug profile |

## 7. Dead / duplicated / unreachable code
None dead in-file. Whole crate unreachable from shipped binaries (see header). `fuzz/Cargo.toml:50` (not a workspace member) is the only external consumer.

## 8. Dependencies & flags
No unsafe, no feature gates, std-only + media-types (unused by this file). Clippy deny-unwrap/expect honored (uses `testkit::Must` in tests only).

## 9. Open questions for the next agent
- dvb/mod.rs:549 silently drops a size-invalid display definition WITHOUT a damage event (contrast: RegionSizeInvalid at mod.rs:452-458 does record one). Check whether DvbDamage has a DisplayDef variant that was forgotten.
- Does any seeded mutation in tests/dvb_vobsub_fuzz_survival.rs ever set the DDS window flag (0x08 in byte 0) with min>max? If yes, the baseline pass would contradict F-001 — I did not re-derive the seeds (UNVERIFIED, seed analysis not run).
