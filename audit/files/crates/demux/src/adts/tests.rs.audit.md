# AUDIT: crates/demux/src/adts/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 643 / 36 (29 tests + 7 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test code: cfg(test) only. Subject (`AdtsDemuxer`) YES (call chain: `apps/cli/src/demux_host.rs:458` `crate::containers::open` → `apps/cli/src/containers/mod.rs:52` `"adts" => adts::open` → `apps/cli/src/containers/adts.rs:66` `AdtsDemuxer::new(stream)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "adts" crates/demux/src/lib.rs` (module pub, line 43); `rg -n "AdtsDemuxer" crates/ apps/` (callers: cli container host, streaming test, es::aac re-export); `rg -n "mod tests" crates/demux/src/adts/mod.rs` → `#[cfg(test)] mod tests;` (mod.rs:108-109); `rg -n "adts" docs/feature-matrix.toml` (row `container-adts-aac`); baseline log greps (below); `sed -n`/`od -c` verification of every quote. Note: one terminal-render glitch showed mod.rs:146 as `#ust_use]`; `od -c` proved the real bytes are `#[must_use]` — no corruption. |

## 1. One-paragraph verdict
This is a genuine, executed unit suite: all 29 tests ran green in the baseline
(`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1494-1522`, every
`adts::tests::*` name `ok`, inside the demux lib run of 462 tests at log lines
1489-1494), and the subject demuxer is independently corroborated against
ffmpeg-measured truth by the corpus suite in the same log
(`tests/adts_corpus.rs` at log lines 1957-1963: `corpus_adts_lanes_match_the_ffmpeg_truth ... ok`,
4/4 passed — corpus present, not skipped). Tests assert exact timestamps,
durations, byte-level payload equality (ADTS-vs-ADIF differential), pool memory
ceilings, sticky typed errors, and seek/reanchor landings — mutation-sensitive
throughout. Two test-honesty defects: the fuzz-lite test's name promises
typed-or-recover behavior its body cannot observe, and the rate-table pin
covers only 6 of 13 entries anywhere in this suite.

## 2. What actually works (verified)
— All 29 tests pass: `orchrun-test-workspace-nofailfast.log.txt:1494-1522` (suite
totals 4534/7/49 per footer; none of the 7 failures in crates/demux).
— Corpus differential executes and passes: same log lines 1957-1963.
— Subject reachable from shipped `lumen` CLI: `apps/cli/src/containers/mod.rs:52`
   `"adts" => adts::open(stream, opts)`; `apps/cli/src/containers/adts.rs:66`
   `AdtsDemuxer::new(stream)`; `apps/cli/src/demux_host.rs:458` calls the registry.
— Pool/counter observables are real state reads, not fakes:
   `crates/demux/src/adts/mod.rs:447-449` `pool_bytes()` → `self.pool.pool_bytes()`.

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/adts/tests.rs:629-643
Code:
```
#[test]
fn committed_mutation_fixtures_are_typed_or_recover_never_panics() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/adts");
    for name in [
        "0059-trunc-4k.bin",
```
What is wrong: The name asserts the committed mutation fixtures "are typed or
recover" and "never panic". The body asserts only that each file exists and is
non-empty (`assert!(!bytes.is_empty(), "{name} exists")`, line 640); the entire
behavioral half is delegated to `super::fuzz::fuzz_demux_adts(&bytes)` (line
641), whose own contract is absence-of-panic only — `crates/demux/src/adts/fuzz.rs:59-74`
swallows every outcome (`Ok(_) => {}`, `Err(_) => break`) and asserts nothing
about packet counts, timestamps, or error codes.
Why it matters: a regression that silently drops every frame of these four
mutated fixtures, or mis-stamps them, still passes this test green. The
"typed" claim in the test name is unverified evidence. (Partially mitigated:
`tests/aac_fuzz_survival.rs::mutated_fixtures_never_panic_and_settle_typed` in
the same baseline log lines 768-772 does assert settled typed errors — but that
is the codecs-crate AAC decoder suite, not this demuxer.)
What correct looks like: assert a concrete outcome per fixture — at minimum the
terminal error code or recovered-event count, as the sibling typed-error tests
in this same file do (e.g. line 218 `assert_eq!(out.terminal.as_deref(), Some("demux.adts.truncated"))`).
Confidence: HIGH — Verification: read (tests.rs + fuzz.rs traced; baseline run cited).

