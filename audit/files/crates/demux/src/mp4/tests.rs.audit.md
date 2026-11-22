# AUDIT: crates/demux/src/mp4/tests.rs

| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-22 |
| Lines / functions | 1111 / 46 (8 helpers, 36 `#[test]`s, 2 builder fns used only by tests) |
| File verdict | REAL (strong conformance suite with one harness-level honesty gap) |
| Reachable from shipped entry point? | TEST-ONLY by design: `#[cfg(test)] mod tests;` (mp4/mod.rs:103-104) → runs under `cargo test -p demux`. Never compiled into release binaries. Not reachable from lumen CLI / lumen-tauri — correct for a test module. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 5 |
| Commands run | rg searches for module wiring, `pub mod fuzz`/`test_support` consumers, `fuzz_demux_mp4` callers; sed quote verification (all quotes below verified with `sed -n`). No cargo/node available (adaptation) → per-test pass status UNVERIFIED; suite-level status cites W00 baseline. |

## 1. One-paragraph verdict

This is the MP4/MOV conformance suite: 36 tests over hand-built in-memory byte fixtures (`test_support::progressive/fragmented/...`), asserting exact packet counts, DTS/PTS ladders (stts/ctts/elst/tfdt), stss keyframe flags, verbatim payload bytes, typed error codes for every malformed-input shape, chunk-table ceilings (AUD-01), and pcmC (ISO/IEC 23003-5) mapping. The fixtures are written by an independent hand-rolled box writer (mp4/test_support.rs) — no circular golden, no mock-of-the-subject; assertions are on real parser output. The one structural defect: the shared `drain()` helper conflates `Err` with end-of-stream, so the entire happy-path family cannot detect a demuxer that errors instead of ending cleanly. Otherwise this is one of the most honest test files in the tree.

## 2. What actually works (verified)

- Verified statically (read + rg): fixtures built by independent writer (`mp4/test_support.rs` builds raw box bytes; zero references to `Mp4Demuxer` inside it), so round-trips exercise the real parser, not a mirror of it.
- Verified statically: 21+ tests assert exact packet counts and exact payload bytes (e.g. `assert_eq!(&packets[3].data[..], &[0x13; 4]);`), so stsz/stco/stts/stss/ctts/elst regressions fail loudly.
- W00 baseline: workspace tests 1751 pass / 2 fail (first-vertical golden drift, UI) / 25 ignored → this suite expected green; per-test pass status UNVERIFIED (cargo not runnable in this audit environment).

## 3. Findings

### F-001 [S2] `drain()` swallows `Err` as if it were end-of-stream; no happy-path test can detect "errored instead of EOS"
Where: crates/demux/src/mp4/tests.rs:57-70
Code:
```rust
fn drain(demuxer: &mut Mp4Demuxer) -> (usize, Vec<media_types::Packet>) {
    let mut packets = Vec::new();
    let mut events = 0;
    while let Ok(event) = demuxer.next_event() {
        events += 1;
        match event {
            DemuxEvent::Packet(p) => packets.push(p),
            DemuxEvent::EndOfStream => break,
            _ => {}
        }
```
What is wrong: `while let Ok(event)` exits silently on `Err`. To every one of the ~21 `drain()` call sites, a mid-stream typed error is indistinguishable from clean `EndOfStream`. The returned `events` counter that could distinguish them is discarded by every caller (`let (_, packets) = drain(&mut mp4);` — tests.rs:128, 161, 174, 191, 204, 219, 239, 264, 294, 318, 393, 439, 467, 532, 562, 668, 715, 737, 761, 867, 878, 892, 1064).
Why it matters: Mutation analysis (reasoned, not executed — UNVERIFIED by run): replace the demuxer's `EndOfStream` delivery with `Err(truncated)` *after the last packet* and every `drain()`-based test still passes — exact-count asserts are already satisfied. Worse, `chap_tref_chapters_surface_with_titles` (tests.rs:294-299) asserts only `assert!(!packets.is_empty())`, and `absent_stss_means_all_sync` (tests.rs:220) asserts `all(keyframe)` over any non-empty subset — a demuxer that errors after packet 1 passes both. The three manual drain loops that ARE strict (`png_cover...` tests.rs:358-371, `ipcm_pcmc_unmapped...` tests.rs:911-925, `drain_noting` tests.rs:948-963, each `unwrap_or_else(|e| unreachable!(...))`) exist only on the damage-note paths.
What correct looks like: `drain()` should panic on `Err` (mirroring `drain_noting`) or return a flag the callers assert; at minimum one happy-path test must assert clean `EndOfStream` delivery.
Confidence: HIGH — Verification: read (mutation conclusion UNVERIFIED, no cargo).

