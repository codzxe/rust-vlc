AUDIT: apps/cli/src/containers/ivf.rs
======================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 687 / 16 (2 production fns, 14 test items) |
| File verdict | REAL-UNPROVEN (code verified by static trace + tests read; not executed — no cargo per adaptations) |
| Reachable from shipped entry point? | YES — `lumen open x.ivf` → run.rs:132 repl_mode → dispatch → host.rs open path → `demux_host::open_source` → probe names `ivf` → **this file's `open`** (line 77). Chain confirmed by `containers/mod.rs` registry (read) and tests calling the same `open_source`. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (2 chunks)
- `ls crates/codecs/tests/fixtures/av1/ tests/fixtures/generated/ivf/` → all 7 fixtures the lane table names EXIST on disk (5 av1 + av1_64x48_30fps + vp9_64x48_25fps)
- `sed -n '85,105p' apps/cli/src/containers/ivf.rs` (quote verification, passed)
- Cross-read: pipeline_host.rs:304-317 (`av1`/`vp9` decoder arms), ttf_seek.rs:357-362 (the binary-level vp9 lane — see open questions).

## 1. One-paragraph verdict

A container host that does exactly what its module doc claims and pins it with unusually honest tests: the facts harvest mirrors the IVF header (fourcc-honest codec, tick base, coded dims), the seek path uses the demuxer's pure query with no invented clamps, and the test suite drives the REAL open path plus the REAL decoder provider — including the 10-bit AV1 lane asserted to refuse typed `av1.bitdepth` on every TU (a refusal pinned as a refusal, not hidden). The production half is 100 lines of wiring with no arithmetic of its own beyond µs→ns saturating multiplies; the remaining 500 lines are the lane table and tests. No fakeness found. The only real gap is a cross-file one: this file proves vp9 DECODES through the provider while ttf_seek.rs still asserts the binary-level vp9 lane refuses at the renderer — one of the two files describes a dead landscape (see open questions).

## 2. What actually works (verified)

— verified by static trace + fixture existence check (`ls` above): `open` (77-123) constructs `IvfDemuxer`, runs `start()`, harvests facts, validates `--start-time` via `demuxer.seek` (103), wraps in `SelectDemuxer` (106-116). Negative start times clamp to 0 (89); past-the-end targets are documented to land-and-admit-nothing (91-95).
— verified by reading tests 308-651: `lanes_open_with_expected_facts` asserts the full facts contract per lane (codec, base, dims, no pcm, no blob, no extent, frame_ns tick math at 337-343, SlowLinear seekability); `lanes_demux_the_full_frame_count` asserts the pts ladder `0..N−1` and per-packet duration; `av1_lanes_decode_the_full_stream` asserts real decode counts (6/18/60/18) through `HostDecoderProvider` and the tenbit refusal (5 refusals, `av1.bitdepth`, 0 frames); `vp9_lane_decodes_the_full_stream` asserts 25 packets → 25 frames; `start_time_skips_packets` asserts the 1 s target keeps exactly the second half (30 of 60) and a past-the-end target admits 0 packets; crafted header-only and VP80 refusals assert exact codes.
— UNVERIFIED at runtime (no cargo): the test bodies' assertions are read-verified only.

## 3. Findings

