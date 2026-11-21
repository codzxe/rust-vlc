# AUDIT: crates/demux/src/mp4/tests.rs#part2 (lines 556-1111)
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-21 |
| Lines / functions | 556 (chunk 556-1111 of 1111; part1=1-555 is W05-A04's) / 24 items (22 full tests + boundary tail of a 25th + 3 test helpers) |
| File verdict | REAL (synthetic-fixture unit suite, all tests baseline-executed; strong regression pins for AUD-01/AUD-03/D4; weakened by disjunctive error assertions and a never-executed real-file corpus differential upstream) |
| Reachable from shipped entry point? | N/A for the file itself (`mod tests` is cfg(test), never shipped). The SUBJECT under test is reachable: `lumen open x.mp4` → `apps/cli/src/demux_host.rs:458 containers::open` → `apps/cli/src/containers/mod.rs:67 "mp4" => mp4::open` → `apps/cli/src/containers/mp4.rs:100 Mp4Demuxer::new` → `crates/demux/src/mp4/mod.rs` (the code every test here drives) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | full Read of tests.rs 1-1111 (context; audited 556-1111); `rg -n "mp4::tests::" orchrun-test-workspace-nofailfast.log.txt` → 38/38 ok incl. every part2 test (:1764-1801); `sed -n '2805,2830p'` same log → containers::mp4::tests incl. corpus line :2814; Read apps/cli/src/containers/mp4.rs:942-1000 (corpus skip gate) + `ls /tmp/my-project/corpus-v4/media` → No such file or directory; `rg -n "MAX_SAMPLES\|pcm_codec_of_ipcm" crates/demux/src/mp4/mod.rs` → :122 (1 MiB), :481, production caller :1179; `sed -n '1269,1276p' mod.rs` (stco take); `sed -n '675,684p' mod.rs` (note fn); `sed -n '2155,2168p' crates/playback-engine/src/pipeline.rs` (skipped summed into damaged); `sed -n '310,315p' test_support.rs` (moov-at-front only); Read crates/demux/src/mp4/fuzz.rs:1-57; `rg -n "language" tests.rs` → no hits; smoke evidence read: mp4-smoke-open-session-420.txt, mp4-smoke-bench-420.txt, mp4-smoke-open-session.txt, lumen-engine-info.txt; `sed -n` on every quoted range; byte-level hex verification of test_support.rs attribute lines after a display artifact triggered a false-corruption suspicion (resolved: file intact, `#[must_use]` present 7×) |

## 1. One-paragraph verdict
Part 2 is the damage/quirks/codec-mapping half of the MP4 unit suite, and it is genuine: every test in the chunk executed and passed in the no-fail-fast baseline (nofailfast:1764-1801), the AUD-01 ceiling regressions (stco/co64/stsc count = 0x0800_0000) pin the exact capped-take arithmetic and the survival of the real samples, the AUD-03 unsorted-stss test would catch a binary-search-without-sort regression, the sticky post-start truncation and the D4 seek-failure-resume tests pin behavior most demuxers get wrong, and the ISO/IEC 23003-5 ipcm/fpcm mapping table drives the real production function (`pcm_codec_of_ipcm`, mod.rs:481, called at :1179) through 10 positive, 4 negative and 2 default cases. The weaknesses: two tests accept a disjunction of error codes, so a regression from the specific contract code to any sibling typed refusal still passes; the ceiling tests bless a damage-note semantics whose magnitude is derived from the declared count (133,169,152 "skipped" units from a box containing 2 real entries) and whose unit (entries) contradicts the `bytes: u64` parameter name it flows through and the byte-denominated damage counters the pipeline adds it to; and the entire executed real-file MP4 evidence in the repo reduces to this synthetic suite plus the four smoke runs, because the one ffmpeg-truth corpus differential loud-skips in every committed run while still reporting `ok`.

## 2. What actually works (verified)
- AUD-01 ceiling regressions (966-1035): `stco_count_over_ceiling…`, `co64_count_over_ceiling…`, `stsc_count_over_ceiling…` all patch a count to 0x0800_0000 and assert `noted == Some(0x0800_0000 - MAX_SAMPLES)` with the real samples still demuxing (2/1/2 packets) — verified against mod.rs:1268-1300 (`take = count.min(MAX_SAMPLES)`, `note(count - take)`); pre-cap code would have walked/allocated the declared 128M entries, so these fail on the regression they were written for. All three ok in baseline (nofailfast:1794, 1767, 1795).
- AUD-03 `stss_unsorted_with_duplicates_matches_sorted_flags` (1037-1073): rewrites the sync table as the unsorted multiset [3,3,1] and requires flag-equality with the sorted count=2 [1,3] form plus the exact expected flag vector — a binary-search-without-sort implementation produces different flags and fails. Baseline ok (:1796).
- `truncated_sample_read_is_typed_after_start` (672-688): one-byte tail cut → packet 1 delivered, then `demux.mp4.truncated` twice (stickiness pinned). `not_started_calls_are_typed` (690-699): both `next_event` and `seek` before `start` → `demux.mp4.not-started`. `seek_failure_leaves_state_usable_d4` (741-763): out-of-range seek refuses typed and the walk resumes with exactly the remaining 2 packets. All baseline ok.
- MOV/3GP quirks (701-739): `wide` pre-box + `qt  ` brand opens with `major_brand() == b"qt  "`; `sowt` → `pcm_s16le`; `samr`/`3gp5` → `amr_nb` with 2 packets. `size_one_reads_64bit_largesize` (536-564; boundary test, 556-564 in this chunk) rebuilds mdat with size=1 + 64-bit largesize and still demuxes 2 samples.
- ISO/IEC 23003-5 PCM (765-930): `ipcm_fpcm_pcmc_mapping_table` drives the production `pcm_codec_of_ipcm` (mod.rs:481; production call site :1179) over 10 (kind,endian,width)→codec rows, 4 unmapped combos → None, and both no-pcmC entry-name defaults; the integration twins (`ipcm_fpcm_with_pcmc_demux_as_pcm_codecs`, `ipcm_without_pcmc_defaults_to_entry_name_convention`) assert codec strings, packet counts and payload bytes through the full `start`+`drain` path; `ipcm_pcmc_unmapped_width_notes_unknown_entry` pins the dropped trak, the healthy sibling's survival, and the note magnitude 50 (8 header + 28 base + 14 pcmC — arithmetic verified). All baseline ok (:1777-1780).
- Fuzz-lite (1077-1111): deterministic truncation-prefix/single-bit-flip/garbage-tail sweeps over `fuzz_demux_mp4` (progressive + fragmented fixtures) — honest for their documented contract (fuzz.rs:4-8: "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else"; errors swallowed by design). Baseline ok (:1775-1776).
- Subject-level execution: the 4:2:0 smoke run proves the demuxer opens a real ffmpeg-authored MP4 end-to-end (mp4-smoke-open-session-420.txt: tracks 2 ready → Buffering → Playing → Stopped → Idle, EXIT=0) — the demux layer is NOT the owner's MP4 blocker (the yuv444p refusal is the decoder layer: mp4-smoke-open-session.txt:11 `error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`; sw.rs exists and gates to 4:2:0 8-bit by design — sw.rs:367-371 `eight_bit_420` check; correction commit 262a99cd).

## 3. Findings

### F-001 [S3]
Where: evidence context for this file's subject: apps/cli/src/containers/mp4.rs:963-967; executed-evidence line: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2814; corpus dir `/tmp/my-project/corpus-v4/media` (absent in this sandbox — `ls` → No such file or directory)
Code:
```rust
    fn corpus_iso_bmff_lanes_match_the_ffmpeg_truth() {
        let Some(dir) = corpus_dir() else {
            skip_loudly();
            return;
        };
```
What is wrong: the ONLY real-file, ffmpeg-measured-truth differential for the MP4 open path takes a loud-skip branch when the corpus dir is absent and `LUMEN_CORPUS` unset — `skip_loudly()` is an `eprintln!` + `return` (mp4.rs:954-959), and a skipped test still reports `ok` in cargo output. The baseline records `test containers::mp4::tests::corpus_iso_bmff_lanes_match_the_ffmpeg_truth ... ok` (nofailfast:2814) with zero proof the differential body executed; the corpus dir does not exist in this sandbox. (Identical shape to the Ogg corpus finding W04-A10 F-002.)
Why it matters: the executed MP4 evidence in the whole committed record is (a) this file's synthetic-fixture unit tests (all fixtures built by `test_support::progressive/fragmented` — boxes the repo itself wrote), (b) the in-crate container tests (synthetic fixtures with real esds/avcC payloads), and (c) four smoke runs on ffmpeg-authored samples. The per-lane ffmpeg-truth table (per-track packet counts, codecs, decodes) that would catch subtle real-file divergence has never executed in any committed run. For the MP4-priority wave this is the difference between "the synthetic dialect demuxes" and "real files demux" — currently bridged only by 4 smoke files.
What correct looks like: commit 2-3 tiny deterministic ffmpeg-authored MP4 fixtures into the repo (the smoke samples dir already has them: sample-420.mp4 is 79,800 bytes) and point the corpus gate at them, or make the skip `ignored`-visible so gates cannot count it as a pass.
Confidence: HIGH — Verification: read (mp4.rs:942-1000); dir absence verified by ls; baseline line cited.

### F-002 [S4]
Where: crates/demux/src/mp4/tests.rs:586-594 and 643-649
Code:
```rust
    let err = mp4.start().err_of("overrun");
    let code = code_of(&err);
    assert!(
        code == "demux.mp4.box-overruns-parent"
            || code == "demux.mp4.truncated"
            || code == "demux.mp4.no-tracks",
        "got {code}"
    );
```
```rust
    assert!(
        code == "demux.mp4.no-sample-table" || code == "demux.mp4.no-tracks",
        "got {code}"
    );
```
What is wrong: disjunctive error-code assertions. The first sub-case of `child_beyond_parent_is_typed` accepts any of THREE codes, and `zero_sample_tables_in_progressive_is_typed` accepts any of TWO — a regression that turns `box-overruns-parent` into `no-tracks` (or `truncated`) passes green. The tests prove "some typed refusal", not the contract code. (The second sub-case of `child_beyond_parent_is_typed`, 615-618, does pin `box-overruns-parent` exactly — the strong pin exists one page down for the direct shape.)
Why it matters: the file's own header promises "every error path pinned to its typed code" (tests.rs:2-4); three-way disjunctions are not pins. Damage-ordering nondeterminism is a real excuse, but the honest form is to make the walk's ordering deterministic and pin one code.
What correct looks like: pin the single deterministic code per shape, or at most assert the code set with each accepted code individually reachable in a dedicated sub-test.
Confidence: HIGH — Verification: read; both bodies quoted.

### F-003 [S3]
Where: crates/demux/src/mp4/tests.rs:979-985 (assertion), with crates/demux/src/mp4/mod.rs:1269-1276 + 675-678 and crates/playback-engine/src/pipeline.rs:2164-2168 (cross-file evidence)
Code:
```rust
    // Exactly MAX_SAMPLES taken; the declared surplus is the note's
    // magnitude — the walk is bounded by construction.
    let max = super::MAX_SAMPLES as u64;
    assert_eq!(noted, Some(0x0800_0000u64 - max));
    assert_eq!(packets, 2, "the two real samples still demux");
```
```rust
                b"stco" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    let take = count.min(MAX_SAMPLES);
                    for i in 0..take {
                        offsets.push(u64::from(u32be(body, 8 + i * 4).unwrap_or(0)));
                    }
```
```rust
    fn note(&mut self, bytes: u64, reason: &'static str) {
```
What is wrong: the ceiling tests bless two semantics problems at once. (1) The damage-note magnitude is `declared_count − MAX_SAMPLES` = 133,169,152 "units" for a box that physically contains TWO entries — nothing was skipped; the surplus was never on disk. The walk "reads" 1,048,576 stco entries from an 8-byte entry area, with every out-of-box read silently zero-filled by `u32be(...).unwrap_or(0)` — the box-extent overrun itself (1,048,574 phantom reads) is never damage-noted. (2) The unit: `note()`'s parameter is named `bytes` (mod.rs:675), and the event flows into `report.damaged_demux`/`report.damaged` (pipeline.rs:2164-2168) which elsewhere accumulate BYTES — the TS bridge's Recovered records count bytes (containers/ts.rs:275-281; pes.rs `Recovered.skipped` = payload bytes) — yet here the magnitude is an ENTRY count. One patched stco adds 133M phantom "bytes" to the engine's damage report.
Why it matters: the tests pin all of this as expected behavior ("the walk is bounded by construction"), so a future fix (extent-bounded take, byte-true magnitudes, a distinct note for extent overrun) would break these assertions and be "reverted to green" by whoever trusts them. Cross-demuxer consumers of `Recovered.skipped` get incompatible units from the same field.
What correct looks like: take = min(count, MAX_SAMPLES, entries actually present in the box extent), a separate note (`stco-past-box-extent`) for reads beyond the box, and note magnitudes in the unit the parameter name (and the TS lane) promise — or rename the parameter/field to `units` everywhere.
Confidence: HIGH on the mechanism (all three sites read); MEDIUM on the right policy (bounded + documented as-is is defensible if the field is renamed).
Verification: read; MAX_SAMPLES = 1 MiB verified (mod.rs:122); pipeline sum site read.

### F-004 [S4]
Where: crates/demux/src/mp4/tests.rs:945-964
Code:
```rust
fn drain_noting(mp4: &mut Mp4Demuxer, reason_want: &str) -> (Option<u64>, usize) {
    let mut noted = None;
    let mut packets = 0usize;
    loop {
        match mp4
            .next_event()
            .unwrap_or_else(|e| unreachable!("event: {e}"))
        {
            DemuxEvent::Packet(_) => packets += 1,
            DemuxEvent::Recovered { skipped, reason } => {
                if reason == reason_want {
                    noted = Some(skipped);
                }
            }
```
What is wrong: `drain_noting` overwrites `noted` on every matching record — only the LAST note of a reason is visible to the assertion. If a walk emitted two `stco-count-over-ceiling` notes (multi-trak fixtures repeat sample-table boxes), the first magnitude would be invisible and an assertion on the last could pass while the first was wrong.
Why it matters: the helper is the ceiling tests' only measuring instrument; it silently collapses multiplicities. Today's fixtures emit one note per reason, so nothing is hidden — the defect is latent.
What correct looks like: `Vec<u64>` capture + explicit length assertion.
Confidence: HIGH — Verification: read; all three callers checked (single-note fixtures).

### F-005 [S4]
Where: crates/demux/src/mp4/test_support.rs:310-313 (fixture shapes), gap evidence: `rg -n "language" crates/demux/src/mp4/tests.rs` → no hits; audit/runs/baseline/mp4-smoke-open-session-420.txt (TrackInfo language)
Code:
```rust
/// Build a progressive MP4: `ftyp + moov + mdat`, one chunk per
/// sample, offsets patched in a second pass.
```
What is wrong: three coverage gaps in what the suite can see. (1) `progressive` always writes moov-at-FRONT (`ftyp + moov + mdat`) — the ffmpeg-default moov-at-END layout (the original smoke sample) is never built at unit level; it is exercised only by the 4 smoke runs. (2) The demux-crate fixtures carry no `avcC`/`esds` children (rg over test_support.rs: zero hits) — codec identity is asserted by fourcc alone; real config-extraction is tested only in apps/cli's container tests (`esds_dsi_walk_extracts_the_asc`, nofailfast:2816) — a layering asymmetry this file cannot fix but should document. (3) No test anywhere in the file asserts track `language`, while the smoke run shows the standing garbage default on every MP4 track — `language: Some("voe")` on both tracks of sample-420.mp4 (mp4-smoke-open-session-420.txt, MediaInfoReady event; INDEX.md "Standing defects the correction EXPOSES": "garbage default (demux wave to trace)"). The unit suite is structurally blind to exactly the defect class the smoke evidence exposed.
Why it matters: for the MP4-priority wave, the suite's green does not cover the moov-at-end layout, config extraction (at this layer), or metadata honesty — the three places real files diverge from the builder's dialect.
What correct looks like: a `progressive_moov_at_end` builder variant (offsets patched after mdat), one language assertion (e.g. `tracks[0].language` is None or a real value — never "voe"), and a comment naming where esds/avcC coverage lives.
Confidence: HIGH on the gaps (greps + builder read); the "voe" defect itself belongs to the mod.rs owner.
Verification: read + greps; smoke evidence cited.

## 4. Per-function table (part2 items; helpers `video_track`/`audio_track`/`open`/`drain`/`ErrOf`/`code_of`/`span_at`/`ftyp_bytes` and tests at lines ≤555 are part1/W05-A04's)
| Item | Lines | Verdict | Severity | Notes (mutation sensitivity, honesty) |
|---|---|---|---|---|
| `size_one_reads_64bit_largesize` (tail; starts 536) | 556-564 | REAL | — | largesize honored → 2 packets; largesize-ignored mutation mis-walks mdat (size=1) → fails; baseline ok (:1791) |
| `child_beyond_parent_is_typed` | 566-619 | REAL | S4 | sub-case 1 disjunctive (3 codes, F-002); sub-case 2 pins `box-overruns-parent` exactly (strong); baseline ok (:1766) |
| `zero_sample_tables_in_progressive_is_typed` | 621-650 | REAL | S4 | disjunctive (2 codes, F-002); patch offsets verified vs box layouts (stsz+16, stco+12) |
| `unknown_boxes_between_ftyp_and_moov_are_tolerated` | 652-670 | REAL | — | junk top-level box skipped, 1 packet; splice at ftyp end (36) verified vs ftyp_bytes() 32+4 |
| `truncated_sample_read_is_typed_after_start` | 672-688 | REAL | — | exact code + sticky repeat; strong |
| `not_started_calls_are_typed` | 690-699 | REAL | — | both paths exact codes |
| `mov_quirks_wide_and_qt_brand_open` | 701-717 | REAL | — | wide box + qt brand + major_brand() assert |
| `sowt_entry_maps_to_pcm_s16le` | 719-727 | REAL | — | codec string only (no drain) — weakest of the mapping trio but honest |
| `threegp_brand_and_samr_entry_open` | 729-739 | REAL | — | amr_nb + 2 packets |
| `seek_failure_leaves_state_usable_d4` | 741-763 | REAL | — | D4: walk resumes with exactly 2 remaining packets after typed seek refusal; strong |
| `pcmC_box` (helper) | 770-778 | REAL | — | 14-byte FullBox per 23003-5 §7.3.2; layout verified (v/f 4, flags 1, format 1) |
| `pcm_track` (helper) | 782-798 | REAL | — | 48 kHz, 2-byte samples; geometry via classic v0 fields as documented |
| `ipcm_fpcm_pcmc_mapping_table` | 800-854 | REAL | — | drives production fn (mod.rs:481, caller :1179); 10+4+2 cases; mutation-sensitive (any table row wrong → fails) |
| `ipcm_fpcm_with_pcmc_demux_as_pcm_codecs` | 856-880 | REAL | — | full open+drain, codec+counts+payload bytes |
| `ipcm_without_pcmc_defaults_to_entry_name_convention` | 882-894 | REAL | — | entry-name default pinned (i=be, f=le) |
| `ipcm_pcmc_unmapped_width_notes_unknown_entry` | 896-930 | REAL | — | dropped trak + sibling survival + note magnitude 50 (8+28+14 verified) |
| `find_box` (helper) | 937-941 | REAL | — | scan-from-4 + i−4 box start; range arithmetic verified (exclusive end len−3 ⇒ max i = len−4) |
| `drain_noting` (helper) | 945-964 | REAL | S4 | last-note-wins capture (F-004) |
| `stco_count_over_ceiling_is_bounded_and_damage_noted` | 966-985 | REAL | S3 | AUD-01 regression: cap+note+2 packets pinned; blesses entry-count/zero-fill semantics (F-003) |
| `co64_count_over_ceiling_is_bounded_and_damage_noted` | 987-1016 | REAL | S3 | co64 restamp arithmetic verified (2×4B → 1×8B entry, off0 rewritten); same semantics blessing |
| `stsc_count_over_ceiling_is_bounded_and_damage_noted` | 1018-1035 | REAL | S3 | healthy-sibling-survival shape; same semantics blessing |
| `stss_unsorted_with_duplicates_matches_sorted_flags` | 1037-1073 | REAL | — | AUD-03: flags equality between unsorted-multiset and sorted-deduped forms + exact vector; binary-search-without-sort caught |
| `fuzz_lite_mutations_never_panic` | 1077-1096 | REAL | — | panic-contract only (fuzz.rs:4-8 documents the swallowing); 6 cuts + stride-37 flips + garbage tail |
| `fuzz_lite_fragment_mutations_never_panic` | 1098-1111 | REAL | — | fragment variant: 5 cuts + stride-53 bit flips |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stco/co64/stsc ceilings | YES for the cap (pre-AUD-01 shape = walk/alloc of 128M entries) | note magnitude + take arithmetic + packet counts all pinned; NO for a hypothetical extent-bounded take (F-003 policy is what's pinned) |
| stss_unsorted | YES | unsorted-binary-search mutation → flags differ from both expected vectors |
| child_beyond_parent sub-case 1 | NO for the specific code (3 accepted) | any sibling typed refusal passes; sub-case 2 carries the exact pin |
| zero_sample_tables | NO for the specific code (2 accepted) | proves typed-refusal-ness only |
| ipcm mapping table + integration twins | YES | exact codec strings, positive/negative/default, full-walk payload bytes |
| fuzz_lite ×2 | NO for behavior (by documented contract) | panic/hang-only; errors swallowed in fuzz_demux_mp4 by design |
| all remaining part2 tests | YES | exact codes/ticks/counts/payloads asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every error path pinned to its typed code (no panics…)" | tests.rs:2-4 (module doc) | SHALLOW | F-002: three-way and two-way disjunctions are not pins |
| AUD-01 "the take is now capped at MAX_SAMPLES with the surplus damage-noted" | tests.rs:968-970 comment | REAL | mod.rs:1268-1276 read; tests pass (nofailfast:1794-1795) — but the note magnitude unit is entries (F-003) |
| AUD-03 "the sync-set membership check binary-searches the sorted, deduped set" | tests.rs:1039-1042 comment | REAL | mod.rs stss arm read (sort_unstable); test pins behavior not internals |
| "verified against ffmpeg 7.1.5's PCM-in-MP4 output" (pcmC table) | tests.rs:802-803 comment | REAL-UNPROVEN | the mapping matches ffmpeg codec naming, but no executed ffmpeg-differential for PCM lanes exists (corpus skipped, F-001); ffmpeg 7.1.5 authored the smoke samples (INDEX.md §Samples) but not a PCM-in-MP4 lane |
| MP4 opens end-to-end (the owner's litmus) | baseline smoke, INDEX.md CORRECTION v2 | REAL for 4:2:0 H.264+AAC | mp4-smoke-open-session-420.txt: Playing → Stopped → Idle, EXIT=0; demux NOT the blocker; yuv444p refusal = decoder layer (mp4-smoke-open-session.txt:11; sw.rs:367-371 4:2:0-8bit gate; commit 262a99cd) |
| decode-bench capability reporting | mp4-smoke-bench-420.txt | FALSE (tool output) | bench prints "no decoder in this build (first vertical: mjpeg + pcm)" and decodes 0 frames for the very file `open` plays — hard-coded fall-through (INDEX.md standing defects; lumen-engine-info.txt decoders: mjpeg, pcm_s16le also hard-coded) |

## 7. Dead / duplicated / unreachable code
- None dead in this chunk. Duplication note: `find_box`/`drain_noting` (932-964) exist only in this chunk while part1's tests use inline scan loops for the same patch-a-box job (e.g. 520-528, 573-580) — two idioms for one task in one file; the part2 helpers are the better ones.
- The fixtures carry no avcC/esds (codec by fourcc) — see F-005 for the layering consequence.

## 8. Dependencies & flags
- Test module compiles under `cargo test -p demux` only (`mod tests` gated at mp4/mod.rs:104; test_support and fuzz are NOT test-gated — `pub mod test_support;`/`pub mod fuzz;` at mod.rs:100-101, compiled into every build and used by apps/cli containers + integration suites + the example). No feature gates on any test in the chunk; all 38 file tests ran in both the plain workspace run and the all-features run.
- Baseline execution: 38/38 `mp4::tests::` ok (nofailfast:1764-1801); all 24 of my chunk's tests present and ok.

## 9. Open questions for the next agent
- For the mod.rs owner: the `note(bytes: u64)` parameter name vs entry-count magnitudes (F-003) — decide units and rename or convert; also `u32be().unwrap_or(0)` silently zero-fills past-box reads across the whole stbl walk (stts/stco/co64/stsc/stss arms) — extent-bounded takes would be the honest shape.
- Who owns the `language: Some("voe")` garbage (INDEX.md standing defect)? No demux-crate test asserts language (F-005) — the mp4 mod.rs hdlr/mdhd language parsing (or its default) is the suspect site.
- REQUEST-ORCH-RUN (optional, would close F-001 for the mp4 lanes): `LUMEN_CORPUS=<restored-corpus-dir> cargo test --release -p demux mp4::tests` plus `LUMEN_CORPUS=<dir> cargo test --release containers::mp4::tests::corpus_iso_bmff_lanes` — or commit samples/sample-420.mp4 (+faststart) as in-repo fixtures for the differential.