### F-002 [S3] Module header overclaims: "every error path pinned ... no panics, no unwrap-driven assertions on demuxer output"
Where: crates/demux/src/mp4/tests.rs:1-4
Code:
```rust
//! Unit tests for the MP4/MOV demuxer (card 5.C1) — in-memory
//! fixtures from [`super::test_support`], every error path pinned to
//! its typed code (no panics, no unwrap-driven assertions on
//! demuxer output).
```
What is wrong: Two overclaims. (1) "every error path pinned" — the loose multi-code oracles (F-003) and the unpinned happy-path EOS (F-001) are exactly the unpinned paths; also no test in this file pins `stsz`-fixed over-ceiling, `trun-count-over-ceiling` (mod.rs:1767-1768), or `MAX_SAMPLE_BYTES` (mod.rs:109) — those codes exist in the demuxer unexercised here. (2) "no panics" — every unexpected Result panics via `unwrap_or_else(|e| unreachable!("{e}"))` (e.g. tests.rs:53-54, 123); that is a panic-driven assertion, merely with a message. The substance (typed codes asserted) is real; the doc comment's absolutism is not.
Why it matters: Doc comments in test files are read as coverage claims by auditors and feature-matrix authors; inflated ones propagate (this is the pattern the whole repo audit exists to counter).
What correct looks like: "error paths pinned where pinned; happy-path EOS not asserted; unexpected results panic the test."
Confidence: HIGH — Verification: read + rg of mod.rs note-sites vs tests.

### F-003 [S4] Loose oracles accept 2-3 alternative error codes, weakening two pins
Where: crates/demux/src/mp4/tests.rs:589-594 and 646-649
Code:
```rust
    assert!(
        code == "demux.mp4.box-overruns-parent"
            || code == "demux.mp4.truncated"
            || code == "demux.mp4.no-tracks",
        "got {code}"
    );
```
What is wrong: The first half of `child_beyond_parent_is_typed` accepts any of three codes; `zero_sample_tables_in_progressive_is_typed` (tests.rs:646-649) accepts two. A regression that turns a specific damage path into a generic `truncated` survives.
Why it matters: The test names promise a specific typed refusal; the body accepts a family.
What correct looks like: Pin the exact code per mutated shape; if the code legitimately depends on parse order, document why and pin the set per input, not a union.
Confidence: HIGH — Verification: read. (Mitigation: the second half of tests.rs:567-619 pins `demux.mp4.box-overruns-parent` exactly.)

