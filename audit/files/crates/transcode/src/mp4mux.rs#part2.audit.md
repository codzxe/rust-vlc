# AUDIT: crates/transcode/src/mp4mux.rs#part2 (lines 466-912)
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 466-912 (chunk boundary declared: part1 = 1-465, module docs + constants + configs + `Mp4Remuxer` API (`new`/`add_packet`/`finalize`/`write_ftyp`/`write_mdat_header`/`build_moov`) + `put_mvhd`/`put_tkhd` [A06]; part2 = 466-912, `put_mdhd`/`put_hdlr`/`put_stbl` + the sample-table math + writers/helpers + the entire test module [A09]) / 26 items (13 production, 13 test) |
| File verdict | UNREACHABLE (real, careful muxer with a genuine demuxer round-trip suite — but the transcode crate has zero dependents; nothing shipped constructs it) |
| Reachable from shipped entry point? | NO — `rg -l "Mp4Remuxer"` → mp4mux.rs, lib.rs (re-export), record.rs (doc), runner.rs:767 (the only constructor, inside `run_mp4_remux`). The transcode crate appears in NO other crate's or app's Cargo.toml (session-start sweep: `transcode = { path` → zero hits). `lumen`/`lumen-tauri` cannot reach one byte of this file. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -l "Mp4Remuxer" crates/ apps/ tools/` (4 files, all in-crate); `rg "transcode = \{ path" **/Cargo.toml` (zero dependents); read of mp4mux.rs:95-913 (part2 complete + part1 context); cross-reads: runner.rs:711-819 (`run_mp4_remux` — the codec/timescale guards + finalize contract), tests/mp4_remux.rs structure (`progressive_source_remuxes_and_redemuxes_packet_identical`, `fragmented_source_remuxes_into_a_progressive_output` — the real round-trip suite), media-types/src/time.rs:134-148 (`Timestamp.ticks: i64` — negative cts is real); matrix rows playback-record/ui-dialog-convert (not_started, honest); `sed -n` quote verifies at 523-531, 553-561, 509-511, 691-698; baseline: transcode unit tests NEVER RAN (both baseline runs abort before the transcode test binary) |

## 1. One-paragraph verdict
This is a competently built progressive-MP4 stream-copy muxer: correct ISO 14496-12 box layouts (I hand-verified the tricky ones — `mdhd` v0/v1 split, the identity matrix, and the `VisualSampleEntry` width/height at payload offsets 24/26, which requires counting `pre_defined` as u32[3]), honest typed refusals in the direct API, ceilings on every table, and a round-trip suite that re-demuxes the output with the real demuxer and asserts packet identity. Its defects are two silent-truncation paths in the sample tables (`stts` deltas and `ctts` offsets over u32::MAX become 0/wrapped instead of saturating or refusing — contradicting the crate's own documented saturation policy), a release-build fallback that mislabels a non-h264/aac entry as `mp4a`, and a `put_box` size field that would lie for a >4 GiB box. All of it is unreachable from the shipped binary — the transcode crate has no consumers, and the matrix honestly marks recording/convert not_started.

## 2. What actually works (verified)
— `put_mdhd` (466-483): v0 while the duration fits u32, v1 (0x0100_0000 + 8-byte creation/modification/duration) otherwise — layout verified against ISO 14496-12 §12.1.2; language `0x55C4` = 'und' packed 5-bit ('u'=0x15<<10 | 'n'=0x0E<<5 | 'd'=0x04) — verified by hand.
— `put_hdlr` (486-496): vide/soun, 12 reserved bytes, NUL-terminated empty name — correct.
— `put_stbl` (499-610): `stsd` carries the verbatim sample entry under the derived fourcc; `stts` run-length merging (`last_mut().filter(last.1 == delta)`) traced on a 2-sample case; `ctts` emitted only when a nonzero offset exists, version 1 with i64 offsets iff any offset is negative (possible: `Timestamp.ticks` is i64, media-types time.rs:138, and `add_packet`'s `pts.ticks.saturating_sub(dts)` is i64 math — consistent); `stss` only for video and only when not all-sync (absence = all-sync per §8.6.2 — correct default); `stsc` fixed 1-sample-per-chunk matching the per-sample `stco`; sizes/offsets are u32 by construction (add_packet's `u32::try_from` guards, part1 :281-293).
— `video_dimensions` (680-688): offsets 24/26 verified against the VisualSampleEntry body-after-8-byte-header layout (reserved[6] + data_reference_index[2] + pre_defined[2] + reserved[2] + pre_defined u32[3]=12 → width at 24) — CORRECT, with a `len >= 28` bound so short payloads answer (0,0), no panic.
— `nanos_to_ticks` (614-620): `(nanos·ts + 5e8)/1e9` — nearest, ties up; zero-guards.
— `track_movie_duration` (647-656): media ticks → movie units with round-to-nearest — correct.
— `put_identity_matrix` (659-669): 0x00010000/0x00010000/0x40000000 — the correct 16.16 identity.
— The test module's `Cursor` (712-768) is a REAL `Write+Seek` (resize-on-overwrite, three SeekFrom arms) — a legitimate sink double; `the_sample_table_ceiling_is_typed` (:886) pre-fills a full 1 Mi row table white-box rather than materializing a gigabyte of samples — honest and documented.

## 3. Findings

### F-001 [S3]
Where: crates/transcode/src/mp4mux.rs:523-531 (`put_stbl`, stts)
Code:
```
        let mut runs: Vec<(u32, u32)> = Vec::new();
        for delta in sample_deltas(state) {
            let delta = u32::try_from(delta).unwrap_or(0);
```
What is wrong: a per-sample `stts` delta larger than u32::MAX (a DTS gap > ~13.25 h at a 90 kHz timescale — craftable from a hostile source file's packet timeline; `add_packet` accepts any monotonic u64 DTS) is silently written as **0** — a valid-looking value that collapses the sample's duration to zero. The crate's own documented overflow policy is saturation (`put_u32_saturating` :671-672 "saturating is the crate's overflow policy"), and `add_packet` refuses size overflows typed; this path neither saturates nor refuses.
Why it matters: silent timeline corruption on crafted input, in the one place the muxer translates source time into container time. A remuxed hostile file plays as a zero-duration blob with no error anywhere.
What correct looks like: `put_u32_saturating`-style saturation, or a typed refusal at finalize when any delta exceeds the field (the muxer already has `mp4_size_overflow` as the pattern).
Confidence: HIGH — Verification: read (523-531, 671-675; add_packet's u64 dts path part1 :253-303 read; Timestamp.ticks i64 verified in media-types).

### F-002 [S3]
Where: crates/transcode/src/mp4mux.rs:553-561 (`put_stbl`, ctts v0)
Code:
```
                if negative {
                    body.extend_from_slice(&offset.to_be_bytes());
                } else {
                    body.extend_from_slice(&(offset as u32).to_be_bytes());
                }
```
What is wrong: in the non-negative branch a composition offset > u32::MAX is written through a truncating `as u32` cast — it wraps modulo 2³² instead of saturating or refusing. Same crafted-input class as F-001 (cts = pts−dts in i64 ticks; a >13 h offset is craftable), same contradiction of the crate's saturation policy; the checklist's exact "`as` truncation" defect.
Why it matters: a wrapped ctts silently reorders presentation by hours-to-milliseconds with no signal; the v1 branch two lines up proves the code knows how to write wide values when it chooses to.
What correct looks like: saturate, or emit ctts v1 whenever any offset exceeds u32::MAX (the version bit is already computed from sign — extend the condition to magnitude).
Confidence: HIGH — Verification: read (540-563).

### F-003 [S4]
Where: crates/transcode/src/mp4mux.rs:509-511 (`put_stbl`, stsd fallback)
Code:
```
        let fourcc = sample_entry_fourcc(&state.setup.codec, state.setup.kind);
        debug_assert!(fourcc.is_some(), "runner refused non-h264/aac already");
        let fourcc = fourcc.unwrap_or(b"mp4a");
```
What is wrong: a direct-API caller who skips the runner's codec guard (runner.rs:745-751) and passes e.g. `(MediaKind::Video, "hevc")` gets a release-build file whose video sample entry is labeled **`mp4a`** (audio) with the verbatim payload behind it — a mislabeled track, not an "honest fallback entry kind" as the comment claims. The `debug_assert` only catches debug builds.
Why it matters: the fallback's failure mode (a video track typed audio) is worse than the alternative (refuse, as the runner does); the comment mislabels the behavior.
What correct looks like: refuse typed at `Mp4Remuxer::new` when a setup's codec is outside the stream-copy set (the guard belongs in the constructor, not the caller), or at minimum fall back to a neutral entry kind.
Confidence: HIGH — Verification: read (502-517; runner.rs:745-751).

### F-004 [S4]
Where: crates/transcode/src/mp4mux.rs:691-698 (`put_box`)
Code:
```
    let size = u32::try_from(buf.len() - start).unwrap_or(u32::MAX);
    buf[start..start + 4].copy_from_slice(&size.to_be_bytes());
```
What is wrong: a box whose body exceeds 4 GiB−1 gets a `u32::MAX` size field while the buffer holds more — the size lies. Unreachable under today's ceilings (1 Mi samples/track → ~10 Mi `moov`), so this is latent, but it is the same silent-corruption shape as F-001 with no typed refusal.
Why it matters: the ceilings are `[PROPOSAL]` constants (:103-105); raising `MAX_REMUX_SAMPLES` (or track count) without auditing this path turns it live.
What correct looks like: a typed overflow error threaded through the writers, or an explicit ceiling check at finalize.
Confidence: HIGH — Verification: read (691-698, 103-111).

### F-005 [S4]
Where: crates/transcode/src/mp4mux.rs:712-912 (test module) + crates/transcode/tests/mp4_remux.rs
What is wrong: none of this executed in the committed baseline — both baseline test runs abort before the transcode test binary. Worse, `tests/mp4_remux.rs` — the round-trip suite that the moov comment (:410-413 "the round-trip suite pins it") and the module docs lean on for the claim that the produced files are demuxable — is in NO wave-10 agent's assignment (the wave's transcode files are src/ only), so its own audit coverage is a hole too.
Why it matters: the muxer's strongest evidence (remux → re-demux → packet-identical) is real code that neither ran in the baseline nor is assigned to any auditor.
What correct looks like: `cargo test -p transcode --no-fail-fast` captured (REQUEST-ORCH-RUN); assign tests/mp4_remux.rs.
Confidence: HIGH — Verification: baseline log inventories read; wave-10 tsv list read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `put_mdhd` | 466-483 | REAL | — | v0/v1 split + 'und' verified by hand. |
| `put_hdlr` | 486-496 | REAL | — | Correct layout. |
| `put_stbl` (stsd) | 499-517 | REAL | S4 | F-003 (mp4a mislabel fallback). |
| `put_stbl` (stts) | 518-538 | REAL | S3 | F-001 (delta→0 truncation); run-length merge correct. |
| `put_stbl` (ctts) | 539-564 | REAL | S3 | F-002 (`as u32` wrap); v1-iff-negative correct. |
| `put_stbl` (stss) | 565-582 | REAL | — | Absence=all-sync correct; index+1 1-based correct. |
| `put_stbl` (stsc/stsz/stco) | 583-609 | REAL | — | 1-sample chunks consistent with per-sample stco; u32 fields guaranteed upstream. |
| `nanos_to_ticks` | 614-620 | REAL | — | Nearest-rounding verified. |
| `sample_deltas` | 625-636 | REAL | — | Final-sample delta = declared else previous; single-sample-no-duration → 0 (zero-duration track — a defensible unknown-duration answer, noted). |
| `track_media_duration` | 639-643 | REAL | — | Saturating sum. |
| `track_movie_duration` | 647-656 | REAL | — | Round-to-nearest, zero-timescale guard. |
| `put_identity_matrix` | 659-669 | REAL | — | Verified constant-for-constant. |
| `put_u32_saturating` | 673-675 | REAL | — | The crate's policy — which F-001/F-002 contradict. |
| `video_dimensions` | 680-688 | REAL | — | Offsets 24/26 hand-verified against ISO 14496-12; bounded, no panic. |
| `put_box` | 691-698 | REAL | S4 | F-004 (u32::MAX lie, latent). |
| `write_all` / `seek_to` | 701-710 | REAL | — | Typed folds. |
| test: `Cursor` | 712-768 | REAL | — | Real Write+Seek semantics (all three SeekFrom arms). |
| test: `setup`/`packet` helpers | 770-793 | REAL | — | 78-byte zero entry (dims → 0×0 — no tkhd-dims assertion exists); 1/900 base. |
| test: `fourcc_mapping_is_the_stream_copy_set` | 795-806 | REAL | — | Pins mapping + cross-kind negatives — kills regressions. |
| test: `muxer_with_twin` | 808-813 | REAL | — | Harness. |
| test: `one_track_two_samples_finalizes_a_wellformed_file` | 815-858 | REAL | — | Pins ftyp/mdat/moov skeleton + size honesty; does NOT verify moov content (that is tests/mp4_remux.rs's job — F-005). |
| test: `typed_refusals_of_the_direct_api` | 860-883 | REAL | — | Three typed codes pinned. |
| test: `the_sample_table_ceiling_is_typed` | 885-911 | REAL | — | White-box 1 Mi-row prefill; honest, documented. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fourcc_mapping_is_the_stream_copy_set` | YES | Exact-match asserts incl. negatives. |
| `one_track_two_samples_finalizes_a_wellformed_file` | PARTIAL | Catches skeleton/size-patch regressions; a wrong stts/stco body would pass this test — content is pinned only by the (never-baseline-run, unassigned) round-trip suite. |
| `typed_refusals_of_the_direct_api` | YES | Error codes asserted verbatim. |
| `the_sample_table_ceiling_is_typed` | YES | Drives the exact guard arithmetic. |
| **ctts v1 (negative offsets), stss body, truncation paths** | NO TEST EXISTS | F-001/F-002 unpinned anywhere in this module; mp4_remux.rs fixtures are B-frames-and-sync-shaped but the truncation magnitudes are absent (UNVERIFIED beyond the test-name/structure scan — content audit belongs to whoever gets tests/mp4_remux.rs). |
| **Baseline execution** | **NEVER RAN** | F-005. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "saturating is the crate's overflow policy" | put_u32_saturating doc :671-672 | SHALLOW | F-001/F-002 truncate/wrap instead, two functions away. |
| "the demux side's codec mapping answers for it" (mp4a fallback) | :505-509 | SHALLOW | F-003: the fallback mislabels kind, the demuxer cannot answer a video payload behind an mp4a entry. |
| "the round-trip suite pins it" (moov well-formedness) | :410-413 | REAL-UNPROVEN | tests/mp4_remux.rs exists with the right shape (remux→re-demux→packet-identical asserts) but never ran in baseline and is unassigned. |
| playback-record / ui-dialog-convert `not_started` | docs/feature-matrix.toml:1630+/2183+ | REAL | Matches the zero-dependents reality — honest rows. |

## 7. Dead / duplicated / unreachable code
The entire file is unreachable from shipped (header). No duplication within part2. `MAX_SAMPLE_ENTRY_BYTES` (:111) is enforced in part1's territory (runner) — cross-noted.

## 8. Dependencies & flags
`media-types` (Packet/Stamp/TimeBase — ticks are i64), `crate::error` (typed constructors), `crate::graph::WriteSeek`. No feature gates. No unsafe. Workspace lints inherited.

## 9. Open questions for the next agent
- A06 (part1): `add_packet`'s 32-bit ceiling check `after + 8 > u32::MAX || mdat_payload_at + after > u32::MAX` (:285) — confirm the second term is redundant with the first for typical layouts (mdat_payload_at ≈ 40) or whether a sink that inserts bytes before mdat (a wrapped `WriteSeek` writing a header) breaks the invariant.
- Orchestrator: assign crates/transcode/tests/{mp4_remux,wav_roundtrip,record,job_lifecycle}.rs — the crate's execution evidence lives in unassigned files; REQUEST-ORCH-RUN `cargo test -p transcode --no-fail-fast --release --locked`.
