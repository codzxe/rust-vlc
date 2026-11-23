# AUDIT: crates/codecs/src/video/hevc/parser/nal.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 1517 / ~36 prod + ~24 test items |
| File verdict | REAL-UNPROVEN in-library, UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — no "hevc" arm in apps/cli/src/pipeline_host.rs:241-346 `decoder_for` nor apps/web/src/host.rs:590+ `builtin_decoder_for`; `rg -ln "codecs::video::hevc"` outside crates/codecs → 0 hits |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg (callers, reachability); `./target/debug/deps/codecs-bae0f1756cc0a809 hevc::parser::nal --nocapture` → 10 passed; 0 failed, exit 0 |

## 1. One-paragraph verdict
The NAL layer (Annex-B splitter, length-prefixed splitter, hvcC record
parser, emulation-prevention removal, Exp-Golomb bit reader) is genuine,
careful, typed-error code with a real fuzz-lite suite — I ran all 10 tests
live on the prebuilt binary, all passed. But the whole module is dead code in
every shipped lane: neither the CLI nor the web host ever dispatches to any
HEVC decoder, and nothing outside crates/codecs references
`codecs::video::hevc`. Additionally, `LengthPrefixedNals::next` has a real
usize-overflow panic on 32-bit targets (wasm32 is an explicit project
target), and the file's flagship corpus test silently no-ops when the corpus
directory is absent — which it is in this sandbox.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 hevc::parser::nal
--nocapture` → `test result: ok. 10 passed; 0 failed` incl.
`fuzz_lite_no_panics_on_random_and_mutated_bytes ... ok` (10k random + 5k
mutated inputs, 64-bit host). Bit reader, splitters, hvcC parse, emulation
removal all behave on the host platform.

## 3. Findings

F-001 [S2]
Where: crates/codecs/src/video/hevc/parser/nal.rs:521-530
Code:
```rust
        let total = len + self.length_size;
        if total > rest.len() {
            self.pos = self.data.len();
            return Some(Err(HevcError::Truncated {
                needed: total as u64,
                have: rest.len() as u64,
            }));
        }
        self.pos += total;
        Some(Ok(&rest[self.length_size..self.length_size + len]))
```
What is wrong: with `length_size == 4` and a parsed length of `0xFFFF_FFFF`,
on any 32-bit usize target `len + 4` overflows: debug builds panic
("attempt to add with overflow"), release builds wrap to 3, pass the
`total > rest.len()` guard (rest.len() >= 4), then `&rest[4..3]` panics
("slice index starts at 4 but ends at 3"). wasm32-unknown-unknown is an
explicit project target (crates/codecs/src/video/mod.rs:18-21 discusses it),
so this is a one-header remote-crash archetype on a supported platform.
Why it matters: hevc/mod.rs:39-41 claims "Every parser entry point is total:
any byte string yields either a parsed structure or a typed `HevcError` — no
panics" — false on 32-bit targets; the fuzz-lite suite that "pins" it runs
only on the 64-bit host and never feeds `LengthPrefixedNals` random data at
all (the fuzz loop at :1358-1385 hits vps/sps/pps/unescape/hvcc only).
Currently unreachable from shipped entry points (see F-003), which is the
only reason this is S2 and not S1.
What correct looks like: `len.checked_add(self.length_size as u64)` compared
against `rest.len() as u64` before any usize arithmetic on `len`.
Confidence: HIGH — Verification: read; overflow arithmetic reasoned (not
executed on wasm32; UNVERIFIED by execution, cargo-build-budget).

F-002 [S3]
Where: crates/codecs/src/video/hevc/parser/nal.rs:1441-1444
Code:
```rust
        let Some(path) = corpus_path("0044-raw-video.265") else {
            eprintln!("SKIP hevc corpus: 0044-raw-video.265 absent (corpus re-imaged)");
            return;
        };
```
What is wrong: the strongest end-to-end test in the file (real Annex-B
stream, VPS/SPS/PPS + 64 slice-segment headers) silently returns Ok when
`/tmp/my-project/corpus-v4/media` is absent — and it IS absent
(`test -d /tmp/my-project/corpus-v4/media` → ABSENT). I observed the skip
live: my run printed `SKIP hevc corpus: 0044-raw-video.265 absent` and the
test still shows `ok` (and is counted as a pass in the orchestrator baseline,
cargo-test-workspace.txt:733). The "verified against the corpus" story
therefore rests entirely on the pinned byte constants (VPS_0100/SPS_0100/
PPS_0100), not on the actual corpus in this environment.
Why it matters: baseline "4744 passed" includes tests that verified nothing.
What correct looks like: distinguish SKIP from PASS (#[ignore] or a
dedicated marker the harness reports).
Confidence: HIGH — Verification: ran (skip message + `ok` observed).

F-003 [S3]
Where: crates/codecs/src/video/hevc/parser/nal.rs (whole file) + mod wiring
Code (apps/cli/src/pipeline_host.rs:241-246, the only dispatch site):
```rust
        match track.codec.as_str() {
            "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
What is wrong: there is no `"hevc"` arm in the CLI's `decoder_for` (arms:
mjpeg/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm_/av1/vp9/dnxhd/h264) nor in
apps/web/src/host.rs `builtin_decoder_for` (mjpeg/pcm_/adpcm_/flac/mp3/
vorbis/opus/.../vp9/dnxhd). Nothing outside crates/codecs references
`codecs::video::hevc` at all (rg across plugins/, apps/, crates/ → 0 hits).
The demux side happily labels tracks "hevc" (crates/demux/src/mp4/mod.rs:441),
so the CLI then fails with exit 65 "no decoder for `hevc`" — matching
audit/runs/baseline/mp4-smoke.md. 1517 lines of parser + fuzz suites ship in
the binary but can never execute from a shipped entry point.
Why it matters: dead weight in the release binary; "parser self-build M"
evidence is test-only, not product reachability.
Confidence: HIGH — Verification: read + rg.
What correct looks like: either register the (refusing) HevcDecoder behind
the hevc descriptor so its typed `hevc.hw.required` refusal is what users
hit, or gate the module out of release builds.

