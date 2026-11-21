# AUDIT: crates/demux/src/asf/tests.rs (part 1 of 3 — lines 1-526)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 526 (of 1578) / 26 (5 helpers, 1 const, 15 tests, 6 vector builders) |
| File verdict | REAL (test layer; chunk covers layer-1 built-bytes tests + the layer-3 packet builders) |
| Reachable from shipped entry point? | N/A (cfg(test)) — executed: baseline `cargo test --workspace` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
Part 1 is the honest half of a real test suite: fifteen built-bytes tests drive the
actual parser through `mem_stream` (real `SourceReader`, real byte layouts, patched
fields at computed offsets) and assert typed error codes, damage-note names, track
facts, and trait-order lanes — every one of them would fail under a concrete mutation
of the code they target. The layer-3 packet builders at the chunk's end (P,
payload_bytes, data_packet, object) are real encoders of the ffmpeg-verified packet
grammar, used by parts 2/3. Defects are hygiene: the error-table test keeps two dead
error constructors alive and thereby misdescribes them as codes "this unit mints";
half of the "raw passthrough" claim in pcm_descriptor_maps_by_bits is unpinned because
the 24-bit passthrough vector asserts a string the registry loop already produces; and
the payload builder silently clamps out-of-range stream numbers instead of failing.

