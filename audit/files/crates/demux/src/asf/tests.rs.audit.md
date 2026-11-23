# AUDIT: crates/demux/src/asf/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1578 / 44 (29 #[test] + 15 helpers/consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `#[cfg(test)] mod tests;` (asf/mod.rs:76-77); compiled only under `cargo test -p demux`. That is the correct wiring for a test module. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | `sha256sum crates/demux/src/asf/fixtures/*` (all 11 pins match, exit 0); `rg encrypted_stream crates/demux/src` (no production call site); `rg -i "extension|codec_list" tests.rs` (no hits); baseline citation: audit/runs/baseline/cargo-test-workspace.txt:1698-2166 |

## 1. One-paragraph verdict
This is a real, strong test suite: 29 tests drive the header walk, packet
grammar, fragment assembler, seek lanes, and a 10-fixture ffmpeg differential
pinned by sha256, plus a live ffprobe cross-check that ran and passed in the
orchestrator baseline. All 11 committed fixture sha pins were re-verified
byte-exact by me with system sha256sum. The one substantive defect is evidence
theater: `error_constructors_table` lists `demux.asf.encrypted-stream` as a
working refusal, but no production code path ever mints it — an encrypted ASF
file is walked as if unencrypted, and the test row hides that.

## 2. What actually works (verified)
- `sha256sum crates/demux/src/asf/fixtures/*` -> all 11 files match the
  `expected_sha` pins at tests.rs:1486-1496 (e.g. `2f1abf090401e5d4
  wma_wmav2_mono8k.wma`, `a9db9175da47fe6c asf_stream_broadcast.wmv`).
- Baseline run: audit/runs/baseline/cargo-test-workspace.txt:1774-1800 — every
  `asf::tests::*` test "ok", including
  `fixture_family_walks_match_the_measured_truth` (line 1780) and
  `fixture_family_cross_checks_against_live_ffprobe` (line 2166) — so the
  ffprobe lane executed, not skipped, in the baseline environment.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/asf/tests.rs:412 (and error.rs:127)
Code:
```
        (error::encrypted_stream(2), "demux.asf.encrypted-stream"),
```
What is wrong: the constructor-table test proves the error constructor mints a
code, and reads as if encrypted streams refuse typed. `rg encrypted_stream
crates/demux/src --glob '!*error.rs' --glob '!*tests.rs'` returns ZERO
production call sites — the walk never emits it (error.rs:120-126 admits the
gate is "pending the spec pin"). Why it matters: an encrypted WMA/WMV (Content
Encryption object present) is walked like plaintext; SP+52 raw bytes are
exposed but never checked, packets yield garbage or opaque refusals instead of
`demux.asf.encrypted-stream`. The test row is evidence for a refusal that
cannot fire. What correct looks like: either a walk-level test patching the
encryption flag byte and asserting the typed refusal (which would fail today —
honestly), or the row removed until the gate exists.
Confidence: HIGH — Verification: read + rg (no call sites).

F-002 [S3]
Where: crates/demux/src/asf/tests.rs (whole file; absence)
Code:
```
use super::{AsfDemuxer, EXTENSIONS, probe};
```
What is wrong: header.rs parses CODEC_LIST (bounded by MAX_CODEC_LIST_ENTRIES
= 256, header.rs:90), EXTENDED_CONTENT_DESCRIPTION, HEADER_EXTENSION and
EXTENDED_STREAM_PROPERTIES (header.rs:609-615), yet tests.rs contains zero
tests for any of those objects — `rg -i "extension|codec_list" tests.rs` finds
only the unrelated `EXTENSIONS` import. Coverage is implicit (fixtures carry
clean codec-list/ECD objects) so corruption, entry-count overflow, and the
extension-child-walk bounds are untested at unit level. The asf fuzz harness
(asf/fuzz.rs:52-80) sweeps only 2 short templates, which does not close this.
What correct looks like: builder-driven tests for codec-list count > cap, ECD
pair-count mismatch, and a Header Extension child walk that overruns its
declared size.
Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: crates/demux/src/asf/tests.rs:280-284
Code:
```
    // Integer PCM at a width the registry does not carry: raw
    // passthrough, never a silent nearest-guess.
    let wfx = waveformatex(1, 1, 8_000, 3, 24);
    let mut asf = started(minimal_asf_audio(&wfx));
    assert_eq!(ok(asf.start())[0].codec.as_str(), "pcm_s24le");
```
What is wrong: bits=24 IS in the registry (header.rs:395 `24 =>
"pcm_s24le".to_string()`), so the first sub-case is a registry hit, not the
passthrough the comment promises; only the second sub-case (bits=48) exercises
the fallback. Misleading comment on an otherwise real test.
Confidence: HIGH — Verification: read (header.rs:394-399).

