# AUDIT: crates/demux/src/avi/tests.rs#part3 (lines 1251–1875)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 625 (of 3128; part3) / 12 tests + 5 helpers (plus 1 boundary test from part2, 1 straddling into part4) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — `#[cfg(test)]` module; executed via `cargo test` (baseline: `orchrun-test-workspace-nofailfast.log.txt:1489-1955` — demux 462 passed / 0 failed / 0 ignored) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "^    fn |^fn "` (item inventory); `sed -n` on every quoted range; `rg -n "Discontinuity" crates/demux/src/avi/*.rs` (never emitted — only matched in 2 drain helpers); `rg -n "fn span_at_nanos"` (:2264, part4 helper used by this range); baseline log grep for demux suite result |

## 1. One-paragraph verdict
This chunk is the AVI damage-containment test battery — rec-list clamps, unknown
chunks, PCM misalignment, idx1 truth/clamping/absolute-offset autodetection/
damage coalescing, typed error flavors, truncation boundaries, payload-prefix
delivery, and the W2-07 resync machinery — and it is the strongest test code I
have audited in this wave: every test builds real bytes with the hand-rolled
`build()` writer (real `mem_stream` SourceStream, no mock of the subject),
asserts exact damage reason strings, exact skipped-byte counts, exact payload
contents and event ORDER (e.g. `truncated_movi_payload_delivers_prefix_damage_then_eos`
pins all 6 events including two clamp records before the packets), and would
fail loudly under any mutation of the containment logic it exercises. The one
defect is an omission, not a lie: nothing here (or in the rest of the file per
the part-owners' ranges) tests a corrupted TOP-LEVEL chunk size or a sub-4-byte
`rec ` size — the exact gaps flagged as S3/S4 in mod.rs#part2.

## 2. What actually works (verified)
- All 12 tests in range executed and passed in the baseline:
  `orchrun-test-workspace-nofailfast.log.txt:1489` `running 462 tests` →
  `:1955 test result: ok. 462 passed; 0 failed; 0 ignored` (demux crate,
  release profile; none of these tests is `#[ignore]`d or feature-gated).
- Test infrastructure is subject-real: `demuxer_over` (:311-319) wraps the
  built bytes in `test_support::mem_stream` — a real in-memory
  `SourceStream`, not a demuxer mock; `build` (:203-309) emits honest
  RIFF/LIST/avih/strh/strf/idx1 bytes with size back-patching.
- `idx1_keyframe_flag_is_the_truth_when_present` (:1366-1400) is
  discriminating: it hand-builds an idx1 whose middle entry is non-keyframe
  and asserts `!packets[1].keyframe` — a `keyframe_for` that always returned
  `true` (the MJPEG/PCM default) fails it.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/avi/tests.rs:1289-1297 (representative of the range)
Code:
```rust
fn unknown_movi_chunks_skip_with_damage() {
    let built = build(&[video_stream()], &[(b"00dc", &b"v0"[..])], false);
    let mut bytes = built.bytes;
    // Splice a JUNK chunk between the movi form and the first packet
    // chunk: rebuild by locating movi form then inserting.
    let movi_form = built.movi_form_pos as usize + 4; // first chunk would start here
    let mut spliced = bytes[..movi_form].to_vec();
    chunk(&mut spliced, b"JUNK", &[0u8; 12]);
```
What is wrong: The suite's only JUNK-chunk test splices the JUNK *inside*
the movi list (after the form FourCC). No test in this range — and per the
byte-corruption test inventory none elsewhere in the file — corrupts a
TOP-LEVEL chunk size (a JUNK/unknown chunk between `hdrl` and `movi`
declaring a size that jumps past the movi), and no test builds a `rec `
list with declared size < 4. Those are precisely the untested paths where
mod.rs#part2 found real defects (F-001: top-level size lie is a fatal
`no_movi` with no recovery and no damage note; F-004: sub-4-byte rec
resumes 4 bytes past its extent uncounted).
Why it matters: The containment claims that ARE tested are pinned
byte-exactly, which makes the untested top-level containment gap invisible
to the suite — a regression or "fix" in the top-level walk cannot be caught
here. The `build()` writer plus `patch_size`/`smash_chunk_header` make both
tests ~15 lines each; the omission is a choice, not a limitation.
What correct looks like: Add (a) `top_level_junk_size_lie_recovers_or_counts`:
JUNK before movi with a size past the movi, asserting damage + movi still
found (or at minimum the fatal carrying a damage record), and (b)
`rec_list_smaller_than_form_type_resumes_at_extent`: rec size 0–3,
asserting the walk resumes at the declared extent with the strayed bytes
damage-counted.
Confidence: HIGH — Verification: read (full range + item inventory via rg;
mutation reasoning below); no run (baseline covers only the suite as-is).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `rec_list_overrunning_parent_bound_clamps_with_damage` (boundary: starts :1228 in part2, body 1251-1286 here) | 1228-1286 | REAL | — | rec +20-byte lie into idx1 region; asserts `Recovered{20,"rec-extends-past-list-bound"}` first, then payloads r0/r1/t0 in order. Mutation: drop the clamp → either fatal or wrong payload set → fails. |
| `unknown_movi_chunks_skip_with_damage` | 1289-1321 | REAL | — | JUNK inside movi; patches RIFF+movi sizes by the splice growth (arithmetic verified: movi LIST size field at `movi_form_pos-4`); asserts 12-byte skip + surviving packet. |
| `patch_size` (helper) | 1324-1333 | REAL | — | u32le add with wrapping_add — test-only, fine. |
| `pcm_misalignment_is_damage_not_silence` | 1336-1363 | REAL | — | 6-byte chunk vs block-align 4 → stray 2 damage, packet emitted, `damage_total()==2`. Pins the ladder's floor behavior. |
| `idx1_keyframe_flag_is_the_truth_when_present` | 1366-1400 | REAL | — | Discriminating (see §2); exercises `keyframe_for`'s idx1-truth over the codec-grammar default. |
| `lying_idx1_size_clamps_to_available_bytes` | 1403-1447 | REAL | — | 0xFFFFFFF0 lie → exact clamp arithmetic (`0xFFFFFFF0-48`), salvaged index still `Fast{indexed:true}` and 3 packets. Self-checks the idx1 position (`assert_eq!(..., b"idx1")`) before patching — layout-brittleness guarded. |
| `absolute_idx1_offsets_are_autodetected_and_seek` | 1450-1501 | REAL | — | Absolute dwOffset rewrite → variant note `idx1-absolute-offsets` with `skipped==0`, Fast seek, seek(40ms) lands tick 1. Cross-checks FFmpeg-behavior claim (ED-d). |
| `idx1_damage_coalesces_per_reason` | 1504-1561 | REAL | — | 1000 lying entries → exactly ONE event (regression pin for the "524k-event storm"), `damage_total()==16000`, movi walk still delivers. |
| `broken_magic_is_typed_not_avi` | 1568-1583 | REAL | — | Asserts code `demux.avi.not-avi` + `Unsupported{format: "avi"}` kind. |
| `truncated_head_is_typed_at_the_boundary` | 1586-1641 | REAL | — | Cuts 1/3 → not-avi; 4/5/11 → truncated{expected:12,found:cut}; 12/20/30/44 → truncated + stage Demux + sticky replay same code. Boundary-exact. |
| `truncated_movi_payload_delivers_prefix_damage_then_eos` | 1644-1707 | REAL | — | Pins the FULL 6-event order with exact byte counts (riff clamp, movi clamp, packet:2, recovered:60, packet:40, eos) + EOS idempotency ×3. The strongest ordering test in the range. |
| `cut_exactly_at_a_chunk_payload_start_is_damage_then_eos` | 1710-1743 | REAL | — | Zero-payload cut: whole 100-byte payload counted, 1 packet survives, `recovered >= 3` (loose but documented: "riff + movi clamps and the chunk clamp"). |
| `chunk_size_past_movi_bound_clamps_and_continues` | 1746-1804 | REAL | — | Mid-movi +40 lie into idx1 region: clamped packet delivers exactly `in_movi` bytes, `clamp_damage==40`, terminal eos. Exercises mod.rs:566-573's clamp-to-bound path. |
| `smash_chunk_header` (helper) | 1815-1819 | REAL | — | Zeroed fourcc + garbage size — the W1-03 traced byte shape. |
| `DamageLedger` type + `drain_with_damage` (helper) | 1823-1845 | REAL | — | Captures (skipped, reason) pairs IN ORDER + payloads + typed terminal. `Ok(Discontinuity) => {}` arm is dead today (AVI demuxer never emits Discontinuity — rg over avi/*.rs: only these two match sites) but would silently hide one if ever emitted — noted, not a finding. |
| `resync_body` (helper) | 1851-1867 | REAL | — | Two-stream six-chunk body, damages chunk #2 — exactly the unclamped-jump-then-valid-tail shape. |
| `garbage_chunk_jump_past_bound_resyncs_via_idx1` (straddles boundary: opens :1870-1875 here, assertions :1876-1904 in part4/A06) | 1870-1904 | REAL | — | Opening audited here: traced garbage size 0x077A_0600, `resync_body` setup. The assertions (exact damage triple + recovered tail of 5 payloads) live in part4's range — noted for A06; read for context, they pin the resync ladder byte-exactly. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| rec_list_overrunning… | YES | Fatal-or-wrong-payloads under any clamp regression. |
| unknown_movi_chunks… | YES | Damage record (12, reason) + packet content both asserted. |
| pcm_misalignment… | YES | Exact stray count + damage_total. |
| idx1_keyframe_flag… | YES | The discriminating mutation (always-true keyframe) fails `!packets[1].keyframe`. |
| lying_idx1_size… | YES | Exact clamp arithmetic + seekability + packet count. |
| absolute_idx1_offsets… | YES | Variant note + Fast + landing tick all asserted. |
| idx1_damage_coalesces… | YES | Exactly-one-event and total-16000 both pinned. |
| broken_magic / truncated_head | YES | Error code/kind/detail/stage/sticky all asserted per cut. |
| truncated_movi_payload… | YES | Full 6-event sequence + counts. |
| cut_exactly_at… | YES | Payload-loss count + survivor count. |
| chunk_size_past_movi_bound… | YES | Clamped length + damage==40 + terminal. |
| garbage_chunk_jump… (boundary) | YES (per assertions read in part4 range) | Damage triple + 5-payload recovery pinned. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| container-avi evidence: "card 2.B2: class-B AC matrix, seek-to-time, 43-test unit suite + fuzz harness" | docs/feature-matrix.toml:203 | REAL (suite) / stale count | Suite exists and is far larger than "43-test" — 85 `test avi::` entries in the baseline run (:1489-1955); this range alone contributes 12. Evidence string undercounts, i.e. drifted, not inflated. |
| mod.rs module doc: "the generated corpus and its expectation ledgers are the conformance truth" | crates/demux/src/avi/mod.rs:11-13 | SHALLOW | The corpus is generated by the repo's own writer (tools/xtask fixtures.rs) and this unit suite re-implements the same writer — writer and parser share one brain. The idx1 absolute-offset test honestly cites FFmpeg behavior (ED-d) as the external anchor, but no external AVI corpus (e.g. ffmpeg-generated files with rec lists, OpenDML, real-world JUNK) is in the loop. |

## 7. Dead / duplicated / unreachable code
- `drain_with_damage`'s `Discontinuity` arm (:1839) and its twin at :779 are
  dead (AVI demuxer never emits `DemuxEvent::Discontinuity`) — defensive,
  harmless.
- No duplication found in range; helpers are shared, not copy-pasted.

## 8. Dependencies & flags
Test-only: `use super::test_support` (in-mem SourceStream), `use super::*`,
`crate::error`, media_types, plugin_api. No feature gates, no unsafe, no
`#[ignore]`, no network/fs dependencies (bytes built in-memory). Baseline
confirms 0 ignored in the demux suite.

## 9. Open questions for the next agent
- part4 (A06): the resync test bodies (1912+ `resyncs_by_scan_without_idx1`,
  1956 `without_landing_is_typed_never_silent_eos`) — verify they pin the
  scan-budget terminal (`walk_resync_failed`) and the `RESYNC_SCAN_BUDGET`
  boundary as tightly as the idx1 rung test does.
- Whoever owns mod.rs#part1: the `MAX_LIST_DEPTH` overflow test — is a
  9-deep rec nesting test present anywhere? (Not in this range.)
