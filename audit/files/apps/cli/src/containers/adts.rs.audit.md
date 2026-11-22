# AUDIT: apps/cli/src/containers/adts.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 649 / 14 (2 production + 12 test fns/tests/helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `main.rs` → `run::run` → `lumen_dispatch` → `CliHost::open_session` → `open_source` → `containers::open` (mod.rs:52: `"adts" => adts::open(stream, opts)`). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |

Commands run:
- `sed -n '73,89p;124,132p' apps/cli/src/containers/adts.rs` (quote verification, exit 0)
- `ls crates/codecs/tests/fixtures | rg "sine48_lc_mono|sine441_lc_stereo"` → both fixtures exist
- `rg -n '"adts" =>' apps/cli/src/containers/mod.rs` → line 52
- `[ -d /tmp/my-project/corpus-v4/media ]` → ABSENT (the two corpus-gated tests skip in this environment)

## 1. One-paragraph verdict

A thin, honest host adapter over the real `demux::adts::AdtsDemuxer`, in the same shape as its siblings: construct, pre-pass, harvest, `--start-time` through the demuxer's pure seek query plus the skip watermark, wrap in `SelectDemuxer`. The no-blob/no-pcm/no-extent harvest is coherent, and the module docs give the strongest justification in this directory for why `extra_data` must stay `None` (a synthetic ASC would flip the decoder into raw-block mode and misframe every packet). Five of seven tests are unconditional over committed/synthesized bytes; two corpus-gated tests — including the file's ONLY damage-path test (dirty tail garbage) — silently skip in this workspace because the corpus dir is absent. The notable defect is a cross-family inconsistency: this host refuses to carry geometry for AAC while its sibling es.rs stuffs the same class of geometry into the same field for MP3, so raw-MP3 sessions get an audio limb and an audio-master clock and raw-AAC sessions do not — opposite choices, each documented as principled.

## 2. What actually works (verified)

- Wiring verified (mod.rs:52). Both committed fixtures exist (`sine48_lc_mono.aac`, `sine441_lc_stereo.aac`).
- The unconditional test set is real: facts pins, ffprobe packet counts (142/131), decode to `packets × 1024` sample frames, a synthesized reserved-sampling-index refusal (stage `demux`, code `demux.adts.bad-sampling-index`), and a synthesized Main-profile open-then-typed-decode-refusal (`cli.codec_unavailable` — LC-only decoder, matching bench.rs:334's "first vertical" posture).
- Not executed (no cargo): assertions read and found specific; W00 baseline corroborates the suite runs.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/adts.rs:28-30 (vs es.rs:147-159, demux_host.rs:202-220)
Code:
```rust
//! * **No PCM pair**: the track is not PCM — `channels`/`rate` live
//!   in the ADTS header the decoder parses itself; the facts carry
//!   only the sample-clock base (one tick = one sample).
```
What is wrong: this host refuses to publish ADTS header geometry (channels/rate are in `stream_info()` — "format/rate/channels/profile — the fields ffprobe reports", adts.rs:117-118) because "the track is not PCM". Its sibling es.rs does the exact opposite for the same class of codec-carried geometry: it stuffs MP3/MP2 header geometry into `pcm` as a "geometry-only carrier" so those sessions get an audio limb (device + audio-master clock) via `audio_geometry()`. ADTS sessions therefore open limb-less on the wall clock while raw-MP3 sessions open with a real audio clock — for streams that are structurally the same kind of thing (headerless-wrapped audio with in-band config).
Why it matters: the two container hosts each document their choice as the principled one; both cannot be. The observable consequence is per-family clock-model divergence and the raw-AAC "limb-less family gap" that host.rs:1741-1757 tests as expected behavior. Any wave-20 remediation for the frozen-clock issue must reconcile this asymmetry.
What correct looks like: one rule for "geometry-only carrier" across all raw-audio families (either both carry it, or neither does and `audio_geometry` grows an ADTS arm).
Confidence: HIGH — Verification: read all three files.
Severity: S3 (consistent, documented, tested behavior — but contradictory design).

### F-002 [S3]
Where: apps/cli/src/containers/adts.rs:466-484, 540-555
Code:
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
```
(corpus_dir, line 471; the same absolute machine path as es.rs). The two corpus-gated tests — `corpus_adts_lanes_decode_through_the_open_path` (the ffmpeg differential: 346/376 packets, 354,304/385,024 frames) and `corpus_dirty_tail_lane_is_honest_through_the_open_path` (the ONLY damage-path test: garbage tail, loud recovery, typed sticky truncation `demux.adts.truncated`) — silently `return` when the corpus is absent. Verified: it is absent in this workspace.
Why it matters: the file's headline honesty claims about damaged input ("never a panic, never a wrong-rate decode") have zero coverage here and on any machine without that corpus; they are not counted among the 25 ignored tests (W00). Smaller ratio than es.rs (2/7 vs 8/13) but the dirty lane is the only damage evidence in the file.
Confidence: HIGH — Verification: ls of the default path (absent) + read.

### F-003 [S4]
Where: apps/cli/src/containers/adts.rs:127-132
Code:
```rust
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            info.rate,
            "the sample clock is the header's declared rate"
        );