## 2. What actually works (verified)
- All 15 part-1 tests pass in the baseline: `sed -n '1489,1626p'
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → every
  `asf::tests::*` line `ok` (suite totals 4534/7/49; no demux failures).
- The fixtures the module docs promise exist: `ls crates/demux/src/asf/fixtures/` →
  10 files (asf_mjpeg.asf, wma_wmav1_st44k.wma, asf_stream_broadcast.wmv, …).
- No mocks of the subject: every test builds real ASF bytes (GUID constants, LE field
  patches) and runs the real `AsfDemuxer` (tests.rs:53-58 `started`).
- House no-unwrap rule kept: `unreachable!("{e}")` pattern throughout (46-51, 74-79).

## 3. Findings

### F-001 [S4] error_constructors_table keeps two dead constructors alive and labels them "codes this unit mints"
Where: crates/demux/src/asf/tests.rs:399-420
Code:
```rust
fn error_constructors_table() {
    // The codes this unit mints (the wav error-table convention).
    let cases = [
        (error::not_asf(b"xx"), "demux.asf.not-asf"),
```
and (tests.rs:416-420)
```rust
        (
            error::packets_not_implemented(),
            "demux.asf.packets-not-implemented",
        ),
        (error::seek_unsupported(), "demux.asf.seek.unsupported"),
```
What is wrong: `error::packets_not_implemented` (error.rs:193) and
`error::seek_unsupported` (error.rs:206) have zero production call sites — their only
callers are these two test rows (rg across crates/demux/src: tests.rs:417 and :420
only). The table's comment calls all 20 rows "the codes this unit mints"; the unit
mints 18. mod.rs:55-57 correctly says of `seek.unsupported` "no site mints it anymore"
— the test contradicts the production reality by exercising the dead constructor.
Why it matters: The dead constructors survive the dead-code lint only because the test
calls them; a reader of the table believes both are live error lanes. This is how
stub-era refusals (`packets-not-implemented` is the pre-ASF-b refusal) linger as
apparently-live taxonomy.
What correct looks like: Delete the two constructors and their rows (the mod.rs doc
already records the history), or move them to a documented "retired codes" list the
table labels as such.
Confidence: HIGH — Verification: rg "seek_unsupported|packets_not_implemented"
crates/demux/src/ → only error.rs definitions + the two test rows; sed -n '399,420p'
(quote verified).

### F-002 [S4] pcm_descriptor_maps_by_bits' "raw passthrough" claim is only half-pinned
Where: crates/demux/src/asf/tests.rs:280-287
Code:
```rust
    // Integer PCM at a width the registry does not carry: raw
    // passthrough, never a silent nearest-guess.
    let wfx = waveformatex(1, 1, 8_000, 3, 24);
    let mut asf = started(minimal_asf_audio(&wfx));
    assert_eq!(ok(asf.start())[0].codec.as_str(), "pcm_s24le");
```
What is wrong: The 24-bit "passthrough" vector asserts `"pcm_s24le"` — the identical
string the registry loop above (tests.rs:269-274, `(24, "pcm_s24le")`) already
asserts for the mapped case. For 24 bits the test cannot distinguish registry mapping
from formatted passthrough; only the 48-bit vector (:285-287, `"pcm_s48le"`, not in
the loop) actually pins the passthrough behavior the comment describes.
Why it matters: If the passthrough lane regressed to nearest-guess for widths the
registry carries, the 24-bit vector stays green — the comment claims coverage the
assert does not provide.
What correct looks like: Drop the redundant 24-bit vector or change it to a width
absent from the loop (e.g. 20 bits → `"pcm_s20le"`).
Confidence: HIGH — Verification: sed -n '267,288p' (both blocks read); comparison of
asserted strings.

### F-003 [S4] payload_bytes silently clamps stream numbers above 127
Where: crates/demux/src/asf/tests.rs:462-464
Code:
```rust
fn payload_bytes(p: &P) -> Vec<u8> {
    let mut b = Vec::new();
    b.push((if p.keyframe { 0x80 } else { 0x00 }) | u8::try_from(p.stream).unwrap_or(0x7F));
```
What is wrong: A `P` with `stream > 127` (a builder bug or a future test for the
skip-undeclared-stream lane) is silently encoded as stream 0x7F instead of failing the
build. The ASF stream field is 7 bits; the builder should refuse.
Why it matters: A test author intending stream 200 gets stream 127 — the test then
asserts against the wrong lane and can pass for the wrong reason.
What correct looks like: `assert!(p.stream <= 0x7F)` (test code — panicking on builder
misuse is correct here; the house no-panic rule is for production code).
Confidence: HIGH — Verification: sed -n '462,478p' (quote verified); no part-1 test
uses a stream > 2 today.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ok` | 46-51 | REAL | — | Test helper, no-unwrap pattern. |
| `started` | 53-58 | REAL | — | Real demuxer over real bytes; discards start's tracks (re-called idempotently by tests). |
| `code_of` | 60-62 | REAL | — | Trivial. |
| `PCM16_WFX` const | 68-71 | REAL | — | 18-byte WAVEFORMATEX PCM16 vector, reused throughout. |
| `err_of` | 74-79 | REAL | — | Expect-Err helper. |
| `probe_anchors_on_the_header_guid_only` | 81-93 | REAL | — | 4 positive/negative probe lanes + EXTENSIONS pin (the const's only consumer — see mod.rs F-007). |
| `minimal_audio_file_walks_to_one_pcm_track` | 95-126 | REAL | — | Deep asserts: children counts, FPO facts, wfx fields, TSD bytes, pcm_fact — strong test. |
| `not_asf_bytes_refuse_typed` | 128-140 | REAL | — | Pins `demux.asf.not-asf`. |
| `truncated_head_is_truncation_not_not_asf` | 142-158 | REAL | — | Pins truncation-vs-not-asf discrimination. |
| `header_size_below_floor_and_overrun_refuse_bad_object_size` | 160-190 | REAL | — | Two lanes: below-floor → bad-object-size; overrun → truncated. |
| `header_above_memory_ceiling_refuses_typed` | 192-206 | REAL | — | Pins MAX_HEADER_BYTES ceiling refusal. |
| `broadcast_flags_refuse_typed` | 208-225 | REAL | — | Patches FPO flags bit 0 → broadcast-unsupported. |
| `zero_channel_waveformatex_refuse_typed` | 227-253 | REAL | — | channels=0 and cbSize-overrun lanes. |
| `unknown_waveformatex_tags_pass_through_raw` | 255-265 | REAL | — | Tag 0x7100 → `asf-audio-tag-7100`, pcm_fact None. |
| `pcm_descriptor_maps_by_bits` | 267-288 | REAL | S4 | F-002: half the passthrough claim unpinned. |
| `declared_child_count_mismatch_is_damage_not_failure` | 290-307 | REAL | — | Damage-note name pinned (`declared-child-count-mismatch`). |
| `non_audio_video_stream_types_skip_with_notes` | 309-328 | REAL | — | Command-stream GUID → no-streams refusal. |
| `unknown_header_children_skip_by_size` | 330-363 | REAL | — | Splice + header-size repair; asserts walked=3, streams=1, rendered GUID, damage note — strong. |
| `trait_order_and_post_start_lanes_are_exact` | 365-397 | REAL | — | 5 lanes: not-started ×2, seekability None, EOS ×3, SlowLinear, seek-UNKNOWN→0, idempotent start. Note: does NOT pin the Fast-claimed-but-index-unparseable lane (mod.rs F-005 stays unpinned — no part-1 vector for an oversized/corrupt Simple Index). |
| `error_constructors_table` | 399-443 | REAL | S4 | F-001: 20 rows, 2 of them dead constructors. |
| `P` struct | 450-457 | REAL | — | Vector spec. |
| `payload_bytes` | 462-478 | REAL | S4 | F-003 stream clamp. Encodes the `5d` property-flags form correctly (replicated 8-byte / 0, offset dword, MO byte, length word). |
| `data_packet` | 484-502 | REAL | — | Builds exact-closing fixed-size packets; `total - b.len()` underflow is caught by the padding assert (fails loudly, verified by reading). |
| `object` | 506-515 | REAL | — | Completed-object spec convenience. |
| `ms_base` / `span_at` | 517-526 | REAL | — | Timing helpers for parts 2/3. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| probe_anchors_on_the_header_guid_only | YES | 4 direct asserts incl. the broadcast-prefix negative. |
| minimal_audio_file_walks_to_one_pcm_track | YES | 15+ field-level asserts on real parsed output. |
| not_asf_bytes_refuse_typed | YES | Error code pinned. |
| truncated_head_is_truncation_not_not_asf | YES | Distinguishes two codes. |
| header_size_below_floor_and_overrun | YES | Both codes pinned via byte patches. |
| header_above_memory_ceiling_refuses_typed | YES | MAX_HEADER_BYTES+1 → header-too-large. |
| broadcast_flags_refuse_typed | YES | Flag patch → broadcast-unsupported. |
| zero_channel_waveformatex_refuse_typed | YES | Two refusal lanes. |
| unknown_waveformatex_tags_pass_through_raw | YES | Passthrough name + pcm_fact None. |
| pcm_descriptor_maps_by_bits | YES (mapped lanes) / HALF (passthrough) | F-002. |
| declared_child_count_mismatch | YES | Damage event string pinned. |
| non_audio_video_stream_types_skip_with_notes | YES | no-streams code. |
| unknown_header_children_skip_by_size | YES | 4 asserts incl. rendered GUID string. |
| trait_order_and_post_start_lanes_are_exact | YES | Covers 5 distinct state-machine lanes. |
| error_constructors_table | YES for code strings | Pins string constants + Demux stage; cannot catch missing mint sites (that's rg work, done in mod.rs F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "36-test asf unit suite" | docs/feature-matrix.toml:216 (row container-asf-wmv) | REAL (understated) | Baseline lists 40 asf test lines (28 tests:: + guid + error + fuzz); all ok. |
| "byte-verified this session against the plan's §4.4 sha256 table" (fixtures) | tests.rs:11-13 | UNVERIFIED | sha256 table lives in docs/plans (not my chunk); fixtures exist in-tree; the verification run is not in the baseline logs. |
| Layer-1 list "(not-asf, truncation, object sizes, the broadcast gate, no-streams, WAVEFORMATEX / video-TSD validation, the non-A/V stream skip, the declared-count mismatch note)" | tests.rs:5-9 | REAL | All seven lanes present in part 1 (128-328). |
| "Skips loudly when ffmpeg is absent" | tests.rs:16-17 | UNVERIFIED (part 3) | Skip mechanism lives with the ffprobe tests at :1325+ — part 3's chunk (A07). |

## 7. Dead / duplicated / unreachable code
- F-001: the two retired constructors kept alive by the table.
- pcm_descriptor_maps_by_bits' 24-bit passthrough vector duplicates the loop's
  coverage (F-002).
- No unreachable test helpers in this chunk; every helper has callers within the file.

## 8. Dependencies & flags
- cfg(test) module; no feature gates; std + workspace crates only; `std::process::Command`
  imported (tests.rs:30) for part 3's ffprobe lane (not used in part 1 — noted, not a
  finding).

## 9. Open questions for the next agent
- Part 2/3 (A06/A07): do any vectors pin `seekability()==Fast` with an index that then
  fails to parse (mod.rs F-005)? Part 1 does not.
- Does any part-2/3 test exercise `payload_oversized` and `packet.bad-length` against
  the builders defined here (they are the consumers of payload_bytes/data_packet)?
