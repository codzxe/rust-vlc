# AUDIT: apps/cli/src/containers/es.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 703 / 23 (5 production + 18 test fns/tests/helpers) |
| File verdict | REAL (production half) with environment-conditional test evidence |
| Reachable from shipped entry point? | YES — `main.rs` → `run::run` → `lumen_dispatch` → `CliHost::open_session` → `open_source` → `containers::open` (mod.rs:62-66: `"flac" => es::open_flac`, `"hevc"/"h264" => es::open_annexb`, `"mp2"/"mp3" => es::open_mpeg_audio`). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |

Commands run:
- `sed -n '147,159p;296,304p' apps/cli/src/containers/es.rs` (quote verification, exit 0)
- `[ -d /tmp/my-project/corpus-v4/media ]` → **DIR ABSENT** — the corpus the majority of this file's tests gate on does not exist in this workspace
- `ls crates/codecs/tests/fixtures | rg "mono48|mpeg2_24k|mp2_48k|mp2_lsf"` → all 4 committed fixtures exist
- `rg -n "open_mpeg_audio|open_annexb|open_flac" apps/cli/src/containers/mod.rs` → wiring at 62-66

## 1. One-paragraph verdict

The production half is a thin, honest adapter over the real crates/demux demuxers (`RawMp3Demuxer`, `AnnexbVideoDemuxer`) plus one deliberately typed refusal (`open_flac` — the FLAC demuxer does not exist, and the error says exactly that). It is not a duplicate demuxer stack. The mp3/mp2 lanes are genuinely wired end-to-end (committed fixtures decode; counts pinned against ffprobe). The h264/hevc lanes open, demux with `Missing` stamps, and refuse decode typed — matching the W00 reality that no H.264/HEVC software decoder exists. The two real defects: (1) 8 of 13 tests silently no-op when the corpus dir is absent — and it IS absent here, so the h264/hevc/flac/differential evidence in this file has zero coverage in this environment and rests on an absolute sandbox path; (2) the `pcm` facts field — documented as "PCM channels + rate, when the track is PCM" — is deliberately stuffed with MP3/MP2 geometry, a cross-file contract lie held together by comments.

## 2. What actually works (verified)

- Wiring of all five arms verified (mod.rs:62-66).
- The four committed MPEG-audio fixtures exist (`crates/codecs/tests/fixtures/{mono48.mp3, mpeg2_24k.mp3, mp2_48k.mp2, mp2_lsf_22k.mp2}`); the fixture test asserts presence loudly (`assert!(path.is_file())` — unlike the corpus lanes).
- The FLAC refusal is reachable and honest: a native `.flac` file probes as `flac` (fLaC marker) and refuses at stage `probe` with the missing-demuxer gap named — verified message shape by reading; not executed (no cargo).
- The annexb "open, demux, refuse decode" posture matches the W00 baseline fact that MP4/h264 content decodes zero frames in this build.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/es.rs:297-304 (and every corpus-gated test: 426-435, 463-472, 494-504, 528-537, 552-583, 611-620, 637-646)
Code:
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
What is wrong: 8 of this file's 13 tests gate on a corpus directory whose default is an absolute, machine-specific path. Verified in this workspace: `/tmp/my-project/corpus-v4/media` is ABSENT, so `h264_corpus_lane…`, `hevc_corpus_lane…`, `flac_lanes…`, `oversized_id3_lane…`, `corpus_mpeg_audio_differential`, `mp3_start_time_watermark_is_exact`, and `h264_start_time_is_the_honest_noop` all `return` after an eprintln — they PASS while proving nothing, and are not even counted among the 25 `#[ignore]`d tests (W00 baseline).
Why it matters: the h264/hevc lanes' headline claims (26/56 ffprobe-exact packets, contiguous file coverage, typed codec refusal) have zero coverage on any machine without that corpus — including this one and, presumably, CI. Any matrix row or evidence string citing these tests is citing environment-conditional proof. The committed-fixture lanes (mp3/mp2) are the only unconditional evidence in the file.
What correct looks like: commit the corpus fixtures (they are small media samples), or mark the lanes `#[ignore = "corpus"]` so the skip is visible in the counts.
Confidence: HIGH — Verification: `ls` of the default path (absent) + read of the gating code.

