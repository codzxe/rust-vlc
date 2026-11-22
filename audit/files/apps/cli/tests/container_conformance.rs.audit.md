# AUDIT: apps/cli/tests/container_conformance.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 763 / 7 helpers + 17 test lanes |
| File verdict | SHALLOW as executed (no recorded run of ANY lane); the committed-fixture lanes are REAL-UNPROVEN, the corpus lanes are skip-as-pass |
| Reachable from shipped entry point? | N/A (test file) — but it tests the shipped binary via `CARGO_BIN_EXE_lumen`, which is the strongest possible harness shape |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | `sed -n` quote verification (all quotes verbatim); `ls` fixture existence (all present); baseline-log binary inventory (`container_conformance` ABSENT from audit/runs/baseline/cargo-test.log's 116 binaries). NO cargo (adaptation). |

## 1. One-paragraph verdict

Structurally the best test file in my assignment: every lane spawns the REAL `lumen` binary, drives it with a piped stdio script, and asserts concrete output (exit codes, event lines, exact positions, negative assertions like `!contains("cli.codec_unavailable")`). The mp4_h264 lane is the repo's only player-path assertion that h264+aac MP4 plays to 100% — directly contradicting the stale "typed refusal" story in mp4.rs:29-32/mkv.rs:54-58 and consistent with pipeline_host.rs:346 and ts.rs's updated docs. But the file's evidentiary value is zero against the current tree: 8 of 15 lanes self-skip as PASS without the corpus (absent here), and the W00 baseline `cargo test --workspace` never ran this binary at all (fail-fast stop at `first-vertical`) — so not one of its assertions, including the project-critical h264 play-out pin, has a recorded green run. The suite also self-admits nondeterminism (jitter prefix-pins) and one lane (webm) depends on wall-clock elapsing during `advance`, contradicting the runner's "zero wall-clock" determinism claim.

## 2. What actually works (verified)

- Harness shape: `Command::new(env!("CARGO_BIN_EXE_lumen"))` with piped stdin/stdout/stderr (lines 43-67) — the real binary, not a library call, not a mock. BrokenPipe on a fast-failing open is tolerated (44-59) — correct shape.
- All referenced fixtures exist (verified `ls`): `tests/fixtures/generated/valid/{avi_mjpeg_pcm.avi, wav_pcm_s16le_44k1_st.wav, aiff_pcm_s16be_44k1_st.aiff, mp4_mjpeg_pcm.mp4, mp4_h264_aac.mp4, webm_vp9_opus.webm}`, `tests/fixtures/generated/ivf/{av1_64x48_30fps.ivf, vp9_64x48_25fps.ivf}`, `crates/codecs/tests/fixtures/{mp2_48k.mp2, flac_s16_stereo_44100.flac}`.
- Assertions are behavioral and would catch regressions: exit 65 vs 0 distinctions, `[control.unsupported]` + `session.no-decodable-track` message shapes, exact position strings (`ok: time 2.000000s (100.0%)`), and negative pins (`!out.contains("audio.ring_full")`, `!out.contains("cli.i420_color_unsupported")`).
- `mpegts_family_lane` (574-599) cross-checks ts.rs's contract exactly: hevc refusal exit 65, `--no-video` accepting with `cli.audio_limb_absent`, `--start-time` → `demux.ts.seek.unsupported`. Consistent with ts.rs tests and crates/demux/src/ts (mod.rs:1162-1167).

## 3. Findings

### F-001 [S2] — Not one lane of the "cross-container conformance matrix" has a recorded run

Where: apps/cli/tests/container_conformance.rs:1-6 (module doc claims "the W3 cross-container CLI conformance matrix"), whole file; evidence: audit/runs/baseline/cargo-test.log

Code (module doc, lines 1-4, verbatim):
```rust
//! 6.OPW-cli-conformance — the W3 cross-container CLI conformance
//! matrix, through the REAL binary (session scripts over piped stdio,
//! the `session_script.rs`/`unsupported_path.rs` pattern): one
//! systematic lane per wired container family — avi, wav, aiff,
```

What is wrong: two independent gaps. (a) The W00 baseline `cargo test --workspace` log contains 116 test binaries and `container_conformance` is not among them — cargo stopped fail-fast at `first-vertical` (cargo-test.log:2499, `error: test failed, to rerun pass \`-p first-vertical --test evidence\``, EXIT=101). (b) In a corpus-less environment (this sandbox), the 8 corpus lanes return before asserting. Net: the suite's asserted behaviors — including the single most important claim in the repo (mp4_h264_family_lane's full h264 play-out) — are UNVERIFIED against the current tree by any recorded run.
Why it matters: this is the file a matrix row would cite as evidence for "CLI conformance"; §3.4 — evidence-shaped tests that have not run are not evidence.
What correct looks like: baseline re-run with `--no-fail-fast`; corpus lanes `#[ignore]`d with a CI lane; committed mini-fixtures for the corpus-dependent families (the file itself proves self-authored fixtures are feasible — see ts.rs's synthetic lane).
Confidence: HIGH — Verification: read + baseline-log inventory + `ls`.

### F-002 [S2] — 8 of 15 lanes skip-as-pass when the corpus is absent

Where: apps/cli/tests/container_conformance.rs:89-96 (gate), and the lanes at 224-227 (adts), 262-264 (ogg vorbis), 292-294 (ogg opus), 353-356 (mov), 388-391 (m4a), 467-470 (mkv), 576-579 (mpegts), 701-704 (es mp3)

Code (lines 89-96 + 224-227, verbatim):
```rust
fn corpus_file(name: &str) -> Option<String> {
    let dir = match std::env::var("LUMEN_CORPUS") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("/tmp/my-project/corpus-v4/media"),
    };
    let path = dir.join(name);
    path.is_file().then(|| path.to_string_lossy().into_owned())
}
```
```rust
    let Some(file) = corpus_file("0059-audio-only.aac") else {
        eprintln!("SKIP adts lane: corpus dir absent");
        return;
    };
```

What is wrong: the `return`-in-`else` pattern counts these lanes as PASSED (not ignored) in any environment without `/tmp/my-project/corpus-v4/media` (absent here, verified). Same defect class as mkv.rs F-002 / mp4.rs F-002 / ts.rs F-001 — the repo-wide convention is skip-as-pass.
Confidence: HIGH — Verification: read + `ls`.

### F-003 [S3] — The webm lane's play-out depends on wall-clock elapsing, contradicting the determinism contract

Where: apps/cli/tests/container_conformance.rs:519-528; contract: apps/cli/src/lib.rs:60-62

Code (lines 521-527, verbatim):
```rust
    // The audio-master null sink advances on real time — the pumps
    // are repeated until the 2.008 s media plays out (each `advance`
    // drives the render loop while wall time passes).
    let mut script = String::new();
    for _ in 0..40 {
        script.push_str("advance 512\n");
    }
```

Code (lib.rs:60-63, verbatim):
```rust
//!   pump at command boundaries plus the explicit `advance` verb —
//!   fully deterministic, zero wall-clock sleeps. Continuous wall-time
//!   pumping joins with the session-thread card (3.B3).
//! - The app's own logic is std-only; its two external dependencies
```

What is wrong: the lane's own comment says the null sink "advances on real time" and the pumps work "while wall time passes" — i.e., end-of-media on this lane is a race between 20 480 pumped ticks and wall-clock-drained audio. Either `advance` is deterministic (lib.rs) or the media ends when wall time passes (this comment); both cannot be true. The pinned `ok: time 2.013500s` (line 535) is a single-sample measurement of a timing race — a loaded CI box can land the `time` query before the sink drains (lane fails) or the harness could spin far past. The module doc's own admission ("aac/av1/vp9 post-seek decode recovery is not byte-deterministic", lines 23-25) concedes the suite is not fully deterministic.
Why it matters: a timing-dependent test will flake or, worse, pass by accident; and it documents a real design smell — the null audio sink advancing on wall time is the same hazard the W00 smoke exposed (clock stuck at 0.000000s without pumping, mp4-smoke2-stderr.txt).
What correct looks like: the pump count alone must reach EOS deterministically (tick-driven sink), or the lane must poll until EOS with a generous bound.
Confidence: MEDIUM — Verification: read (both files); race behavior UNVERIFIED (no run).
Severity: S3 (test flakiness + contract contradiction), not S2, because the production behavior it exposes is documented elsewhere.

### F-004 [S4] — Prefix-pin assertions are weaker than their comments claim

Where: apps/cli/tests/container_conformance.rs:452-455, 505-508, 556-559

Code (lines 452-455, verbatim):
```rust
    assert!(
        out.contains("ok: time 0.50"),
        "the anchor lands past 500 ms (jitter tail):\n{out}"
    );
```

What is wrong: `contains("ok: time 0.50")` passes for any position in [0.500000s, 0.509999s] — a landing 10 ms off the anchor passes silently. The comment says "lands past 500 ms" but the assert also matches a landing that MISSES 500 ms by up to +10 ms, and — worse — matches nothing about the actual anchor if the position were, say, 0.4999s (fails) vs 0.504s (passes): the tolerance is asymmetric and unbounded above within the prefix. Honest per the jitter admission, but it is the weakest assertion shape in the file.
Confidence: HIGH — Verification: read.

### F-005 [S4] — Module doc overstatement: "Every row asserts the same five behaviors"

Where: apps/cli/tests/container_conformance.rs:8-10

Code (verbatim):
```rust
//! Every row asserts the same five behaviors, each pinned to the
//! FAMILY's measured honest answer (this session's measurements, not
//! a uniform wish): open (accept, or the typed exit-65 refusal shape
```

What is wrong: rows differ materially — `mpegts_family_lane` never asserts an `advance` play script (it quits immediately, 580); `ivf_row` asserts "state: Playing (position " instead of end-of-media (618-622); the flac row asserts only the refusal. "This session's measurements" is also unverifiable — no session output is committed anywhere the file points to.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 43-67 | REAL-UNPROVEN | — | Spawns real binary; tolerates BrokenPipe; `code().unwrap_or(-1)` — a signal-killed child (timeout shape, cf. W00 smoke exit=124) surfaces as -1 with no hint. |
| `fixture` / `codec_fixture` | 70-84 | trivial-correct | — | Paths verified to exist. |
| `corpus_file` | 89-96 | SHALLOW | S2 (F-002) | Gate; no corpus → lanes answer nothing. |
| `assert_accepted` | 100-106 | trivial-correct | — | |
| `assert_no_decodable_track` | 112-124 | REAL-UNPROVEN | — | Pins exit 65 + `[control.unsupported]` + message + negative `!accepted`. |
| `avi_family_lane` | 135-155 | REAL-UNPROVEN | — | Full-green row: eom at exactly 2.000000s (100.0%), both selection flags, seek lands 1.000000s (50.0%). Strong. |
| `wav_family_lane` | 165-184 | REAL-UNPROVEN | — | Audio-only row; `--no-audio` → typed empty-session refusal. |
| `aiff_family_lane` | 188-207 | REAL-UNPROVEN | — | WAV twin. |
| `adts_aac_family_lane` | 222-250 | SHALLOW | S2 (F-002) | Corpus-gated. Pins the `cli.audio_limb_absent` honest event. |
| `ogg_vorbis_family_lane` | 260-285 | SHALLOW | S2 (F-002) | Corpus-gated; exact 4.032000s pin. |
| `ogg_opus_family_lane` | 290-309 | SHALLOW | S2 (F-002) | Corpus-gated; the 99.8% container-vs-decoded honesty pin. |
| `mp4_family_lane` | 320-339 | REAL-UNPROVEN | — | Committed mjpeg+pcm fixture; matches W00 bench evidence (75/130 decode, mjpeg-smoke-bench.txt). |
| `mov_family_lane` | 351-380 | SHALLOW | S2 (F-002) | Corpus-gated; pins the 6.FIX-pcm regression (sub-44.1kHz 6ch stall). |
| `m4a_family_lane` | 386-408 | SHALLOW | S2 (F-002) | Corpus-gated. |
| `mp4_h264_family_lane` | 425-456 | REAL-UNPROVEN | — | THE critical lane: asserts h264+aac MP4 plays to `1.532517s (100.0%)`, `Playing → Ended`, and `!cli.codec_unavailable`. Consistent with pipeline_host.rs:346 + ts.rs:38-45; contradicts mp4.rs:29-32/mkv.rs:54-58 docs. Never executed in any recorded run (F-001). If it passes, the owner-facing MP4 failure is NOT decode — it is the one-shot/REPL UX (W00 smoke 1: `exit=124`). |
| `mkv_family_lane` | 465-509 | SHALLOW | S2 (F-002) | Corpus-gated; its comment admits "Exact position pins stay [VERIFY] until a corpus-enabled run re-measures them" (463-464). |
| `webm_family_lane` | 518-560 | REAL-UNPROVEN | S3 (F-003) | Committed fixture, but wall-clock-dependent play-out (finding above). |
| `mpegts_family_lane` | 574-599 | SHALLOW | S2 (F-002) | Corpus-gated; behavior pins consistent with ts.rs. |
| `audio_less_start_time_is_deterministic_across_runs` | 655-674 | REAL-UNPROVEN | — | Genuine determinism regression test (runs the binary twice, asserts byte-identical output). Strong shape. |
| `ivf_row` + `ivf_av1_family_lane` + `ivf_vp9_family_lane` | 614-645, 677-688 | REAL-UNPROVEN | — | Committed fixtures; asserts converter closure (`!cli.i420_color_unsupported`). |
| `es_mp3_family_lane` | 699-721 | SHALLOW | S2 (F-002) | Corpus-gated; the honest `(0.0%)` no-extent position. |
| `es_mp2_family_lane` | 725-744 | REAL-UNPROVEN | — | Committed codec fixture. |
| `es_flac_family_lane_is_the_honest_refusal` | 751-763 | REAL-UNPROVEN | — | Pins that `lumen open x.flac` REFUSES (exit 65, "raw-FLAC elementary-stream demuxer has not landed") — an honest, documented capability gap: raw FLAC does not open; FLAC inside mkv/wav does. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| avi/wav/aiff/mp4/mp4-h264/webm/ivf/es-mp2/flac/determinism lanes | YES | Real binary, concrete output pins incl. negative assertions. (All runtime-UNPROVEN — the binary never ran in the W00 baseline.) |
| the 8 corpus lanes | NO (corpus-less env) | Return before asserting (F-002). |
| webm lane | UNCERTAIN | Timing-race-dependent (F-003) — may fail on loaded machines or pass vacuously. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one systematic lane per wired container family" | :2-6 | REAL-UNPROVEN | All families present as lanes (avi…es), but 8/15 never execute without corpus. |
| "Every row asserts the same five behaviors" | :8-10 | FALSE (overstated) | F-005: mpegts has no play script; ivf asserts a different position shape; flac asserts only the refusal. |
| "aac/av1/vp9 post-seek decode recovery is not byte-deterministic" | :23-25 | REAL-UNPROVEN | Self-admission; consistent with the prefix pins used. |
| mp4_h264 lane's implied claim "h264+aac MP4 plays to 100% through the CLI" | :418-437 | UNVERIFIED | No recorded run (F-001); consistent with pipeline_host.rs:346 and ts.rs:38-45; contradicts mp4.rs:29-32/mkv.rs:54-58 module docs and the bench refusal string (bench.rs audit). |

## 7. Dead / duplicated / unreachable code

- `assert_no_decodable_track`'s `container` parameter is used only inside a format string; fine.
- No dead code. Duplication: the corpus-gate triple (`corpus_file` here, `corpus_dir` in mkv/mp4/ts/ogg) is 5 near-identical copies across the crate — one shared `testkit` helper would delete them all.

## 8. Dependencies & flags

- Dev-dependency shape: uses `testkit::Must`, `env!("CARGO_BIN_EXE_lumen")` (requires the bin target — present). No feature gates. This is an integration test (`tests/`), so it only runs under `cargo test -p lumen-cli` — which the W00 baseline never reached.

## 9. Open questions for the next agent

- Does `mp4_h264_family_lane` actually pass? It is the decisive evidence for "h264 MP4 plays through the CLI". Needs a corpus-less run of just this binary — blocked (no cargo). Wave-20 A05 must run it.
- The webm lane's wall-time dependence (F-003): is the null sink's counter driven by ticks or wall time? Answer lives in audio-output/pipeline_host (outside my files; pipeline_host.rs's `RENDER_QUANTUM` docs say the pump drives renders — the truth needs a run).