### F-004 [S4] `fuzz_lite_*` tests have no oracle at all
Where: crates/demux/src/mp4/tests.rs:1077-1096
Code:
```rust
#[test]
fn fuzz_lite_mutations_never_panic() {
    let track = video_track(4, 100, &[true, false, false, true], &[0]);
    let bytes = progressive(&[track], b"isom");
    // Truncation prefixes.
    for cut in [0usize, 7, 8, 32, 64, 128, bytes.len() / 2] {
        super::fuzz::fuzz_demux_mp4(&bytes[..cut.min(bytes.len())]);
```
What is wrong: Zero assertions; the oracle is "did not panic/hang", which is also the entire documented contract of `fuzz_demux_mp4` (mp4/fuzz.rs:6-10). Honest by naming, but it contributes "test count" without behavioral verification; bit-flips at stride 37/53 cover ~3% of bytes.
Why it matters: Named as fuzz evidence; it is a smoke sweep. No cargo-fuzz target exists in-repo (rg: only in-crate `fuzz.rs` harnesses), so this is the only fuzzing MP4 gets.
What correct looks like: Keep, but label as smoke; add a real libFuzzer/cargo-fuzz target if fuzz evidence is claimed anywhere.
Confidence: HIGH — Verification: read + rg `fuzz_demux_mp4` (callers: this file + crates/demux/tests/mp4_fixtures.rs:247).

### F-005 [S4] Byte-scan fixture patching is fragile and can underflow
Where: crates/demux/src/mp4/tests.rs:519-529 (same pattern at 541-549, 573-578, 601-605, 627-636)
Code:
```rust
    let mut at = None;
    for i in 0..bytes.len().saturating_sub(4) {
        if &bytes[i..i + 4] == b"mdat" {
            at = Some(i - 4);
        }
    }
```
What is wrong: Raw scan for a 4cc can match payload bytes (here payloads are synthetic 0x10../0xA0.. so it cannot fire, but a fixture change to ASCII sample data would silently patch the wrong offset), and `i - 4` underflows (panics in debug/test builds) if the 4cc ever appears at offset < 4. The shared helper `find_box` (tests.rs:937-941) got this right (`(4..len).find(...)`); the five inline scans did not reuse it.
Why it matters: Test-infra fragility; a future fixture edit turns into a confusing panic or a silently wrong patch.
What correct looks like: Route all patch sites through `find_box`.
Confidence: HIGH — Verification: read.

### F-006 [S4] `drain()`'s `events` return value is dead
Where: crates/demux/src/mp4/tests.rs:69-70
Code:
```rust
    (events, packets)
}
```
What is wrong: All ~23 call sites bind `let (_, packets)`. The counter exists only to feed the internal `assert!(events < 10_000)` budget; as a return value it is dead weight that misleads readers into thinking event totals are checked somewhere.
Why it matters: Dead return values in a shared harness invite misuse (see F-001 — the flag that mattered was error-vs-EOS, and it is the one not returned).
What correct looks like: Return packets only, or return a struct with an `errored: bool` callers must assert.
Confidence: HIGH — Verification: read + rg of call sites.

## 4. Per-function table