### F-002 [S3]
Where: apps/cli/src/containers/es.rs:147-159 (contract at demux_host.rs:155-156)
Code:
```rust
        // The geometry pair: channels + rate from the first frame
        // header (u16 channels → the pair's u32). NOT a PCM claim —
        // the mp4-aac convention; the descriptor gates decode.
        let geometry = demuxer
            .head()
            .map(|head| (u32::from(head.channels), head.sample_rate));
        facts.push(TrackFact {
            ...
            pcm: geometry,
```
What is wrong: the `TrackFact::pcm` field is documented at its definition as "PCM channels + rate, when the track is PCM" (demux_host.rs:155-156), but this harvest stuffs it with MP3/MP2 geometry (and containers/mp4.rs:203 does the same for `aac`). The field name and struct-level contract lie; correctness now depends on every consumer gating on the codec descriptor BEFORE looking at `pcm` — a convention enforced only by comments and tests.
Why it matters: §3.5 naming that lies. The failure mode is concrete: a consumer that treats `pcm.is_some()` as "this is a PCM track" will configure a PCM decoder for an MP3 stream and decode garbage. Today's provider gates correctly (the PS VOB `pcm_dvd` refusal path proves the descriptor-first discipline), but the invariant is implicit.
What correct looks like: a separate `geometry: Option<(u32, u32)>` field (or rename `pcm` → `output_geometry`) and `pcm` reserved for actual PCM.
Confidence: HIGH — Verification: read both files.

### F-003 [S4]
Where: apps/cli/src/containers/es.rs:189-194, 206-211
Code:
```rust
    // --start-time: the watermark is set for shape-parity with the
    // audio lanes, but every Annex-B packet carries `Missing` pts —
    // the wrapper's never-drop-on-a-guess rule passes them all (the
    // module docs' honest no-op).
    let start_us = opts.start_time_us.unwrap_or(0).max(0);
```
What is wrong: for the Annex-B lanes, `--start-time 500ms` sets `start_position_ns = 500_000_000` (line 209) while the demuxer still delivers from byte 0 — the session anchors its clock at a position no content corresponds to. The comment calls the WATERMARK a no-op (true), but the anchor is not a no-op. A player UI would report 0.5 s elapsed while the first frame plays. Masked today only because no h264/hevc decoder exists to present anything.
What correct looks like: anchor at 0 when no positioning occurred, or refuse `--start-time` typed on unpositionable lanes.
Confidence: HIGH — Verification: read.
Severity: S4 (unreachable consequence until a decoder lands; then S2).