### F-002 [S4]
Where: crates/demux/src/adts/tests.rs:92-98
Code:
```
#[test]
fn rate_table_matches_the_spec() {
    assert_eq!(SAMPLE_RATES[0], 96_000);
    assert_eq!(SAMPLE_RATES[1], 88_200);
    assert_eq!(SAMPLE_RATES[11], 8_000);
    assert_eq!(SAMPLE_RATES[12], 7_350);
}
```
What is wrong: Pins 4 of 13 table entries by index. Entries at indices 2
(64_000), 5 (32_000), 6 (24_000), 7 (22_050), 8 (16_000), 9 (12_000), 10
(11_025) are not asserted by any test in this file (indices 0, 4 appear in
`header_parse_roundtrips_the_field_set`; 3 and 4 in the ADIF/walk tests; 1 only
here; corpus lanes pin 88_200/96_000 against ffmpeg — `tests/adts_corpus.rs:89,98`).
Why it matters: a typo in `SAMPLE_RATES[7]` (22_050) would demux every 22.05 kHz
file with wrong timestamps and pass the entire suite. The table lives at
`crates/demux/src/adts/mod.rs:139-142` and is the single source of rate truth.
What correct looks like: one loop asserting all 13 literals (a 13-entry expected
array compared wholesale).
Confidence: HIGH — Verification: read (table cross-checked at mod.rs:139-142; index usage traced across this file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Drain` (fields) | 16-21 | REAL | — | Local result collector; plain data. |
| `fn drain` | 23-43 | REAL | — | Loops `next_event` to EOS/Err; no recursion; streams are in-memory and bounded by construction; `.context.code.clone()` on terminal. |
| `fn open` | 45-49 | REAL | — | new + start; `.must()` panics on error — appropriate in tests. |
| `header_parse_roundtrips_the_field_set` | 53-81 | REAL | — | Field-by-field bit extraction pin; covers MPEG-4 shape, 9-byte protected header, samples()/descriptor(). |
| `probe_detects_adts_adif_and_rejects_the_rest` | 83-90 | REAL | — | 5 probe cases; probe is 4 lines (mod.rs:147-152); does not pin the ID3-prefixed probe path (see §9). |
| `rate_table_matches_the_spec` | 92-98 | REAL | S4 | See F-002: 4/13 indices. |
| `clean_walk_stamps_frame_arithmetic` | 102-146 | REAL | — | Strongest test: exact pts series [0,1024,2048,4096] over blocks [1,1,2,1], base (1,44100), per-packet ns durations, keyframe flags, `frames_demuxed()==4`, end duration 116_099_773 ns (=5120/44100 s, arithmetic verified). |
| `id3_head_and_interframe_tags_are_skipped` | 148-160 | REAL | — | Head + mid-stream ID3v2 tags; `id3_skipped()==310+74` exact byte accounting. |
| `pool_memory_stays_bounded` | 162-175 | REAL | — | 64 frames vs 2-slot pool; asserts `pool_bytes() <= 2*MAX_FRAME_BYTES` (8191+9, mod.rs:117) and >0. |
| `lost_sync_mid_stream_resyncs_with_recovered_events` | 179-206 | REAL | — | Smashed syncword on f2; 1 Recovered with exact skip bytes, discontinuity flag, hole preserved in pts (1024, not smoothed). |
| `truncated_frame_is_the_typed_truncation` | 208-225 | REAL | — | Sticky error replays on second `next_event` — pins stickiness, not just first error. |
| `truncated_first_frame_is_the_typed_truncation_at_start` | 227-234 | REAL | — | `start()` itself errors; header intact, payload cut. |
| `sampling_index_fifteen_is_typed` | 236-242 | REAL | — | Reserved index 15 → `demux.adts.bad-sampling-index`. |
| `channel_config_zero_is_typed` | 244-250 | REAL | — | channel_config 0 → `demux.adts.bad-channel-config`. |
| `crc_mismatch_is_typed_and_good_crc_walks` | 252-265 | REAL | — | Forged bad CRC 0xBEEF refused; good CRC walks 109-byte packet (9-byte header + 100). |
| `frame_shorter_than_its_header_is_typed_bad_header` | 267-278 | REAL | — | flen forced to 6 (<7-byte header) → `demux.adts.bad-header`. |
| `garbage_head_without_sync_is_the_typed_not_adts` | 280-291 | REAL | — | 512 zero bytes and ID3-then-garbage both `demux.adts.not-adts`. |
| `nonzero_layer_head_is_not_adts` | 293-302 | REAL | — | Layer bits nonzero → not-adts (layer check lives in `start`, not `probe` — pinned here). |
| `trailing_fragment_and_lost_sync_at_end_are_clean` | 304-323 | REAL | — | 3-byte tail and 128-byte garbage tail: 1 packet + 1 Recovered with exact byte counts, no terminal error. |
| `not_started_is_typed` | 325-336 | REAL | — | `next_event` and `reanchor` before `start` → `demux.adts.not-started`. |
| `mid_stream_parameter_change_is_a_discontinuity` | 338-353 | REAL | — | channels 2→1 mid-stream: Discontinuity event + packet flag; clock keeps emitted-sample arithmetic (pts 1024). |
| `adif_cbr_walk_matches_the_block_arithmetic` | 357-391 | REAL | — | 7 blocks; header size derived-and-asserted (14 B); per-byte ramp payload equality; duration 149_333_333 ns (=7168/48000 s, verified). |
| `adts_and_adif_agree_on_the_same_payload` | 393-426 | REAL | — | Cross-container differential: same essence through both framings, byte-equal. Shared-author risk (builder and parser both in-repo) mitigated by the ffmpeg corpus suite. |
| `struct Bits` + `push` + `bytes` | 428-447 | REAL | — | Hand-rolled MSB-first bit writer; correct; no unsafe. |
| `fn adif_raw` | 449-481 | REAL | — | Malformed-ADIF builder (VBR, multi-PCE, bad params, no data). `0..=num_pce` writes num_pce+1 PCEs — consistent with the multi-program refusal at num_pce=1. |
| `adif_vbr_is_the_typed_refusal` | 483-489 | REAL | — | `demux.adts.adif-vbr`. |
| `adif_multi_program_is_the_typed_refusal` | 491-497 | REAL | — | `demux.adts.adif-multi-program`. |
| `adif_bad_params_are_typed` | 499-507 | REAL | — | 3-way loop: index 15, front 0, bitrate 0 → `demux.adts.adif-bad-params`. |
| `adif_header_without_data_is_typed` | 509-516 | REAL | — | Header-only → `demux.adts.adif-no-data`. |
| `adif_partial_final_block_is_marked` | 518-531 | REAL | — | Truncated-by-40 CBR stream: 60-byte final packet, discontinuity-marked, no terminal error. |
| `adif_seek_is_format_inherent_arithmetic` | 533-564 | REAL | — | SeekSupport::Fast{indexed:false}; seek to 3*1024 lands exactly; reanchor emits Discontinuity then the packet at 3*1024. |
| `adts_seek_roundtrip_lands_at_or_before_the_target` | 568-605 | REAL | — | SlowLinear support; mid-frame target (5*1024+512) clamps to frame 5; post-seek remaining count 16-5-1. |
| `adts_seek_past_the_end_lands_at_the_last_frame` | 607-625 | REAL | — | Past-end clamps to last (only) frame, pts 0. |
| `committed_mutation_fixtures_are_typed_or_recover_never_panics` | 629-643 | SHALLOW | S3 | See F-001: name overpromises; body is existence + no-panic sweep. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| clean_walk_stamps_frame_arithmetic | YES | Mutation: pts = frame_index*1024 ignoring blocks → packet 3 gets 3072 ≠ 4096 (lines 120/125). Mutation: duration unit confusion → line 134 fails. |
| lost_sync_mid_stream_resyncs | YES | Mutation: smooth the hole (emit pts 2048 for f3) → line 203 fails; mutation: no Recovered event → line 195 fails. |
| truncated_frame_is_the_typed_truncation | YES | Sticky-replay assert (lines 220-224) fails if error is non-sticky; code assert (218) fails on wrong code. |
| pool_memory_stays_bounded | YES | Per-packet allocation → pool_bytes grows past 2*MAX_FRAME_BYTES (line 173). Getter is a real state read (mod.rs:447-449). |
| adif_cbr_walk_matches_the_block_arithmetic | YES | Per-byte ramp equality (382-384) fails on any block-boundary drift; duration assert (387-390) pins the sample clock. |
| adts_and_adif_agree_on_the_same_payload | YES (with caveat) | Byte-equality across framings fails on any framing bug; caveat: builder is same-author in-repo (shared-bug risk; corpus differential is the external anchor). |
| adif_seek / adts_seek roundtrips | YES | Exact landing ticks and remaining-packet counts (599-604). |
| committed_mutation_fixtures… | NO (for the "typed" claim) | Body cannot observe typed-or-recover (F-001); only a panic would fail it. |
| rate_table_matches_the_spec | PARTIAL | Only 4/13 entries (F-002). |
| All 12 typed-error-path tests (209-336, 483-516) | YES | Exact `err.context.code` string equality per path. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| `container-adts-aac` status windows/linux/macos = implemented | docs/feature-matrix.toml:272-282 | REAL (linux) / REAL-UNPROVEN (win/macos) | Linux: baseline nofailfast log 1494-1522 (29/29 unit) + 1957-1963 (corpus 4/4). Windows/macOS: no run executes this crate there — orchrun-platform-macos-crosscheck.log.txt covers only platform-macos targets, not demux. |
| "0059/0110 lanes PASS, dirty-lane 350-vs-351 divergence recorded" | feature-matrix.toml:281 evidence | REAL | `corpus_adts_lanes_match_the_ffmpeg_truth ... ok` + `corpus_dirty_tail_garbage_is_honest ... ok`, baseline log 1960-1962. |
| Module doc: "damage policy… bounded resync, ID3 skip" | crates/demux/src/adts/mod.rs:44-58 | REAL | `id3_head_and_interframe_tags_are_skipped` + `lost_sync_mid_stream…` pass (baseline log 1510-1511). |

## 7. Dead / duplicated / unreachable code
None in this file. (`Bits`/`adif_raw` are local to the ADIF malformed-lane
tests; the `Bits` writer duplicates `test_support::adif_stream`'s closure-based
pusher — same-author duplication, noted, not defect-grade.)

## 8. Dependencies & flags
No feature flags gate this file (`#[cfg(test)] mod tests;` at mod.rs:108-109).
Baseline ran it in the default `cargo test --workspace` selection (release
profile deviation recorded in INDEX.md footnote). No unsafe, no external deps
(demux crate is std-only per lib.rs:37-38).

## 9. Open questions for the next agent
- Who generates `crates/demux/fixtures/adts/0059-*.bin`? If xtask-generated,
  confirm `cargo xtask fixtures --check` (baseline PASS, INDEX.md row 12)
  covers them — a drift between generator and committed bytes would be invisible.
- `probe` comment (mod.rs:145) says "the registry sniffs past the tag at open"
  for ID3-prefixed streams — no test in this file pins that path
  (`garbage_head_without_sync…` covers ID3-then-garbage refusing, not
  ID3-then-valid-frame acceptance). Worth a row in the open-path wiring audit.