| Function / test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `video_track` (helper) | 15-31 | REAL | — | Builds TrackSpec; `syncs.get(i).unwrap_or(true)` silently defaults missing flags — fixture convenience, fine. |
| `audio_track` (helper) | 33-49 | REAL | — | Trivial spec builder. |
| `open` (helper) | 51-55 | REAL | — | `unreachable!` on construct errors — acceptable test precondition. |
| `drain` (helper) | 57-70 | SHALLOW | S2 | F-001: Err≡EOS conflation; F-006 dead `events` return. Bounded by 10k budget (good). |
| `ErrOf::err_of` (trait+impl) | 72-83 | REAL | — | Panics on Ok — strict, fine. |
| `code_of` (helper) | 85-87 | REAL | — | String-code accessor; codes are stringly compared (see error.rs audit, file 3). |
| `probe_accepts_known_brands_and_qt_heads` | 91-114 | REAL | — | Positive+negative brands, pre-ftyp moov, short header, cut-short brands. Would fail if brand table/probe broke. Verified probe logic mod.rs:154-174 matches. |
| `progressive_single_track_demuxes_samples_in_order` | 118-149 | REAL | — | Exact count 4, DTS ladder i*3000@9000, stss flags, verbatim payloads. Strong. Drain caveat F-001. |
| `progressive_two_tracks_interleave_by_file_offset` | 152-166 | REAL | — | 6 packets, handles at 0 and 3. Would catch audio-first ordering. |
| `ctts_shifts_pts_off_dts` | 169-182 | REAL | — | Negative cts via v0 box (builder writes `s.cts as u32`, tests.rs test_support.rs:244) pins signed interpretation; would fail if parser read v0 ctts unsigned. |
| `elst_empty_edit_delays_and_media_time_trims` | 185-211 | REAL | — | Empty edit (-1 → 0xFFFFFFFF) dwell 1000; media_time trim → negative PTS -1000. Pins elst v0 semantics incl. empty-edit special case. |
| `absent_stss_means_all_sync` | 214-221 | REAL | — | All-sync default; weak-ish oracle (`all` over subset) — F-001 exposure. |
| `fragmented_demuxes_moofs_with_tfdt_ladder` | 226-254 | REAL | — | tfdt 0/1024/2048/4096, trun flags sync pattern, `SeekSupport::SlowLinear`, `is_fragmented`. Strong. |
| `fragmented_two_tracks_per_moof` | 257-271 | REAL | — | Per-moof traf interleave by track handle. |
| `chap_tref_chapters_surface_with_titles` | 276-300 | REAL | — | Chapter track hidden from track list, titles, ns conversion (1_000_000_000), no packet leak. `!packets.is_empty()` is the weak leg (F-001). |
| `cover_art_surfaces_as_a_still_image_video_track` | 305-332 | REAL | — | Cover→mjpeg still-image track, exactly 1 cover packet, bytes verbatim, audio order. Strong (exact count 3). |
| `png_cover_and_unknown_cover_types_map_or_note` | 335-374 | REAL | — | PNG→png codec; unknown wkt 27 → `Recovered{reason:"cover-type-unsupported"}`, audio survives. Strict manual loop (panics on Err). |
| `cover_and_chapters_keep_packet_handles_aligned_with_tracks` | 377-401 | REAL | — | Hidden chapter trak between media trak and cover; handle alignment ≤1. |
| `span_at` (helper) | 405-413 | REAL | — | ns→TimeSpan. |
| `progressive_seek_lands_on_keyframe` | 416-445 | REAL | — | Lands on keyframe 0 then 4; drain-after-seek count 2; out-of-range typed `demux.mp4.seek.out-of-range`. Strong. |
| `fragmented_seek_scans_moofs` | 448-470 | REAL | — | Linear moof scan landing at 4000 ticks, 4 packets after. |
| `not_mp4_is_typed_refusal` | 475-479 | REAL | — | RIFF/WAVE → `demux.mp4.not-mp4`. |
| `missing_moov_is_typed` | 482-491 | REAL | — | ftyp + garbage → `demux.mp4.no-moov`. |
| `ftyp_bytes` (helper) | 493-502 | REAL | — | Hand-built ftyp; size 32 self-consistent. |
| `truncated_box_header_is_typed_truncation` | 505-513 | REAL | — | Half-file cut → `demux.mp4.truncated`. |
| `size_zero_convention_extends_to_stream_end` | 516-534 | REAL | — | mdat size 0 = to-EOF convention; 2 packets. Patch via fragile byte scan (F-005). |
| `size_one_reads_64bit_largesize` | 537-564 | REAL | — | size=1 + largesize rebuild; 2 packets. |
| `child_beyond_parent_is_typed` | 567-619 | REAL | — | stbl shrink (loose 3-code oracle, F-003) + trak inflate pinned to `demux.mp4.box-overruns-parent`. |
| `zero_sample_tables_in_progressive_is_typed` | 622-650 | REAL | — | stsz/stco counts→0; loose 2-code oracle (F-003). |
| `unknown_boxes_between_ftyp_and_moov_are_tolerated` | 653-670 | REAL | — | Splice junk box; hard-codes ftyp end at 36 (couples to builder layout — brittle but asserted via success). |
| `truncated_sample_read_is_typed_after_start` | 673-688 | REAL | — | Partial delivery then `demux.mp4.truncated`, sticky repeat. Good — the only test that pins mid-stream Err behavior. |
| `not_started_calls_are_typed` | 691-699 | REAL | — | `demux.mp4.not-started` for next_event+seek. |
| `mov_quirks_wide_and_qt_brand_open` | 702-717 | REAL | — | `wide` prefix + qt brand + `major_brand()` accessor. |
| `sowt_entry_maps_to_pcm_s16le` | 720-727 | REAL | — | sowt→pcm_s16le. |
| `threegp_brand_and_samr_entry_open` | 730-739 | REAL | — | 3gp5 + samr→amr_nb, 2 packets. |
| `seek_failure_leaves_state_usable_d4` | 742-763 | REAL | — | OOR seek does not kill the walk; remaining 2 packets flow. |
| `pcmc_box` (helper) | 770-778 | REAL | — | Correct 23003-5 pcmC FullBox shape (v/f + flags + format). |
| `pcm_track` (helper) | 782-798 | REAL | — | ipcm/fpcm spec, samplesize stays 16 (matches ffmpeg quirk). |
| `ipcm_fpcm_pcmc_mapping_table` | 801-854 | REAL | — | 10 mapping cases + 4 unmapped + 2 no-pcmC defaults, asserted against `super::pcm_codec_of_ipcm` directly. Strong unit pin. |
| `ipcm_fpcm_with_pcmc_demux_as_pcm_codecs` | 857-880 | REAL | — | End-to-end pcm_s16le/pcm_f32le with pcmC children, payload bytes checked. |
| `ipcm_without_pcmc_defaults_to_entry_name_convention` | 883-894 | REAL | — | ipcm→pcm_s16be default. |
| `ipcm_pcmc_unmapped_width_notes_unknown_entry` | 897-930 | REAL | — | Unmapped 0x30 → trak dropped, `unknown-stsd-entry` note with exact magnitude 50, sibling demuxes. Strict loop. Strong. |
| `find_box` (helper) | 937-941 | REAL | — | Underflow-safe first-box scan. |
| `drain_noting` (helper) | 945-964 | REAL | — | Strict (panics on Err) + captures note magnitude. The harness `drain` should have been. |
| `stco_count_over_ceiling_is_bounded_and_damage_noted` | 967-985 | REAL | — | AUD-01 regression: count 0x0800_0000 → note `0x0800_0000 - MAX_SAMPLES`, 2 real packets still demux. Verified against mod.rs:1268-1276 (`take = count.min(MAX_SAMPLES)`; past-box reads return `unwrap_or(0)` offsets). Would fail if the ceiling were removed (unbounded loop/alloc) or the note dropped. |
| `co64_count_over_ceiling_is_bounded_and_damage_noted` | 988-1016 | REAL | — | co64 restamp twin; 1 packet. Convoluted but correct. |
| `stsc_count_over_ceiling_is_bounded_and_damage_noted` | 1019-1035 | REAL | — | stsc twin; healthy sibling still demuxes. |
| `stss_unsorted_with_duplicates_matches_sorted_flags` | 1038-1073 | REAL | — | Property test: unsorted dup multiset [3,3,1] ≡ canonical {1,3}, plus exact flag vector. Pins mod.rs:1310-1311 sort+dedup. |
| `fuzz_lite_mutations_never_panic` | 1078-1096 | SHALLOW | S4 | F-004: no oracle; absence-of-panic smoke only. |
| `fuzz_lite_fragment_mutations_never_panic` | 1099-1111 | SHALLOW | S4 | Same, fragmented shape. |