### F-004 [S4]
Where: apps/cli/src/containers/es.rs:135-143
Code:
```rust
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            head.sample_rate,
            "the sample clock is the first frame's declared rate"
        );
```
What is wrong: the harvest's sample-clock invariant is a debug-only assert (same pattern as ps.rs F-005) — a demuxer regression passes silently in release builds.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/cli/src/containers/es.rs:605-623
Code:
```rust
    /// frames whose sample-clock pts predates 1.000 s are dropped
    /// (19 frames of 576 samples @ 11025 Hz: k×576 < 11025 ⇒
    /// k ≤ 19), the walk resumes at the first frame whose pts is
```
vs the assertion at 623:
```rust
        assert_eq!(count, 59, "79 − 20 pre-target frames (k=0..=19)");
```
What is wrong: the doc comment says "19 frames" are dropped; the assertion and its own message say 20 (k=0..=19). Trivial comment arithmetic inconsistency (the assertion is the coherent one).
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: apps/cli/src/containers/es.rs:399-418
Code:
```rust
            ("mono48.mp3", "mp3", 64u64, 1u32, 73_199u64),
```
What is wrong: the mp3 decode count asserted (73,199) is the whole-frame window MINUS start delay, knowingly divergent from ffmpeg's 72,000 — the gapless end-trim is unimplemented and the divergence is documented in exhaustive comments. The assertion is honest about what it pins; the note is that "decode end-to-end" (module docs line 13) is real, but "sample-exact vs ffmpeg" is only true for the mp2 lanes. Recorded, not hidden — S4 bookkeeping.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open_mpeg_audio` | 88-121 | REAL | — | no extent to clamp against (documented); watermark without seek query (demuxer refuses seek typed) |
| `harvest_mpeg_audio` | 128-170 | REAL | S3/S4 | pcm stuffing (F-002); debug-only invariant (F-004); codec from track descriptor (no local copy) |
| `open_annexb` | 181-212 | REAL | S4 | anchor-vs-noop (F-003); typed error paths documented |
| `harvest_annexb` | 222-247 | REAL | — | codec mirrors construction codec (descriptor() crate-private — mirrored, commented); 1/25 fallback base forwarded; nothing invented |
| `open_flac` | 258-271 | REAL (typed refusal) | — | always-Err; names the missing demuxer and the landing contract; stage `probe` + format detail |
| test `codec_fixture` | 289-293 | REAL | — | |
| test `corpus_dir` | 297-304 | REAL | S3 | machine-specific absolute default (F-001) |
| test `corpus_file` | 306-308 | REAL | — | |
| test `open` / `open_refused` | 312-325 | REAL | — | through the REAL open path |
| test `walk` | 329-340 | REAL | — | single-shot, counts |
| test `CountingSink::accept` | 345-355 | REAL | — | |
| test `mpeg_audio_fixture_opens_with_expected_facts` | 363-385 | REAL | — | loud fixture presence assert; 4 lanes |
| test `mpeg_audio_fixtures_decode_the_full_stream` | 399-418 | REAL | S4 | F-006 divergence recorded |
| test `h264_corpus_lane…` | 426-458 | REAL-UNPROVEN (env-gated) | S3 | F-001; skips when corpus absent (it is) |
| test `hevc_corpus_lane…` | 463-488 | REAL-UNPROVEN (env-gated) | S3 | F-001; also asserts contiguous byte coverage |
| test `flac_lanes…` | 494-518 | REAL-UNPROVEN (env-gated) | S3 | F-001 |
| test `oversized_id3_lane…` | 528-542 | REAL-UNPROVEN (env-gated) | S3 | F-001; documents the 64 KiB probe-window gap for a 99,682-byte ID3 tag |
| test `corpus_mpeg_audio_differential` | 552-603 | REAL-UNPROVEN (env-gated) | S3 | F-001; mp3 trim divergence honestly excluded from the equality |
| test `mp3_start_time_watermark_is_exact` | 611-632 | REAL-UNPROVEN (env-gated) | S4 | F-001 + comment math (F-005) |
| test `h264_start_time_is_the_honest_noop` | 637-650 | REAL-UNPROVEN (env-gated) | S3 | F-001; pins the no-op watermark behavior |
| test `decode_refusal` | 655-672 | REAL | — | |
| test `decode_sink` | 676-701 | REAL | — | errors returned, never panicked |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mpeg_audio_fixture… | YES (when run) | exact facts pins, loud fixture assert — but NOT RUN here (no cargo) |
| mpeg_audio_fixtures_decode… | YES (when run) | exact packet/decode counts vs ffprobe |
| h264/hevc/flac/id3/differential/start-time corpus lanes | NO-OP in this environment | corpus dir absent → early `return`; they prove nothing here and are not counted as ignored |
| Oversized-ID3 note | — | the 64 KiB probe window vs a 99,682-byte ID3 tag is a real, documented open gap: a playable MP3 refuses to open (probe cannot see a frame) — content probing's own blind spot, honestly recorded |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mp3/mp2 lanes decode end-to-end" | module docs 9-14 | REAL | fixtures + provider arms (cross-ref pipeline_host); UNVERIFIED by execution here |
| "h264/hevc … open and demux, then refuse decode honestly" | module docs 15-24 | REAL-UNPROVEN | code path read; the test that would prove it is corpus-gated and the corpus is absent |
| "`flac` → the honest typed refusal" | module docs 25-34 | REAL | open_flac 258-271 |
| "the watermark is exact" (mpeg audio) | module docs 61-63 | REAL-UNPROVEN | corpus-gated test |

## 7. Dead / duplicated / unreachable code

- No dead production code. `harvest_annexb`'s codec mirror (descriptor crate-private) is duplication by necessity, commented.
- Test evidence duplication: `CountingSink`/`decode_sink`/`decode_refusal` are near-copies of helpers in ps.rs/adts.rs tests (same convention, third copy in this directory — noted, test-only).

## 8. Dependencies & flags

No feature gates. Depends on `demux::es::{mp3, annexb}`, the shared `SelectDemuxer`, and `pipeline_host::HostDecoderProvider` (test-only).

## 9. Open questions for the next agent

- Where does the corpus live for CI (if anywhere)? If CI never has `/tmp/my-project/corpus-v4/media`, the h264/hevc lanes' ffprobe-pinned claims are unverifiable claims — check the workflows (Wave 17).
- The oversized-ID3 gap (99,682-byte tag > 64 KiB window): is there a matrix row claiming "MP3 with cover art opens"? That row is FALSE for tag > 64 KiB.
- pipeline_host's provider arms: confirm the descriptor-gates-before-pcm discipline (F-002's safety) is asserted somewhere, not just commented.