### F-001 [S3] — this file proves vp9 decodes; ttf_seek still pins a vp9 binary-level refusal — unresolvable contradiction inside one crate
Where: apps/cli/src/containers/ivf.rs:480-513 (vp9 decode lane) vs apps/cli/tests/ttf_seek.rs:357-362 (ivf/vp9 lane `refusal: Some("cli.i420_color_unsupported")`)
Code:
```rust
    /// The VP9 lane RECONCILED (card 6.M-vp9 landed — the Lead
    /// inversion per the ogg/opus precedent): the file opens, the
    /// facts are true (fourcc-honest `vp9` descriptor, dims, tick
    /// base), the packets flow — and rusty_vp9 decodes the full
    /// stream 1:1 through the real provider (no seeding: in-band
```
What is wrong: not this file's code — the contradiction. This test (487-513) proves the full vp9 stream decodes 1:1 through the same provider the binary uses; ttf_seek.rs:357-362 asserts the binary-level ivf/vp9 session refuses at the present limb (`cli.i420_color_unsupported`), which after pipeline_host.rs:1475-1488's four-pair conversion (BT.709 limited included — vp9's default) should no longer fire. One of the two is stale; the corpus gate (ttf F-001) guarantees the stale one never runs.
Why it matters: the repo's own through-the-binary landscape (ttf) and its unit-level truth (this file) disagree about whether a vp9 file plays; any consumer of the "honest landscape" doc inherits the wrong answer.
What correct looks like: re-measure the ttf lane table; if vp9 now presents, update both files' docs in one change.
Confidence: MEDIUM — Verification: read (both files + pipeline_host; the actual vp9 frame color tag is UNVERIFIED — crates/codecs is another wave's file).
### F-002 [S4] — fixed temp filenames in tests; leftover files on panic paths
Where: apps/cli/src/containers/ivf.rs:577, 594, 620
Code:
```rust
        let path = std::env::temp_dir().join("lumen-cli-ivf-header-only.ivf"); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
```
What is wrong: three fixed scratch names in the shared temp dir. If a test panics between write and `remove_file` (e.g. `unreachable!` at 580), the file is left behind and a subsequent run opens the STALE file (contents are deterministic here, so masked — but two concurrent `cargo test` runs on one machine race the same paths).
Why it matters: hygiene + a latent parallel-run flake.
What correct looks like: `tempfile` crate or pid-suffixed names.
Confidence: HIGH — Verification: read.

### F-003 [S4] — `Truth` lane numbers claim ffprobe provenance that cannot be re-verified from the tree
Where: apps/cli/src/containers/ivf.rs:237-257
Code:
```rust
    /// One lane's recorded truth (ffprobe, measured this session:
    /// `codec_name,width,height,r_frame_rate,time_base,nb_frames` —
    /// the same commands as the demux suite's fixture table).
```
What is wrong: "measured this session" has no recorded artifact (no ffprobe output committed next to the fixtures); the numbers are plausible (frame counts match fixture names like `_60f`) but their provenance is a comment.
Why it matters: evidence-chain hygiene: the numbers pin real behavior, but the claim of measurement is unverifiable after the fact.
What correct looks like: commit the ffprobe dump beside the fixture or generate it in the test when ffmpeg exists.
Confidence: HIGH — Verification: read (numbers cross-check against filenames where derivable: 6f/18f/5f/60f/18f/30/25 — consistent).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` (production) | 77-123 | REAL-UNPROVEN | — | wiring only; negative start clamp (89); saturating µs→ns (98, 104, 120); no panics; no arithmetic hazards. |
| `harvest_facts` (production) | 133-180 | REAL-UNPROVEN | — | debug_assert invariants (145-154) vanish in release — acceptable (they pin demuxer agreement, both sides its own crate); `frame_ns` from `tick_nanos()` (158); facts per track, video-only, no blob. |
| tests `av1_fixture` | 200-204 | REAL | — | codecs-crate fixture path (exists — verified by ls). |
| tests `generated_fixture` | 209-212 | REAL | — | testkit fixture path (exists — verified by ls). |
| tests `CountingSink` | 217-226 | REAL | — | counts frames + last size; no-op accept Ok. |
| tests `open` helper | 230-235 | REAL | — | through the real `open_source`. |
| tests `Truth` + `lanes` | 240-295 | REAL | S4 | F-003 provenance. |
| `lanes_open_with_expected_facts` | 308-351 | REAL | — | would fail on facts drift (codec/base/dims/pcm/blob/extent/frame_ns/seekability/start all asserted). |
| `lanes_demux_the_full_frame_count` | 358-399 | REAL | — | pts ladder + durations + counts asserted; Recovered/Discontinuity must not occur on clean fixtures. |
| `av1_lanes_decode_the_full_stream` | 409-464 | REAL | — | decode counts + tenbit typed-refusal pin (`av1.bitdepth`, 5 refusals, 0 frames). |
| `generated_av1_lane_decodes_the_full_stream` | 470-478 | REAL | — | 30→30. |
| `vp9_lane_decodes_the_full_stream` | 486-513 | REAL | S3 | F-001 cross-file contradiction with ttf_seek. |
| `start_time_skips_packets` | 520-563 | REAL | — | exact 30/60 split; past-the-end admits 0; `i64::MAX/8` target unclamped (548-552) — honest no-duration shape. |
| `crafted_shapes_refuse_at_the_demux_stage` | 573-607 | REAL | S4 | exact refusal codes asserted; F-002 temp names. |
| `non_unit_scale_timebase_is_the_containers_own_ratio` | 616-651 | REAL | — | NTSC 30000/1001: base (1001, 30000) verbatim, frame_ns 33_366_666 floored — would catch tick-math regressions. |
| `decode_all` | 657-686 | REAL | — | real provider + decoder; typed refusal RETURNED; `let _ = decoder.decode(...)` discards only the outcome enum (errors still `?`). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lanes_open_with_expected_facts | YES | full facts contract per lane, fixtures committed and present. |
| lanes_demux_the_full_frame_count | YES | exact counts + pts ladder + duration presence. |
| av1_lanes_decode_the_full_stream | YES | exact decode counts + refusal-by-name on tenbit. |
| generated_av1_lane_decodes_the_full_stream | YES | 30→30 + size. |
| vp9_lane_decodes_the_full_stream | YES | 25→25 + size (and it is the file that exposes F-001's contradiction). |
| start_time_skips_packets | YES | exact 30-packet survivor count + 0-packet past-the-end. |
| crafted_shapes_refuse_at_the_demux_stage | YES | exact stage + code strings. |
| non_unit_scale_timebase_is_the_containers_own_ratio | YES | exact base tuple + ns figure. |

This is the strongest test file audited in this batch: no corpus gate, committed fixtures, refusals pinned as refusals.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "AV1 **decodes** (the rav1d-backed `Av1Decoder` …); VP9 **decodes** too (the rusty_vp9-backed `Vp9Decoder` …)" | ivf.rs:18-23 | REAL-UNPROVEN | decode arms at pipeline_host.rs:304-317; decode tests 409-513 assert the counts; not run here. |
| "The 10-bit lane is the honest exception: every TU refuses typed `av1.bitdepth`" | ivf.rs:405-408 | REAL-UNPROVEN | test 429-463 asserts exactly this. |
| "Seek is `SlowLinear`" | ivf.rs:50-53 | REAL-UNPROVEN | asserted at 344-348. |
| "No `.ivf` file exists in the corpus (verified)" | ivf.rs:198-199, 255-256 | UNVERIFIED | corpus absent in this sandbox; consistent with ttf_seek needing codec fixtures for its ivf lanes. |

## 7. Dead / duplicated / unreachable code

None found. `debug_assert_eq` invariants (145-154) are debug-only by design (documented as demuxer-agreement pins).

## 8. Dependencies & flags

- Production path: `demux::ivf::IvfDemuxer`, `plugin_api::Demuxer/SourceStream`, crate `demux_host` types. No feature gates. The vp9/av1 decode arms it relies on (pipeline_host.rs:304-317) are unconditional in the crate (no cfg gate found) — so the shipped binary carries these decoders.

## 9. Open questions for the next agent

1. crates/codecs rusty_vp9 wrapper — what ColorDescription does it stamp (Bt601 full? Bt709 limited?)? Decides F-001's resolution direction (does a vp9 file PLAY through the binary today?).
2. demux::ivf `tick_nanos` — flooring behavior for non-unit scale (33_366_666 asserted here); the demux wave should confirm the floor vs round choice matches ffmpeg's duration convention.
