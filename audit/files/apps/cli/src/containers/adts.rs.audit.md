# AUDIT: apps/cli/src/containers/adts.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 648 / 15 (2 production fns, 13 test items/helpers/consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` main.rs → run/dispatch → `host.rs:904 open_source` → `demux_host.rs:362 open_source` → probe → `containers/mod.rs:52 "adts" => adts::open(stream, opts)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "adts" apps/cli/src/containers/mod.rs` (dispatch arm at :52); `rg -n "adts::open" apps/cli/src/` (sole caller = registry); `rg -n "open_source" apps/cli/src/host.rs` (:904, the session open path); `sed -n '350,470p' apps/cli/src/demux_host.rs` (probe → registry flow); `sed -n '95,175p' apps/cli/src/containers/mp4.rs` + `sed -n '150,215p' apps/cli/src/containers/mkv.rs` (family start-time convention); `sed -n '360,470p'/'1120,1194p' crates/demux/src/adts/mod.rs` (AdtsDemuxer API: new/start/stream_info/seek/descriptor); `rg -n "lumen-cli|Running unittests" audit/runs/baseline/test-workspace.log.txt` (lumen_cli tests ABSENT from runs #3/#4 — both aborted at other packages' failures); `rg -n "containers::adts" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 7 tests ok :2774-2785); `ls /tmp/my-project/` (corpus-v4 absent now); `sed -n` on every quoted range (verbatim verified) |

## 1. One-paragraph verdict
This is the ADTS/ADIF container host wiring: two small production functions (`open`, `harvest_facts`) plus 494 lines of tests, and the wiring is genuine — the demuxer is constructed, `start()` pre-passed, facts harvested, `--start-time` validated through the demuxer's pure `seek`, all wrapped in the shared `SelectDemuxer` skip/selection wrapper. The code matches the family convention verified against mp4.rs/mkv.rs (`.max(0)` instead of `.clamp(0, duration_us)` because raw AAC declares no up-front duration — true, `MediaFacts::duration_us` is `None` for these tracks), the unit conversion µs→ns is correct, and all 7 tests executed and passed in the orchestrator's no-fail-fast run. The defects are evidentiary, not functional: an invariant the comment calls "pinned" is enforced only by a `debug_assert_eq!` that is compiled out of every recorded run (all baseline test runs were `--release`, no `debug-assertions` in any profile), and the two corpus differential tests report `ok` even when they skip, so the recorded pass does not prove the differential executed.

## 2. What actually works (verified)
- Full open lane through the real CLI path: `containers::adts::tests::{fixture_opens_with_expected_facts, fixtures_decode_the_full_stream, main_profile_opens_but_refuses_the_decoder, reserved_sampling_index_refuses_at_the_demux_stage, start_time_skips_packets, corpus_adts_lanes_decode_through_the_open_path, corpus_dirty_tail_lane_is_honest_through_the_open_path}` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2774-2785` (lumen_cli lib, 129 tests).
- End-to-end conformance: `container_conformance.rs::adts_aac_family_lane ... ok` (same log, tests/container_conformance section, 19/19 ok).
- Demuxer-side backing: `crates/demux/src/adts` tests (baseline test-workspace.log.txt:1535-1562) all ok — the container host's assumptions (typed `bad-sampling-index`, ADIF VBR/multi-program refusals, seek-past-end lands at last frame) are enforced where the container doc claims them.

## 3. Findings
### F-001 [S3]
Where: apps/cli/src/containers/adts.rs:124-132
Code:
```
    if let Some(info) = demuxer.stream_info()
        && let Some(track) = tracks.first()
    {
        debug_assert_eq!(
            track.timing.base.as_parts().1,
            info.rate,
            "the sample clock is the header's declared rate"
        );
    }