F-004 [S4]
Where: crates/codecs/src/video/hevc/parser/nal.rs:691-697
Code:
```rust
    pub fn looks_like_hvcc(bytes: &[u8]) -> bool {
        bytes.len() >= 23
            && bytes[0] == 1
            && bytes[13] >> 4 == 0x0f
            && bytes[15] >> 2 == 0x3f
            && bytes[16] >> 2 == 0x3f
    }
```
What is wrong: `parse` enforces five reserved patterns (bytes 13,15,16,17,18
at :624-628) but the sniffing helper checks only three of them (13,15,16); a
record that passes the sniff can still fail parse on bytes[17]/[18]. Sniff/
parse asymmetry is documented as heuristic but invites
looks_like_hvcc→parse→Err sequences in callers.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HevcError + Display/Error | 35-100 | REAL | - | 6 typed variants, all constructed somewhere |
| NalUnitType::from_id/id | 162-225 | REAL | - | round-trip tested over 0..=63 |
| NalUnitType::is_irap/is_vcl | 231-240 | REAL | - | correct ranges 16..=23 / <=31 |
| NalHeader::split | 267-294 | REAL | - | checks forbidden bit, temporal_id!=0 |
| next_start_code | 300-311 | REAL | - | O(n) scan, borrowed |
| trim_trailing_zeros | 317-326 | REAL | - | spec-justified |
| AnnexBNals::new/next | 349-385 | REAL | - | no infinite loop; pos advances >=3 |
| annexb_nals | 390-392 | REAL | - | thin wrapper |
| unescape_rbsp_into | 410-442 | REAL | - | overlapping escapes re-scanned correctly (pinned test) |
| unescape_rbsp | 448-452 | REAL | - | wrapper |
| LengthPrefixedNals::new | 475-487 | REAL | - | validates 1..=4 |
| LengthPrefixedNals::next | 492-531 | REAL (buggy on 32-bit) | S2 | F-001 overflow |
| HvcCConfig::parse | 610-684 | REAL | - | bounds-checked, typed errors, alloc bounded by input |
| HvcCConfig::looks_like_hvcc | 691-697 | REAL | S4 | F-004 partial pattern set |
| read_u16 / read_counted_nal | 701-730 | REAL | - | u16 lengths, no overflow |
| BitReader::{new,bit,bits,ue,se} | 748-828 | REAL | - | ue capped at 31 leading zeros |
| BitReader::more_rbsp_data | 837-850 | REAL | - | two-1-bit rule, sound |
| BitReader::finish | 859-890 | REAL | - | no OOB (stop bit index < len) |
| tests: Bits, corpus_path, splitmix64 | 899-1005 | REAL | S3 | corpus_path → F-002 |
| 10 #[test] fns | 1007-1516 | REAL | - | ran live, all pass; corpus one no-ops here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nal_classification_roundtrips_table_7_1 | YES | exhaustive 0..=63 round-trip |
| header_split_decodes_two_byte_geometry | YES | pinned geometry incl. all 3 error paths |
| annexb_splits_three_and_four_byte_prefixes | YES | exact byte vectors |
| emulation_removal_handles_overlapping_escapes | YES | chained-escape vector + real VPS bytes |
| length_prefixed_split_and_typed_failures | PARTLY | covers 64-bit paths; cannot catch F-001 (needs 32-bit) |
| hvcc_parse_field_exact_on_corpus_arrays | YES | field-exact against pinned record |
| hvcc_typed_error_paths | YES | 5 mutation cases |
| bit_reader_roundtrip_and_bounds | YES | writer/reader round-trip + bounds |
| fuzz_lite_no_panics_on_random_and_mutated_bytes | PARTLY | strong, but LengthPrefixedNals/annexb only every-7th & not length-prefixed |
| corpus_0044_parameter_sets_and_slice_prefixes_parse | NO (here) | silently skips — corpus dir absent (observed) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every parser entry point is total ... no panics" | hevc/mod.rs:39-41 | FALSE on 32-bit targets | F-001 arithmetic; UNVERIFIED by wasm32 execution |
| "steady-state parse path allocates nothing" | nal.rs:24-29 | REAL | borrowed-slice iterators, unescape reuses `out` (read) |
| "hvcC ... five fixed reserved patterns enforced" | nal.rs:599-601 | REAL | :624-628 |
## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-003). No internal
dead functions found.
## 8. Dependencies & flags
`use core::fmt` only — no external deps. Module unconditional
(crates/codecs/src/video/mod.rs:32), compiled into release binary but never
called.
## 9. Open questions for the next agent
- hevc/decode.rs (next file): does the HevcDecoder parse real samples
  (reaching this nal.rs) or refuse before? Confirm exit-65 refusal chain.
- Does any 32-bit (wasm32) lane ever parse attacker MP4 hvcC? If a future
  card wires hevc, F-001 becomes S1 immediately.