## 5. Test honesty (§3.4 judgment)

| Test (family) | Would it fail if the code were broken? | Reason |
|---|---|---|
| 22 exact-count/payload tests (rows above marked REAL) | YES | Exact counts, ticks, bytes, codec strings — any stbl math regression fails them. |
| `absent_stss_means_all_sync`, `chap_tref...` | PARTIALLY | Weak legs (`all()` over subset, `!is_empty()`) — fail only if breakage hits before last delivered packet; see F-001. |
| All typed-error tests (10) | YES (8 exact), PARTIALLY (2 loose, F-003) | Codes asserted by string equality. |
| Ceiling tests (3) | YES | Note magnitude `declared - MAX_SAMPLES` is arithmetic, not recorded golden; removal of cap changes both note and behavior. |
| `fuzz_lite_*` (2) | NO (by design) | No oracle; panic/hang absence only. |
| Happy-path EOS delivery (all drain users) | NO | F-001: Err after last packet is indistinguishable from EOS. |

Circular goldens: none (no golden files; builder is an independent writer). Mocks-of-the-subject: none. Ignored tests: none (rg `#[ignore]` in file: 0 hits).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every error path pinned to its typed code" | tests.rs:2 | FALSE (overclaim) | F-001/F-002/F-003: EOS path unpinned, 2 loose oracles, `trun`/`stsz`-fixed/`MAX_SAMPLE_BYTES` codes unexercised in this file. |
| "no panics, no unwrap-driven assertions on demuxer output" | tests.rs:3-4 | FALSE (cosmetic) | `unwrap_or_else(|e| unreachable!(...))` throughout (e.g. :53-54, 123) is panic-driven assertion with a message. |
| "fixtures from super::test_support" | tests.rs:2 | REAL | Verified: builders at test_support.rs:313-515, no parser round-trip. |
| test_support doc "the integration suites + fuzz harness need deterministic shapes" | test_support.rs:3-5 | MOSTLY FALSE for MP4 | rg across crates/demux/tests/: zero uses of `mp4::test_support` progressive/fragmented/chapter_track; only `mem_stream` is consumed externally (mp4/fuzz.rs:46 and apps/cli/src/containers/mp4.rs:637, the latter `#[cfg(test)]` at :430). The fixture builders' only consumer is this file. |