```
What is wrong: The invariant the surrounding comment calls "pinned as the harvest's invariant (divergence would mean the track and the accessor disagree about one header — a demuxer bug, never a wiring guess)" is checked exclusively by `debug_assert_eq!`, which is compiled out of release builds. No `[profile]` section in the root Cargo.toml enables `debug-assertions` for release (rg over Cargo.toml/rust-toolchain.toml/.cargo/config.toml: no hits), and every recorded test run used `--release` (baseline INDEX.md, test-run footnote).
Why it matters: The pin never executes in the shipped binary nor in any recorded evidence run. The comment claims an enforcement that does not exist anywhere the audit can observe; a future demuxer regression making the track base diverge from the header rate would pass this check silently in every environment that matters.
What correct looks like: Either a real `assert_eq!`/typed check (if divergence is a bug worth refusing on), or an honest comment stating the cross-check runs only under `debug_assertions` and is not part of shipped behavior.
Confidence: HIGH — Verification: read (profile settings verified; baseline runs verified `--release` via INDEX.md and log headers "Finished `release` profile").

### F-002 [S3]
Where: apps/cli/src/containers/adts.rs:475-484 (same shape at 538-555)
Code:
```
    #[test]
    fn corpus_adts_lanes_decode_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
                // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
                "corpus(adts-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS \
                 unset — SKIP (the differential table for this instance is in the worklog)"
            );
            return;
        };
```
What is wrong: Both corpus tests (`corpus_adts_lanes_decode_through_the_open_path`, `corpus_dirty_tail_lane_is_honest_through_the_open_path`) return `Ok` when the corpus directory is absent — the default `/tmp/my-project/corpus-v4/media` does not exist in this sandbox now (`ls /tmp/my-project/` → audit-workspace, scripts, target, tool-results, upload, wt). Under `cargo test`'s default capture, the `eprintln!` "loud skip" is swallowed for passing tests, so the result line reads identically for "differential executed and matched ffmpeg" and "skipped".
Why it matters: The recorded baseline evidence (`orchrun-test-workspace-nofailfast.log.txt:2774-2775` both `ok`) cannot distinguish executed from skipped; the comment's "loud skip" is only loud with `--nocapture`. A regression in the corpus differential lane would be invisible in every default CI run performed where the corpus is absent.
What correct looks like: Emit a machine-visible signal on skip — `#[ignore]` with a note, a `warn`-level marker parsed by CI, or assert the corpus presence under a feature flag so absence is a distinct outcome from pass.
Confidence: HIGH — Verification: read + `ls /tmp/my-project/` (corpus absent); whether the orchestrator's no-fail-fast run had the corpus present is UNVERIFIED (captured output not in the log).