F-004 [S4]
Where: crates/demux/src/asf/tests.rs:1305-1310
Code:
```
            let landed = ok(asf.seek(span_at(1000)));
            assert!(
                matches!(&landed.start, Stamp::Present(ts) if ts.ticks >= 0),
```
What is wrong: `ts.ticks >= 0` passes for any non-negative landing; combined
with `resumed > 0` it proves only "seek returned and packets followed", not
that the fixture index re-anchored at the right bucket. Real re-anchor
arithmetic is only proven on the synthesized two-bucket index
(simple_index_seek_reanchors_the_walk). Weak assertion, name promises more.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/asf/tests.rs:464, 496-497
Code:
```
    b.push((if p.keyframe { 0x80 } else { 0x00 }) | u8::try_from(p.stream).unwrap_or(0x7F));
```
```
    let padding = total - b.len();
    assert!(padding <= u16::MAX as usize, "packet overflow");
```
What is wrong: builder hygiene — `unwrap_or(0x7F)` silently mangles any
stream id > 127 into 127, and `total - b.len()` panics with a raw overflow
message before the intended "packet overflow" assert can fire. Test-only blast
radius (streams used are 1, 2, 9; payloads never exceed `total`), but a future
vector author gets a confusing failure. Also `sha256_16` (1505-1578) is a
73-line SHA-256 twin of tests/common/mod.rs:164 (comment admits "in-crate
twin") — forced by the src/tests barrier, but duplication nonetheless.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ok / started / code_of / err_of | 46-79 | REAL | — | unwrap-avoidance helpers, used throughout |
| probe_anchors_on_the_header_guid_only | 81-93 | REAL | — | negative + short-buffer probe cases |
| minimal_audio_file_walks_to_one_pcm_track | 95-126 | REAL | — | exact sizes/counts/wfx/tsd |
| not_asf_bytes_refuse_typed | 128-140 | REAL | — | |
| truncated_head_is_truncation_not_not_asf | 142-158 | REAL | — | |
| header_size_below_floor_and_overrun_refuse_bad_object_size | 160-190 | REAL | — | two patch cases |
| header_above_memory_ceiling_refuses_typed | 192-206 | REAL | — | MAX_HEADER_BYTES+1 |
| broadcast_flags_refuse_typed | 208-225 | REAL | — | FPO flags patch |
| zero_channel_waveformatex_refuse_typed | 227-253 | REAL | — | channels=0, cbSize overrun |
| unknown_waveformatex_tags_pass_through_raw | 255-265 | REAL | — | tag 0x7100 |
| pcm_descriptor_maps_by_bits | 267-288 | REAL | S4 | F-003 comment lies about sub-case |
| declared_child_count_mismatch_is_damage_not_failure | 290-307 | REAL | — | |
| non_audio_video_stream_types_skip_with_notes | 309-328 | REAL | — | |
| unknown_header_children_skip_by_size | 330-363 | REAL | — | splice + size fixups |
| trait_order_and_post_start_lanes_are_exact | 365-397 | REAL | — | pre/post-start, idempotence |
| error_constructors_table | 399-443 | REAL | S2 | F-001 dead encrypted refusal row |
| payload_bytes / data_packet / object | 462-515 | REAL | S4 | F-005 builder mangling |
| ms_base / span_at | 517-526 | REAL | — | |
| packet_fields_parse_and_close_exactly | 528-552 | REAL | — | padding arithmetic |
| multi_payload_packets_parse_in_order | 554-574 | REAL | — | |
| malformed_packets_refuse_typed_never_guess | 576-606 | REAL | — | 4 mutation cases |
| fragments_reassemble_into_whole_objects | 608-651 | REAL | — | gap-abandon semantics |
| assembler_refusals_are_typed | 653-696 | REAL | — | incl. u32::MAX oversized refusal |
| walk_emits_whole_objects_with_preroll_subtracted_stamps | 698-750 | REAL | — | 150-100=50, 250-100=150 |
| payloads_for_undeclared_streams_are_skipped_not_fatal | 752-760 | REAL | — | |
| non_fixed_packet_sizes_refuse_typed | 762-780 | REAL | — | min-packet patch at 122 |
| simple_index_seek_reanchors_the_walk | 782-825 | REAL | — | bucket math + clamp |
| indexless_files_seek_by_bounded_linear_scan | 827-858 | REAL | — | |
| indexless_seeks_past_the_scan_budget_refuse_typed | 860-880 | REAL | — | 5000 packets vs 4096 budget |
| FixtureTruth/FIXTURES | 889-1065 | REAL | — | 10 fixtures, exact facts |
| fixture_dir / generate / read_file | 1083-1125 | REAL | — | in-tree first, loud skip |
| fixture_family_walks_match_the_measured_truth | 1127-1322 | REAL | S4 | F-004 weak seek assert |
| fixture_family_cross_checks_against_live_ffprobe | 1324-1426 | REAL | — | ran in baseline (2166) |
| broadcast_fixture_refuses_typed_where_ffprobe_refuses | 1428-1481 | REAL | — | ffprobe-refusal oracle |
| expected_sha | 1484-1499 | REAL | — | pins verified by sha256sum |
| sha256_16 | 1505-1578 | REAL | S4 | F-005 duplication twin |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all Layer-1/3 tests | YES | exact typed codes, byte offsets, arithmetic (e.g. preroll 150-100=50) |
| fixture_family_walks_match_the_measured_truth | YES | sha pins + exact object tree/facts; any walk regression fails |
| fixture_family_cross_checks_against_live_ffprobe | YES | live ffprobe packet-count oracle (ran in baseline) |
| error_constructors_table | NO for encrypted_stream | constructor mints code regardless of walk behavior (F-001) |
| fixture seek sub-lane | weakly | `ts.ticks >= 0` only (F-004) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Skips loudly when ffmpeg is absent" | tests.rs:16-17 | REAL | fixture_dir():1091-1093 eprintln + None |
| "§4.4 sha pins pin the COMMITTED bytes" | tests.rs:18-21 | REAL | sha256sum match on all 11 files |
| "byte-verified this session against the plan's §4.4 sha256 table" | tests.rs:11-13 | REAL | re-verified 2026-11-23 (this audit) |
| encrypted-stream refusal exists | tests.rs:412 | FALSE (as walk behavior) | no production call site (F-001) |

## 7. Dead / duplicated / unreachable code
- `error::encrypted_stream` — UNREACHABLE from any parser path (F-001).
- `sha256_16` — duplicated from tests/common/mod.rs:164 (documented reason).
- Nothing else dead; all helpers are used.

## 8. Dependencies & flags
Test-only module behind `#[cfg(test)]`. Uses in-crate `test_support` (pub mod,
shipped in the lib — see asf/test_support.rs doc "nothing in the demuxer
itself uses it": production surface carries test builders; noted here, owned
by the test_support audit). External binaries: ffmpeg/ffprobe, both present
in sandbox (/usr/bin/ffmpeg, /usr/bin/ffprobe).

## 9. Open questions for the next agent
- asf/test_support.rs (pub, shipped): how much builder code links into the
  release lib? Check dead-code elimination in the lumen-cli binary.
- asf/fuzz.rs sweeps only 2 templates × short mutations — does the
  fuzz/fuzz_targets/fuzz_demux_asf.rs nightly target actually run anywhere in
  CI? (fuzz/README.md "Lead-owned wiring" — who runs it?)