```
What is wrong: debug-only invariant pin (same pattern as ps.rs/es.rs) — compiled out in release.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/cli/src/containers/adts.rs:87 (and 347-351)
Code:
```rust
        demuxer.seek(target).map_err(|err| wire_error(&err))?;
        skip_until_ns = Some(start_us.saturating_mul(1000));
```
What is wrong: (a) the pure-seek landing is discarded (validation only — verified harmless: the watermark does the positioning, same as ps.rs F-008); (b) the past-the-end test path anchors `start_position_ns` at `i64::MAX/8 × 1000` ns (~292 years) while admitting no packets — an anchor no content corresponds to (asserted as "unclamped but honest" at 348-351). Same anchor-without-positioning class as es.rs F-003.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/cli/src/containers/adts.rs:519-524, 641-646
Code:
```rust
            table.push(format!(
                "{} {} {}Hz -> {} packets, {} frames",
```
What is wrong: both corpus tests print their differential via `eprintln!` — evidence-by-stdout in tests, unusable as assertion (the assertions do exist alongside; the print is bookkeeping). Trivia.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: apps/cli/src/containers/adts.rs:580-585
Code:
```rust
        assert_eq!(
            packets,
            TRUTHS[0].packets + 4,
            "346 real + 4 complete garbage frames (ffmpeg: 351 — the short-packet \
             divergence, recorded in the demux card's worklog)"
        );
```
What is wrong: the assertion pins OUR divergent count (350) while ffmpeg says 351 — the divergence is documented in the message, honestly. Bookkeeping note only: a known 1-packet divergence from the reference is asserted as correct behavior.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 61-107 | REAL | S4 | pure-seek validation + watermark; no extent to clamp (documented); negative start clamped to 0 |
| `harvest_facts` | 116-153 | REAL | S4 | no pcm/blob/extent — internally consistent, externally inconsistent with es.rs (F-001); debug-only pin (F-003) |
| test `codec_fixture` | 176-180 | REAL | — | |
| test `CountingSink::accept` | 188-198 | REAL | — | third copy of this helper in containers/ tests (ps.rs, es.rs, here) |
| test `open` | 202-207 | REAL | — | real open path |
| test `decode_frames` | 214-243 | REAL | — | typed refusals returned |
| test `fixture_opens_with_expected_facts` | 253-269 | REAL | — | unconditional; pins the limb-less facts (pcm None, blob None) |
| test `fixtures_decode_the_full_stream` | 278-316 | REAL | — | loud fixture assert; ffprobe counts + ×1024 decode |
| test `start_time_skips_packets` | 324-362 | REAL | S4 | mid-target watermark + past-the-end zero-packet; the ~292-year anchor (F-004b) |
| test `reserved_sampling_index_refuses_at_the_demux_stage` | 369-384 | REAL | — | synthesized bytes; stage+code pinned |
| test `main_profile_opens_but_refuses_the_decoder` | 392-420 | REAL | — | synthesized; open-then-refuse boundary pinned |
| const `TRUTHS` | 446-461 | REAL | — | ffmpeg truth table (corpus-gated use) |
| test `corpus_adts_lanes_decode_through_the_open_path` | 476-525 | REAL-UNPROVEN (env-gated) | S3 | F-002 |
| test `corpus_dirty_tail_lane_is_honest_through_the_open_path` | 539-647 | REAL-UNPROVEN (env-gated) | S3 | F-002; the only damage-path evidence in the file; sticky-truncation replay asserted |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_opens… | YES (when run) | exact facts incl. the None fields |
| fixtures_decode… | YES | exact packet + sample-frame counts |
| start_time_skips… | YES | watermark admission pinned both ends |
| reserved_sampling_index… | YES | synthesized bad header; stage+code |
| main_profile… | YES | open + typed refusal code |
| corpus lanes ×2 | NO-OP in this environment | corpus absent → early return; not counted as ignored |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the demuxer exposes no ASC … there is nothing to harvest" | module docs 25-27 | REAL | harvest reads; matches demuxer accessor set |
| "None is the correct wiring, not an omission" (extra_data) | module docs 23-25 | REAL | the misframe argument is coherent; cross-ref pipeline_host's aac arm (not in my files) |
| "No PCM pair: the track is not PCM" | module docs 28-30 | TRUE-but-INCONSISTENT | es.rs stuffs non-PCM geometry into the same field (F-001) |
| "a past-the-end target … admits no packet — the same honest shape as a clamped refusal" | module docs 34-37 | REAL | test 347-361 |

## 7. Dead / duplicated / unreachable code

- Test-helper duplication: `CountingSink` + decode-driver near-copies in ps.rs, es.rs, adts.rs (test-only; third copy).
- No dead production code.

## 8. Dependencies & flags

No feature gates. Depends on `demux::adts` (incl. `SAMPLES_PER_BLOCK`, `test_support`), shared `SelectDemuxer`, `pipeline_host::HostDecoderProvider` (tests).

## 9. Open questions for the next agent

- The ADTS-vs-MP3 geometry asymmetry (F-001): which of the two documented principles does the wave-20 remediation adopt?
- pipeline_host's `aac`/`aac-lc` arm: confirm the "blob only when one exists" seeding claim (module docs 21-24) — the misframe hazard argument is testable and should be pinned there.