### F-003 [S4]
Where: apps/cli/src/containers/adts.rs:231-242
Code:
```
            match opened.demuxer.next_event() {
                Ok(DemuxEvent::Packet(packet)) => {
                    let _ = decoder.decode(packet, &mut sink)?;
                }
```
What is wrong: `decoder.decode` returns `Result<DecodeOutcome, EngineError>` (codec-api trait doc, crates/codec-api/src/lib.rs:63) and the test helper discards the outcome (`let _ =`). Additionally, `start_time_skips_packets` (line 331-338) only asserts the ns bound for `Stamp::Present` pts — packets with missing stamps would count toward `seen` without any timing check.
Why it matters: Test-only; the sample-frame counting via the sink is the real assertion, but a decode outcome that reported e.g. "no output" would go unremarked, and the skip test would tolerate a broken skip that emitted un-stamped early packets.
What correct looks like: Match on the outcome (or assert it produced output where expected) and assert `seen == 0`-style invariants independent of stamp presence.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `open(stream, opts)` | 61-107 | REAL | — | Reachable via registry arm. `.map_err(wire_error)` at each stage; `start_us.unwrap_or(0).max(0)` clamps negative `--start-time` to 0 (family convention: es.rs:102, ivf.rs:89, ts.rs:117 use `.max(0)`; duration-declaring containers use `.clamp`); seek return (landing) discarded — identical to mp4.rs:126/mkv.rs:166 (family convention, `start_position_ns` = requested, not landing); `saturating_mul(1000)` µs→ns correct against `NANO_BASE`. |
| `harvest_facts(demuxer, tracks)` | 116-153 | REAL | S3 | Debug-only invariant (F-001); fields honest for an audio-only family (`pcm: None`, `extra_data: None`, `frame_ns: None`, `dimensions: None` — module-doc truth table matches demuxer: ADTS carries no out-of-band ASC); `tracks.to_vec()` clone is open-time, not per-frame. |
| `codec_fixture(name)` (test helper) | 176-180 | REAL | — | Path via `CARGO_MANIFEST_DIR`; fixtures verified present (`sine48_lc_mono.aac`, `sine441_lc_stereo.aac` in crates/codecs/tests/fixtures). |
| `CountingSink::accept` (test helper) | 188-198 | REAL | — | Counts sample frames via `format.frames_in`, records negotiated channels; no panic paths. |
| `open(path, start_us)` (test helper) | 202-207 | REAL | — | Real `open_source` path; `unreachable!("{err}")` panics on open failure — acceptable in a helper whose fixtures are committed. |
| `decode_frames(opened)` (test helper) | 214-243 | REAL | S4 | Discards `DecodeOutcome` (F-003); typed refusals returned, not panicked. |
| `fixture_opens_with_expected_facts` | 252-269 | REAL | — | Honest: asserts container/tracks/kind/codec/base/no-pcm/no-blob/no-extent/start pos. |
| `fixtures_decode_the_full_stream` | 277-316 | REAL | — | Two-pass differential: packet count (142/131) + decoded frames == packets×1024 via the real provider. Executed ok (no-failfast :2777). |
| `start_time_skips_packets` | 323-362 | REAL | S4 | In-media target asserts every present pts ≥ 1 ms and tail survives; past-the-end (i64::MAX/8) asserts 0 packets. Weak on missing-stamp packets (F-003). |
| `reserved_sampling_index_refuses_at_the_demux_stage` | 368-384 | REAL | — | Synthetic frame via `demux::adts::test_support::adts_frame(1, 15, …)`; asserts stage Demux + code `demux.adts.bad-sampling-index`; would fail (unreachable!) if the demuxer accepted index 15. Temp file cleaned up (`.ok()` discard is fine here). |
| `main_profile_opens_but_refuses_the_decoder` | 391-420 | REAL | — | Profile-honest `aac-main` descriptor + provider refusal `cli.codec_unavailable`; would fail if the provider matched aac-main. |
| `CorpusTruth` + `TRUTHS` | 434-461 | REAL | — | Data table: 346/376 packets, 88.2k/96k rates; consistent with demux corpus test names in baseline. |
| `corpus_dir()` | 466-473 | REAL | — | `LUMEN_CORPUS` env override else `/tmp/my-project/corpus-v4/media`; absent → None. |
| `corpus_adts_lanes_decode_through_the_open_path` | 475-525 | REAL | S3 | Skip-ambiguous pass (F-002); assertions exact when it runs. |
| `corpus_dirty_tail_lane_is_honest_through_the_open_path` | 538-647 | REAL | S3 | Same skip ambiguity; when run: 346+4 packets, sticky `demux.adts.truncated` replay asserted, decode refusal asserted. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_opens_with_expected_facts | YES | Exact facts assertions (codec string, base parts, None fields) against a committed fixture. |
| fixtures_decode_the_full_stream | YES | Exact packet and sample-frame counts; any demux/decode regression in the LC lane changes them. |
| start_time_skips_packets | YES (mostly) | Early-packet assertion on present pts + zero-packet past-the-end; a fully broken skip emits early present-pts packets and fails. |
| reserved_sampling_index_refuses_at_the_demux_stage | YES | Ok(...) hits `unreachable!`. |
| main_profile_opens_but_refuses_the_decoder | YES | Ok(decoder) hits `unreachable!`. |
| corpus_adts_lanes_decode_through_the_open_path | ONLY IF CORPUS PRESENT | Returns Ok on skip; corpus absent in this sandbox (F-002). |
| corpus_dirty_tail_lane_is_honest_through_the_open_path | ONLY IF CORPUS PRESENT | Same (F-002); also skips when the dirty lane file is missing. |
Execution note: the wave-0 baseline runs #3/#4 (test-workspace.log.txt, test-workspace-allfeatures.log.txt) contain NO `lumen_cli` unittest section — both aborted at other packages' failures (`-p desktop --test zero_alloc_transport`, `-p first-vertical --test evidence`). Execution evidence for this file's tests comes from `orchrun-test-workspace-nofailfast.log.txt:2769` (`Running unittests src/lib.rs (lumen_cli-3610105d84efd3c4)`, 129 tests, adts rows :2774-2785 all ok).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The codec descriptor is the header's profile-honest mapping … the demuxer's own `descriptor()`, never a local copy" | module docs :14-18 | REAL | harvest uses `track.codec.as_str()` (:139); track minted from `raw.descriptor()` (crates/demux/src/adts/mod.rs:1155); profile bits are 2-bit (mod.rs:200 `head[2] >> 6`), so the `_ => "aac-lc"` arm catches only LC. |
| "No `extra_data` … `None` is the correct wiring, not an omission" | module docs :19-27 | REAL | `extra_data: None` (:143); pipeline_host.rs:288-293 `"aac" \| "aac-lc"` arm — ADTS mode ignores extra_data; test asserts `extra_data.is_none()` and full decode works. |
| "the `pipeline_host` `aac`/`aac-lc` arm seeds the blob only when one exists" | module docs :22-23 | REAL (imprecise wording) | The arm actually "seeds unconditionally" (its own comment) — with whatever exists (None here); the functional claim (None ⇒ in-band ADTS mode) is true; wording drift S4-level, noted not counted. |
| "No PCM pair … facts carry only the sample-clock base" | module docs :28-30 | REAL | `pcm: None` (:142); test asserts `pcm.is_none()`. |
| "No declared duration … span end is `Missing` at open" | module docs :31-37 | REAL | `span_end_ticks` from `track.timing.span.end` (:135) is None pre-walk; test asserts `span_end_ticks.is_none()`. |
| "a past-the-end target lands at the last frame, and the skip watermark then admits no packet" | module docs :34-37 | REAL | test :361 `assert_eq!(packets, 0)`; demux test `adts_seek_past_the_end_lands_at_the_last_frame ... ok` (baseline test-workspace.log.txt:1542). |
| Errors: "ADIF VBR/multi-program refusals, seek-range violations" | doc comment :56-60 | REAL | demux tests `adif_vbr_is_the_typed_refusal`, `adif_multi_program_is_the_typed_refusal` ok (baseline :1538, :1540); `error::seek_out_of_range` (crates/demux/src/adts/error.rs:204). |