## 7. Dead / duplicated / unreachable code

- `drain`'s `events` return (F-006) — dead at all call sites.
- Five inline byte-scan loops duplicate `find_box` (F-005).
- Nothing unreachable: whole file is `#[cfg(test)]`.

## 8. Dependencies & flags

- No feature flags gate this file; `#[cfg(test)] mod tests;` (mp4/mod.rs:103-104) is the only gate. Sibling `pub mod fuzz` / `pub mod test_support` (mp4/mod.rs:100-101) are NOT cfg-gated and compile into every `demux` build — test-support surface in the production rlib (S4 hygiene; justified for `fuzz` by the cross-crate consumer crates/demux/tests/mp4_fixtures.rs:20,247, and for `test_support::mem_stream` by apps/cli cfg(test) code at apps/cli/src/containers/mp4.rs:637; but the fixture builders `progressive*`/`fragmented` ship unused by any non-cfg-test consumer).
- Upstream deps of the tested surface: `media_types` (Packet, EngineError, time), `plugin_api` (DemuxEvent, Demuxer, SeekSupport), `crate::avi::riff::{PayloadSlots, SourceReader}` (mp4/mod.rs:97 — the MP4 demuxer reuses the AVI reader seam).

## 9. Open questions for the next agent

- mp4/mod.rs (different agent's file): does `next_event` ever return `Err` after all packets are emitted on a healthy file? If not, F-001 is latent-only; if the moov-at-end or mdat-trailing path can, it is live. Static check of mod.rs drain paths recommended.
- stts handler (mod.rs:1253-1266) has NO per-run note-cap consistency test here for `stts-count-over-ceiling` (the ceiling tests cover stco/co64/stsc only) — coverage gap worth one test.
- crates/demux/tests/mp4_fixtures.rs + corpus_mp4.rs (other agents): whether real ffmpeg-authored corpus files cover EOS-cleanly-delivered on happy paths (would close F-001's hole at the integration level).
