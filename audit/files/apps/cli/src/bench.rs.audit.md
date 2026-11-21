# AUDIT: apps/cli/src/bench.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 586 / 16 |
| File verdict | REAL (first-vertical lane real and evidenced; zero-limb success path and capability message are S2 defects) |
| Reachable from shipped entry point? | YES (call chain: `lumen decode-bench <file>` → apps/cli/src/main.rs → `crate::run::run` → run.rs:144 `Some(CliCommand::DecodeBench { source }) => bench::run_bench(&source, out, err)`) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | rg "decode-bench" docs/ tools/ (claims); Read tools/corpus/speed_bench.py:55-165 (rc-only success check :88, ffprobe-sourced fps :141-148); sed bench.rs quotes 8-15, 35-43, 94-101, 264-266, 299-301, 328-341; rg bench tests in baseline: orchrun-test-workspace-nofailfast.log.txt:2772-2780 all green; Read audit/runs/baseline/mp4-smoke-decode-bench.txt (verbatim zero-work BENCH + EXIT=0); rg rc-smoke.sh BENCH parser (:98-116); rg ThreadBudget default (codec-api/src/config.rs:63-67 = 1 thread, matches doc); rg run.rs run_bench (:144) |

## 1. One-paragraph verdict
The decode loop, concealment budget, routing, and error wiring are real and the AVI MJPEG+PCM lane is fully evidenced (baseline tests green; rc-smoke 60/60). But this file is the source of a standing baseline defect: it hard-codes exactly two decoders (`MjpegDecoder`, `PcmDecoder`) instead of the `HostDecoderProvider` registry the player uses, so for every other codec it decodes NOTHING, prints a success-shaped `BENCH` line of zeros, and exits 0 — while telling the user "no decoder in this build", which is false (the build's provider decodes h264-sw/av1/vp9; the ivf.rs tests prove it in the same crate). Downstream, `speed_bench.py` treats rc==0 as a successful measurement and credits lumen with ffprobe's frame count over a near-zero wall — the tool poisons the very speed-parity benchmark it was built to serve, contradicting its own recorded design ("non-MJPEG/PCM codecs DNF by typed refusal… no silent skips", worklog archive-2026-10.md:7813).

## 2. What actually works (verified)
- AVI MJPEG+PCM lane end-to-end: baseline `test bench::tests::{bench_line_is_the_exact_contract, garbage_file_is_a_typed_refusal, happy_path_fixture_reports_bench_line} ... ok` (orchrun-test-workspace-nofailfast.log.txt:2772-2780) and `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` (INDEX.md §samples).
- Garbage input → typed `probe.unsupported`, exit 65, no BENCH line — verified by the baseline test at :2773 and INDEX.md adversarial claim (0 panics on 88 corpus files, worklog:7826).
- The BENCH line shape is pinned by an exact-string test (`bench_line_is_the_exact_contract`), and `scripts/rc-smoke.sh:98-116` parses it (with decoded==frames validation).
- Routing (bench.rs:365-372) and the shared concealment budget (one `consecutive` counter across limbs, documented as mirroring the engine's one-budget design) are internally consistent; `ThreadBudget::default()` = 1 thread matches the "one thread" doc (codec-api/src/config.rs:63-67).

## 3. Findings
### F-001 [S2]
Where: apps/cli/src/bench.rs:299-343 (selection), 213-237 (exit-0 success shape)
Code:
```rust
        match track.kind {
            MediaKind::Video if video.is_none() && fact.codec == "mjpeg" => {
                let mut decoder = codecs::video::mjpeg::MjpegDecoder::new();
```
and the fall-through at 328-341:
```rust
                let reason = if matches!(track.kind, MediaKind::Video) && video.is_some()
                    || matches!(track.kind, MediaKind::Audio) && audio.is_some()
                {
                    "first-of-kind already benched"
                } else {
                    "no decoder in this build (first vertical: mjpeg + pcm)"
                };
```
What is wrong: Only `mjpeg` video and `pcm_*` audio get a limb — every other codec (h264, aac, av1, vp9, …) selects no decoder, yet `run_bench` prints the success `BENCH` line and returns 0. Recorded proof: `audit/runs/baseline/mp4-smoke-decode-bench.txt` — `bench: video track#0 h264 — not benched … audio track#1 aac — not benched …` then `BENCH file=…sample.mp4 … v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.006 decode_s=0.000` and `EXIT=0`. Two compounding defects: (a) `v_frames`/`a_frames` count LIMB feeds, not demuxed packets, so `v_frames=0` is indistinguishable from "demuxer enumerated no track" — for sample.mp4 two tracks WERE demuxed and all packets walked and dropped (bench.rs:370-374); (b) the consumer `speed_bench.py` marks a run `ok` purely on `rc == 0` (`ok = [r for r in runs if r["rc"] == 0]`, tools/corpus/speed_bench.py:88) and computes `fps` from FFMPEG's ffprobe frame count (:141-148) — so a zero-decode 6 ms run is recorded as a successful, absurdly fast decode, fabricating parity wins for every non-MJPEG/PCM class.
Why it matters: This is the project's speed-parity evidence tool; its zero-work success path fabricates measurements. The file's own recorded design intent was the opposite — "non-MJPEG/PCM codecs DNF by typed refusal (D-018 first-vertical scope — the ratio table says so, no silent skips)" (docs/worklog/archive-2026-10.md:7813); when the container registry grew to 15 families the refusal disappeared and the DNF silently became a fake win.
What correct looks like: When zero limbs were selected (or `v_decoded + a_decoded == 0` with tracks present), exit nonzero with a typed `decode.bench.no-decoder` error (the documented DNF shape), or emit an explicit `dnf=` field the harness can gate on; count demuxed packets per kind independently of limb feeds.
Confidence: HIGH — Verification: read + baseline run mp4-smoke-decode-bench.txt + speed_bench.py:88,141-148.

### F-002 [S2]
Where: apps/cli/src/bench.rs:334
Code:
```rust
                    "no decoder in this build (first vertical: mjpeg + pcm)"
```
What is wrong: The string is a false claim about the shipped build. The build's `HostDecoderProvider` constructs and runs h264-sw, av1 (rav1d), vp9 (rusty_vp9), mjpeg, pcm decoders — proven inside this very crate by `containers::ivf::tests::{av1_lanes_decode_the_full_stream, vp9_lane_decodes_the_full_stream}` (baseline orchrun log:2796-2812) and by INDEX.md CORRECTION v2 (h264 sw decoder exists and plays 4:2:0 MP4 through `lumen open`). The bench bypasses the provider and hard-codes two decoder constructors, then tells the user the decoders do not exist in the build.
Why it matters: User-visible false capability information from the shipped binary — it already contributed to Wave-0's wrong "MP4 cannot be opened / no H.264 decoder" headline (INDEX.md CORRECTION v2 lists this exact message as a standing defect). Any user diagnosing with `decode-bench` is actively misled.
What correct looks like: Either use `HostDecoderProvider` (the same registry `lumen open` uses), or state the truth: "decode-bench only wires mjpeg+pcm lanes (tool scope), the player's provider has more".
Confidence: HIGH — Verification: read + baseline INDEX.md CORRECTION v2 + ivf tests green in baseline.

### F-003 [S3]
Where: apps/cli/src/bench.rs:98-108
Code:
```rust
struct NullSink {
    /// Frames accepted (cross-checked against the outcomes' sums).
    frames: u64,
}
```
What is wrong: The documented cross-check does not exist. `sink.frames` is incremented in `accept` (:105) and never read again — `bench()` builds `BenchReport` exclusively from the limbs' `DecodeOutcome` sums (:407-412); no assertion, no comparison, nothing. Dead counter plus a doc claim of a safety mechanism that was never wired.
Why it matters: A reader (or auditor) trusts the comment and assumes outcome-vs-sink divergence would be caught; it would not be. If a decoder double- or under-reported `DecodeOutcome::Emitted { frames }`, the BENCH line would silently carry the wrong number.
What correct looks like: One comparison after the loop (`assert_eq!`-style or a typed internal error when `sink.frames != v_decoded + a_decoded`), or delete the field and the claim.
Confidence: HIGH — Verification: read; rg confirms no other use of `sink.frames`/`NullSink.frames` in the file.

### F-004 [S3]
Where: apps/cli/src/bench.rs:178-190
Code:
```rust
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        ErrorKind::Corrupt { .. }
            | ErrorKind::Decoder {
                cause: DecodeCause::Bitstream
            }
    )
}
```
What is wrong: This is a hand-maintained mirror of `playback-engine`'s crate-private `per_frame_damage` rule (the comment admits it). Nothing pins the two together: no test feeds a representative error set through both implementations, so when the engine's rule changes (e.g. a new `DecodeCause` variant becomes concealable), the bench silently diverges and benches a different error policy than the player exhibits.
Why it matters: The bench's numbers are only meaningful as a proxy for player behavior; a drifted concealment rule changes which damaged streams keep decoding (and thus wall time) in ways the comparison can't surface.
What correct looks like: Export the rule from `playback-engine` (or `codec-api`) and call it from both sites; or add a parity test asserting both classify a fixed error corpus identically.
Confidence: HIGH — Verification: read; no test in this file references `per_frame_damage`; the engine twin is crate-private per the comment.
### F-005 [S4]
Where: apps/cli/src/bench.rs:264-266 (comment), 8-15 and 35-43 (module docs)
Code:
```rust
    // 1. Open — the exact path `lumen open` takes (canonicalize →
    //    access → probe → demuxer start → strf facts). Garbage files
    //    refuse here (probe), non-AVI containers refuse here (D-018).
```
What is wrong: Stale D-018-era prose. Non-AVI containers do NOT refuse at open today — `open_source` dispatches through the 15-family container registry (containers/mod.rs:50-75); the MP4 sample opens, enumerates both tracks, and walks every packet. The module doc's "Per-stream coverage (honest)" section likewise only describes `v_frames=0` as "the demuxer enumerates no such track", never the actually-observed no-limb case (F-001), and the header still says the open path runs "`AviDemuxer::start` + the strf facts pre-pass" as if AVI were the only container.
Why it matters: Maintainers reading these comments believe non-AVI is refused at open; the false comfort hides exactly the F-001 fake-success path.
What correct looks like: Update comments to the registry era: open succeeds for all wired containers; bench coverage is mjpeg+pcm; zero-limb outcome = zeros + exit 0 (and fix that per F-001).
Confidence: HIGH — Verification: read + containers/mod.rs:50-75 + baseline mp4-smoke-decode-bench.txt.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mark_process_start` | 84-86 | REAL | — | `let _ = PROCESS_START.set(...)` — deliberate once-only; called from main. |
| `process_start` | 90-92 | REAL | — | Library fallback documented. |
| `NullSink` + `accept` | 98-108 | REAL | S3 (F-003) | Drops frames untouched; dead counter with false cross-check doc. |
| `Limb::feed` | 126-163 | REAL | — | Move-only packet, budget on consecutive damage, typed ResourceLimit refusal at 1000; non-damage Err wired. Correct. |
| `Limb::drain` | 166-175 | REAL | — | Flush outcome counted; errors wired. |
| `per_frame_damage` | 182-190 | REAL | S3 (F-004) | Unpinned mirror of the engine's rule. |
| `run_bench` | 213-238 | REAL | S2 (F-001 shape) | Prints BENCH line and returns 0 even when zero limbs decoded anything; concealment note to stderr only. |
| `bench_line` | 242-254 | REAL | — | Exact contract string; pinned by test. |
| `bench` | 258-425 | REAL | S2 (F-001, F-002), S4 (F-005) | Open path real; limb selection hard-codes mjpeg/pcm (:300, :312-313); `bytes` from `fs::metadata(source)` after open — re-stats argv path (same file; benign); demux fatal carries partial counts as details (good); `Recovered` counted; `Discontinuity` ignored silently (fine for a bench). |
| `configure` | 431-446 | REAL | — | DecoderConfig with FIRST generation, 1 thread, no extradata — PCM facts went in via `with_format`; consistent with demuxer stamping FIRST. |
| `facts_absent` | 450-456 | REAL | — | Defensive internal error for an unreachable state (guard already checked `fact.pcm.is_some()`); dead in practice, honestly labeled. |
| `kind_name` | 460-467 | REAL | — | Trivial mapping. |
| `tests::fixture` | 473-478 | REAL | — | Manifest-relative path. |
| `tests::bench_line_is_the_exact_contract` | 482-501 | REAL | — | Exact string pin. |
| `tests::happy_path_fixture_reports_bench_line` | 506-551 | REAL | — | Real `run()` dispatch, real fixture, bytes pinned to MANIFEST, decoded==frames both limbs, decode_s ⊆ wall. |
| `tests::garbage_file_is_a_typed_refusal` | 555-585 | REAL | — | exit 65, `[probe.unsupported]`, no BENCH line; temp file cleaned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bench_line_is_the_exact_contract | YES | Byte-exact string comparison. |
| happy_path_fixture_reports_bench_line | YES | Field-by-field parse of a real run; bytes pinned; 60/60 both limbs. |
| garbage_file_is_a_typed_refusal | YES | Exit code + stderr text + stdout shape all asserted. |
| NO test covers the zero-limb success path | NO | A mutation making the mjpeg gate never match (or selecting no limb for MP4) leaves all three tests green — the F-001 fake-success shape is completely untested; the AVI-only fixture set cannot see it. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "stdout carries exactly one machine-parseable line on success" | bench.rs:19-23 | REAL | bench_line test + baseline outputs. |
| "Every failure is a typed error: line … and a nonzero exit" | bench.rs:31-33 | FALSE-in-part | Zero-decode is not treated as failure: MP4 → BENCH zeros + EXIT=0 (mp4-smoke-decode-bench.txt). "Failure" is defined only as open/demux/decode errors, not as benched-nothing — the harness-consuming distinction is lost. |
| "a codec family without a decoder shows up as v_decoded < v_frames (or v_frames=0 when the demuxer enumerates no such track)" | bench.rs:35-43 | FALSE | For sample.mp4 the demuxer DID enumerate a video track and demuxed all packets; the line still shows v_frames=0 (limb-feed counting, F-001). |
| "non-MJPEG/PCM codecs DNF by typed refusal … no silent skips" | worklog archive-2026-10.md:7813 | FALSE (stale) | Current code: no refusal, silent skip with exit 0 (F-001/F-005). |
| "it decodes to null and prints one [line]" | docs/guides/troubleshooting.md:38 | REAL | For the first-vertical lane. |
| "decode-bench decodes every frame of the default fixture" | docs/release-checklist.md:73 | REAL | AVI fixture 60/60 (baseline mp4-smoke-bench-avi-mjpeg-pcm.txt). |

## 7. Dead / duplicated / unreachable code
- `NullSink.frames` dead counter (F-003).
- `facts_absent` defensive arm unreachable by construction (documented as such — acceptable).
- `per_frame_damage` duplication of playback-engine's rule (F-004).
- The two hard-coded decoder constructors bypass `HostDecoderProvider` — duplication of the provider's own selection logic (root of F-001/F-002).

## 8. Dependencies & flags
- `codecs::video::mjpeg`, `codecs::audio::pcm` direct construction — a layering choice that hard-codes coverage; everything else via `demux_host`, `commands`, `media_types`, `plugin_api`, `codec_api`. No feature gates; whole file ships in the release binary (baseline build #1 clean).

## 9. Open questions for the next agent
- PIPELINE HOST owner (A05): does `HostDecoderProvider` expose a constructor usable here without an engine session (the ivf tests use it with just `Arc<MediaFacts>` — if so, F-001's fix is a two-line change and the "first vertical" excuse is void)?
- Who owns `tools/corpus/speed_bench.py`'s `rc==0`-only success check — should the harness parse `v_decoded`/`a_decoded` and DNF on zeros (defense in depth against any runner that exits 0 without decoding)?
- The `engine-info` hard-coded literal (host.rs) prints the same "mjpeg, pcm_s16le" story — same pattern, A07/A09's chunk; cross-reference when auditing host.rs.