## 7. Dead / duplicated / unreachable code
None. Both production functions are reachable (registry arm containers/mod.rs:52); no dead helpers; `wire_error`/`SelectDemuxer`/`OpenOptions` are the shared seam (demux_host.rs, other agents' scope). The structural duplication of `open()` across the 11 container modules is the family template by design (containers/mod.rs:5-15 collision rule) — flagged at family level, not per-file.

## 8. Dependencies & flags
Imports: `commands::CommandError`, `demux::adts::AdtsDemuxer`, `media_types` (Track/Stamp/TimeSpan/Timestamp), `plugin_api::{Demuxer, SourceStream}`, `media_clock::NANO_BASE`, crate::demux_host types. No feature gates on the production code (plain `mod adts;` in containers/mod.rs:22 — compiled in the shipped build; lumen-cli has no relevant cfg). Test-only: `testkit::Must`, `demux::adts::test_support`, `LUMEN_CORPUS` env knob (test harness, correctly annotated for lint-literals).

## 9. Open questions for the next agent
- demux_host.rs owner: `SelectDemuxer::start()` filters `_ => false` for non-A/V kinds — ADTS is audio-only so unaffected; confirm no container mints subtitle tracks through this wrapper.
- host.rs owner (part containing `host.rs:904`): does the session consume `start_position_ns` as the anchor, and does any consumer depend on the seek *landing* rather than the requested time (family-wide: all container hosts discard the landing)?
- The orchestrator should state whether `/tmp/my-project/corpus-v4/media` existed during the no-fail-fast run, to settle whether the corpus rows in that log are executed passes or skips (REQUEST-ORCH-RUN noted in log).
