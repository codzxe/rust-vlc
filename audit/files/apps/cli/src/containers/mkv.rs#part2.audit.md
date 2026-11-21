# AUDIT: apps/cli/src/containers/mkv.rs#part2 (lines 359-1303)
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = 359-1303 (`#[cfg(test)] mod tests` through EOF). Part1 = 1-358 (module doc + production: `open`, `harvest_facts`, `config_blob`, `vorbis_chain`, `flac_streaminfo`, `audio_pair`, `dimensions_of`) belongs to W13-A09. Seam picked at the `mod tests` attribute so no agent re-audits a range. |
| Lines / functions | 945 / 17 (4 helpers, 1 sink impl, 8 `#[test]` fns, 1 truth-table struct+const, 2 corpus-gate helpers, plus `CountingSink::accept`) |
| File verdict | REAL/REAL-UNPROVEN — fixture- and unit-based tests are REAL (executed, baseline-cited); the 4 corpus-gated tests (incl. the 34-lane differential, "the user's standing mandate") pass VACUOUSLY when the corpus is absent, and the corpus is absent in this sandbox and untracked in the repo |
| Reachable from shipped entry point? | N/A for a `#[cfg(test)]` module — not compiled into `lumen`. Reached by `cargo test -p lumen-cli` (baseline `orchrun-test-workspace-nofailfast.log.txt:2803-2811,2900`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `Read` mkv.rs 1-1304 (full); `ls tests/fixtures/generated/valid | grep -iE 'mkv\|webm'` (4 fixtures exist: mkv_h264_aac.mkv, mkv_h264_flac.mkv, mkv_multitrack.mkv, webm_vp9_opus.webm); `rg "corpus_matroska\|mkv::tests" audit/runs/baseline/` (all 10 tests "ok" in orchrun-test-workspace-nofailfast.log.txt:2803-2811,2900 — pass/skip indistinguishable); `[ -d /tmp/my-project/corpus-v4/media ]` → MISSING (also `/home/z/my-project/corpus-v4`, `/home/z/my-project/worktrees/w5-16/corpus-v4` missing); `rg "LUMEN_CORPUS\|corpus-v4" audit/runs/baseline/` (only hit: `corpus_dirty_outcome_classes_hold ... ignored, randomized-corpus tier 2: set LUMEN_CORPUS to activate` — the honest #[ignore] convention this file does NOT use); `grep corpus-v4 docs/worklog/archive-2026-10.md` (line 8045: "corpus-v4/ media (126 files, 66 MiB) lives untracked in worktrees/w5-16"); `grep` corpus manifests: `docs/corpus/corpus-v4-smoke-manifest-20261023.json` (18 unique matroska media lanes, numbering 0005/0015/0016…) and `corpus-big-manifest-20261022.json` (333 mkv/mka/webm mentions, numbering 0002/0003/0006…) — BOTH disjoint from TRUTHS' numbering (0002-audio-only.mkv, 0007-chapter-bomb.mkv…); `sed -n` on every quoted range (verified verbatim). |

## 1. One-paragraph verdict
The fixture-based half of this test module is genuinely hostile and genuinely executed: five tests drive the REAL CLI open path (`open_source` → `containers::mkv::open`) against four committed fixtures and assert measured facts (avcC version byte, 5-byte ASC with AOT 2, 34-byte STREAMINFO, 88 200 decoded sample frames, subtitle deselection, OpusHead sniff, 20 vp9 frames), and the two unit tests for `vorbis_chain`/`flac_streaminfo` cover the magic gates, 0xFF accumulation, wrong block type, and lying length — all "ok" in the baseline no-fail-fast run. The corpus half is where the module lies about its own evidence: four tests gate on `corpus_dir()`, and when the gate fails they `eprintln!` and `return` — i.e., they report **pass** to the harness while asserting nothing. The corpus is not in the repo (worklog archive-2026-10.md:8045: media "lives untracked in worktrees/w5-16"), does not exist at the baked-in default `/tmp/my-project/corpus-v4/media` in this sandbox, and no committed corpus inventory uses the TRUTHS table's file numbering — so the 34-lane ffmpeg differential, the `--start-time`/Cues test, the 6-channel AAC refusal test, and the Vorbis-lane decode test cannot be shown to have executed in the baseline at all, and their "ok" lines are consistent with zero assertions having run. The repo itself demonstrates the honest alternative (`#[ignore]` + "set LUMEN_CORPUS to activate", baseline log line 2543); this file chose the silent variant.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2804-2811` → `containers::mkv::tests::{flac_fixture_opens_and_decodes_the_full_stream, flac_streaminfo_extracts_the_verified_block, h264_aac_fixture_opens_with_expected_facts, multitrack_fixture_deselects_subtitles_at_open, six_channel_aac_geometry_flows_but_the_decoder_refuses, start_time_skips_packets_and_reports_indexed_seek, vorbis_lacing_strip_joins_the_measured_packing, vorbis_lane_decodes_the_full_stream_from_the_laced_private, corpus_matroska_lanes_match_the_ffmpeg_truth}` all `ok`; `:2900` → `webm_vp9_and_opus_decode_from_the_private_head ... ok`. CAVEAT (F-001): the four corpus-gated names in that list pass identically when skipped, so "ok" proves execution only for the 6 fixture/unit tests.
— Verified by `ls tests/fixtures/generated/valid/` → the four referenced fixtures exist and are committed; the fixture tests' assertions are hard (no skip path in them: `open()` helper panics via `unreachable!("{err}")` on any open failure, mkv.rs:383).
— Verified by read: tests exercise the REAL wiring — `open()` calls `crate::demux_host::open_source` (380-385) and `decoder_for()` builds the real `HostDecoderProvider` (412-426), not a mock of the subject under test.

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/containers/mkv.rs:1083-1098 (gate), 636-639 / 673-676 / 1102-1105 / 1245-1248 (four call sites)
Code:
```rust
    fn corpus_dir() -> Option<PathBuf> {
        // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
        if let Ok(dir) = std::env::var("LUMEN_CORPUS") {
            return Some(PathBuf::from(dir));
        }
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
    }

    fn skip_loudly() {
        eprintln!(
```
What is wrong: When the corpus is absent (the default state outside the original authoring sandbox) the four corpus-gated tests — `start_time_skips_packets_and_reports_indexed_seek` (636-639), `six_channel_aac_geometry_flows_but_the_decoder_refuses` (673-676), `corpus_matroska_lanes_match_the_ffmpeg_truth` (1102-1105, the 34-lane differential the comment at 756 calls "the user's standing mandate"), and `vorbis_lane_decodes_the_full_stream_from_the_laced_private` (1245-1248) — print to stderr and `return`, which the harness records as **pass**, not ignored. The corpus is untracked (docs/worklog/archive-2026-10.md:8045: "corpus-v4/ media (126 files, 66 MiB) lives untracked in worktrees/w5-16 — promote to a shared read-only location if other cards need it") and is absent in this sandbox (`/tmp/my-project` exists, `/tmp/my-project/corpus-v4` does not; `/home/z/my-project/worktrees/w5-16/corpus-v4` does not).
Why it matters: The baseline's 1681-passed count and this module's "ok" lines cannot distinguish executed from vacuously-skipped for these four tests; the strongest evidence in the file (a 34-lane ffmpeg differential + a real `--start-time`/Cues assertion) may have contributed zero assertions to the recorded evidence. This is exactly the "test proving nothing used as evidence" class. The repo's own sibling test shows the honest convention — baseline log orchrun-test-workspace-nofailfast.log.txt:2543: `test corpus_dirty_outcome_classes_hold ... ignored, randomized-corpus tier 2: set LUMEN_CORPUS to activate`.
What correct looks like: `#[ignore = "set LUMEN_CORPUS to activate"]` on the four tests (harness reports `ignored`, evidence stays honest), or make the corpus a committed fixture set, or assert-fail when a lane file named in TRUTHS is missing while the dir exists.
Confidence: HIGH — Verification: read + sandbox dir checks + baseline log grep. UNVERIFIED only in the narrow sense that I cannot prove whether the baseline run had the corpus mounted (nothing in the logs either way).

### F-002 [S3]
Where: apps/cli/src/containers/mkv.rs:789-1079 (TRUTHS table), comment 755-767
Code:
```rust
    const TRUTHS: [Lane; 34] = [
        // 0002: ac3-only — every track refuses typed (no ac3 decoder).
        Lane {
            file: "0002-audio-only.mkv",
```
What is wrong: The table claims "every Matroska lane of corpus-v4 … the same ffmpeg-measured truth as crates/demux/tests/mkv_corpus.rs … measured this session", but no committed inventory matches its numbering: `docs/corpus/corpus-v4-smoke-manifest-20261023.json` lists 18 unique matroska media lanes numbered 0005/0015/0016/0023…, `docs/corpus/corpus-big-manifest-20261022.json` numbers run 0002-muxed-av.webm / 0003-muxed-av.mkv / 0006-multi-av.mkv — neither contains "0002-audio-only.mkv", "0007-chapter-bomb.mkv" (TRUTHS' names), etc. The measured corpus instance exists only untracked (F-001).
Why it matters: The "truth" the differential is checked against is unreproducible from the repo — a fresh clone cannot regenerate the exact instance (the committed manifests' seeds produce different numbering), so the table's 34 rows are unauditable numbers. Any drift in the demuxer vs the real corpus can never be re-verified by a third party.
What correct looks like: commit the corpus instance's manifest (file list + ffprobe truths) the table was measured against, or generate the lanes from a committed seed as sibling corpus tooling does.
Confidence: HIGH — Verification: read + grep of both committed corpus manifests (numbering disjoint from TRUTHS); the corpus media itself unavailable → table contents UNVERIFIED.

### F-003 [S3]
Where: apps/cli/src/containers/mkv.rs:820, 843, 866, 909, 930, 944, 974
Code:
```rust
            streams: &[("h264", 60), ("aac", 153)],
            audio_frames: Some(153 * 1024),
```
What is wrong: For 7 of the 12 audio-decoding lanes the "decoded sample frames" expectation is computed as the same lane's asserted packet count × 1024 (`153*1024`, `27*1024`, `3638*1024`, `4208*1024`, `35*1024`, `3332*1024`, `89*1024`) — arithmetic on this test's own packet-count assertion, not an independently measured ffmpeg decode truth. Only `26_543`, `39_791`, `173_448`, `88_200`, `28_489` (and `37_568` at 1300) are independent measurements.
Why it matters: The comment at 762-764 promises "decode counts where a decoder is dispatched … the mp3/vorbis/opus/alac/flac engines' own emission windows", but the AAC rows cannot detect a decoder that drops or duplicates whole packets in lockstep with the demuxer count, and they launder the packet-count assertion into a second "decode" assertion. Weak evidence presented as differential truth.
What correct looks like: hard-code the ffprobe-measured decoded-sample count per lane (as rows 0014/0078/0098/0127 do), not `packets * 1024`.
Confidence: HIGH — Verification: read of all 12 `audio_frames` values.
(Note: the ×1024 rows still catch per-packet emission-size changes — not a pure tautology, hence S3 not S2.)

### F-004 [S4]
Where: apps/cli/src/containers/mkv.rs:651-656
Code:
```rust
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|e| unreachable!("{e:?}"));
                        assert!(ns.ticks >= 500_000_000, "early packet: {}", ns.ticks);
```
What is wrong: Packets whose pts is `Stamp::Missing` silently bypass the ≥500 ms watermark assertion, so a demuxer regression stamping every post-seek packet `Missing` would still pass `start_time_skips_packets_and_reports_indexed_seek` (the `seen > 0` check at 663 counts packets regardless of stamp).
Why it matters: The one test guarding the `--start-time` skip watermark has a hole exactly where a timestamps-regression would hide; blast radius is one test.
What correct looks like: also assert that the fraction of `Missing`-pts packets after the seek is zero (or below a recorded bound).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fixture` (helper) | 373-377 | REAL | — | CARGO_MANIFEST_DIR/../../tests/fixtures — 4 referenced fixtures verified present |
| `open` (helper) | 380-385 | REAL | — | Real `open_source` path; panics on error (`unreachable!("{err}")`) — no silent pass |
| `CountingSink` + `accept` | 390-407 | REAL | — | Audio: frames+channels; video: +1; subtitle: ignored — matches what each test asserts |
| `decoder_for` (helper) | 412-426 | REAL | — | Real `HostDecoderProvider`; no mock of the subject under test |
| `h264_aac_fixture_opens_with_expected_facts` | 435-475 | REAL | — | Executed (baseline :2806); asserts avcC[0]==1, dims, 10 fps cadence, 5-byte ASC AOT 2, geometry pair, duration None — would fail on harvest regression |
| `flac_fixture_opens_and_decodes_the_full_stream` | 483-525 | REAL | — | Executed (:2804); 20 packets → 88 200 frames end-to-end decode |
| `multitrack_fixture_deselects_subtitles_at_open` | 532-560 | REAL | — | Executed (:2807); counts [20,88,20,2] per track; subtitle deselection asserted |
| `webm_vp9_and_opus_decode_from_the_private_head` | 570-628 | REAL | — | Executed (:2900); vp9 20 frames + opus 96 648 frames through real decoders |
| `start_time_skips_packets_and_reports_indexed_seek` | 634-664 | REAL-UNPROVEN | F-001, F-004 | Corpus-gated; vacuous pass when corpus absent; Missing-pts hole |
| `six_channel_aac_geometry_flows_but_the_decoder_refuses` | 671-687 | REAL-UNPROVEN | F-001 | Corpus-gated; asserts refusal code `aac.engine.init` — good shape, unproven execution |
| `vorbis_lacing_strip_joins_the_measured_packing` | 692-725 | REAL | — | Executed (:2810); covers join, magic-gate refusal, 0xFF accumulation, unterminated lacing — would fail on any `vorbis_chain` mutation |
| `flac_streaminfo_extracts_the_verified_block` | 730-753 | REAL | — | Executed (:2805); wrong-type, lying-length, bare-34 refusals all asserted |
| `Lane` struct + `TRUTHS` const | 773-1079 | REAL-UNPROVEN | F-002, F-003 | 34 lanes; numbering matches no committed corpus manifest; 7 AAC rows are packets×1024 |
| `corpus_dir` | 1083-1090 | REAL | F-001 | Gate; default path absent in this sandbox; env-overridable |
| `skip_loudly` | 1092-1098 | REAL | F-001 | eprintln + caller returns Ok → harness records pass, not ignored |
| `corpus_matroska_lanes_match_the_ffmpeg_truth` | 1100-1234 | REAL-UNPROVEN | F-001 | 34-lane differential: codecs, packet counts, seekability, decode counts, typed-refusal cross-check (1152-1160), vp9 emit-seam drop rule (1193-1198) — well-built, execution unproven; `counts[p.track.handle() as usize]` panics on non-contiguous handles (fail, acceptable in test) |
| `vorbis_lane_decodes_the_full_stream_from_the_laced_private` | 1243-1302 | REAL-UNPROVEN | F-001 | Corpus-gated; asserts magics, >3 000-byte codebooks, geometry sniff, 37 568 frames |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| h264_aac_fixture_opens_with_expected_facts | YES | Committed fixture + hard asserts on harvested bytes/dims/cadence; no skip path; executed in baseline |
| flac_fixture_opens_and_decodes_the_full_stream | YES | End-to-end decode to exact sample count on committed fixture |
| multitrack_fixture_deselects_subtitles_at_open | YES | Exact per-track packet counts + deselection subset assert |
| webm_vp9_and_opus_decode_from_the_private_head | YES | Two real decoders to exact frame counts |
| vorbis_lacing_strip_joins_the_measured_packing | YES | Unit-level; 4 refusal/accumulation cases; mutation of `vorbis_chain`'s magic gate or 0xFF loop breaks it |
| flac_streaminfo_extracts_the_verified_block | YES | Wrong-type/lying-length refusals asserted |
| start_time_skips_packets_and_reports_indexed_seek | NOT PROVABLY | Corpus-gated — returns pass when corpus absent (F-001); when executed, only Present-pts packets checked (F-004) |
| six_channel_aac_geometry_flows_but_the_decoder_refuses | NOT PROVABLY | Corpus-gated (F-001) |
| corpus_matroska_lanes_match_the_ffmpeg_truth | NOT PROVABLY | Corpus-gated (F-001); AAC decode rows partially self-referential (F-003) |
| vorbis_lane_decodes_the_full_stream_from_the_laced_private | NOT PROVABLY | Corpus-gated (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "corpus differential … every Matroska lane of corpus-v4" (module comment 755-757) | mkv.rs itself | UNVERIFIED | Corpus absent/untracked; committed manifests numbering-disjoint (F-002); test passes vacuously without it (F-001) |
| Subtitle tracks "never block playback" (doc §3 wiring, per comment 108-111) | module doc part1 | REAL | `multitrack_fixture_deselects_subtitles_at_open` executed ok (baseline :2807) — 4 facts, 3 engine tracks, no Subtitle kind |
| vp9 "DECODES … the Lead-reconciled plain dispatch" (comment 948-950) | mkv.rs | REAL | `webm_vp9_and_opus…` 20/20 frames, baseline :2900 |
| Baseline 1681-passed as evidence these tests ran | audit/runs/baseline/INDEX.md #3 | PARTLY FALSE | 6 of 10 test names provably ran on committed fixtures; 4 corpus-gated names' "ok" is skip-compatible (F-001) |

## 7. Dead / duplicated / unreachable code
None in this range. `CountingSink`/`decoder_for`/`open`/`fixture` helpers are all used. The corpus-gate helper pair duplicates the same 6-line `corpus_dir`/skip pattern as crates/demux/tests/{mkv_corpus,mkv_attachments_chapters,mkv_editions_tags}.rs (4th copy across the repo — S4-level duplication, folded into F-001's fix scope).

## 8. Dependencies & flags
Test-only deps: `codec_api::{DecoderConfig, FrameSink, ThreadBudget}`, `commands::catalog::types::SourceSpec`, `media_types::{EngineError, Frame, GenerationId}`, `playback_engine::DecoderProvider`, `plugin_api::{DemuxEvent, SeekSupport}` — all workspace crates; no new external deps; `#[cfg(test)]` gates the whole range out of the shipped binary. Two `lumen-allow-literal` waivers at 1084/1088/1094 acknowledge the hard-coded path (see F-001).

## 9. Open questions for the next agent
- Did the baseline run have `/tmp/my-project/corpus-v4/media` mounted? REQUEST-ORCH-RUN: `LUMEN_CORPUS=<regenerated corpus> cargo test -p lumen-cli --release containers::mkv` would convert the four REAL-UNPROVEN rows to REAL — needs the corpus regenerated from a committed seed first (none matches TRUTHS numbering — F-002).
- W13-A09 (mkv.rs part1): `vorbis_chain`'s `lens.len() + 1 != MAGIC.len()` assumes exactly 3 headers; my unit-test read confirms behavior, but the `count > 8` bound at 275 deserves your severity call.
