# AUDIT: crates/transcode/src/record.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 311 / 8 production items (Recorder: new, select, target, extra_data, record; Remuxer: new, track, remux) — a pure builder/assembly layer over `TranscodeGraph` + `submit` (graph.rs/runner.rs/wav.rs/mp4mux.rs are other agents' files) |
| File verdict | UNREACHABLE (real, integration-tested library code; no shipped binary links the transcode crate) |
| Reachable from shipped entry point? | NO. Census: zero references to `transcode::`/`use transcode` outside `crates/transcode/**` (rg over apps/ crates/ tools/); no app Cargo.toml depends on it. The feature-matrix rows for this work are honestly `not_started` with empty evidence (`playback-record` :1630-1639, `ui-dialog-convert` :2912-2921). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool full file; `grep -n` fn pins; `sed -n` verification of quotes; cross-file verification reads: runner.rs:150-175 (remux selection derivation), :1025-1075 (`select_tracks` DefaultAudio 0/1/many), Cargo.toml:57 (demux IS a normal dep — doc-link target valid), demux/src/mp4/mod.rs:669 (`sample_entry_bytes` exists); `rg "extra_data"` over tests (all `None`) + `rg "\.extra_data\("` repo-wide (zero callers); baseline: `orchrun-test-workspace-nofailfast.log.txt` :5880-5891 (mp4_remux 7/7 ok), tests/record.rs 5/5 ok (:5836-5842 region), :5831 (graph config-validation unit ok), doc-test `record.rs - record (line 50) - compile … ok` (log tail) |

## 1. One-paragraph verdict
This file is the one-call assembly layer for record-to-WAV and remux-to-MP4 — two builders whose entire job is constructing a `TranscodeGraph` and calling `submit`, and it does exactly that with no hidden logic. Unusually for this repo, every load-bearing doc claim checks out against code I traced in other agents' files: the "inert default" `DefaultAudio` in `Remuxer::remux` really is overridden inside `submit` (runner.rs:160-171 derives `TrackSelection::Tracks` from the config list — the claim that would sink video remux if false); "zero or several audio tracks are typed failures" matches `select_tracks` (runner.rs:1037-1047) with both branches unit/integration-pinned; the stream-copy-only refusal set and the honest-partial-on-cancel contract are pinned by executed e2e tests. The suite coverage is real: 5/5 record + 7/7 remux integration tests plus the compiling doctest ran green in the baseline, including packet-identical redemux and a mid-cancel playable-prefix check. One defect only: `Recorder::extra_data` is a public builder with zero callers anywhere in the repo and zero test coverage — every test constructs `extra_data: None` — so its documented wiring ("handed to the decoder at configure time") could be deleted and nothing would fail. None of it is reachable from a shipped binary, and the feature matrix says so honestly.

## 2. What actually works (verified)
— Integration execution, all green in baseline `orchrun-test-workspace-nofailfast.log.txt`: `tests/record.rs` 5/5 (`recorder_output_redemuxes_and_redecodes_bitexactly`, `recorder_surfaces_the_factory_refusal_typed`, `recorder_widening_target_records_losslessly`, `recorder_without_audio_fails_typed`, `mid_record_cancel_leaves_an_honest_playable_prefix`); `tests/mp4_remux.rs` 7/7 (:5880-5891, incl. `progressive_source_remuxes_and_redemuxes_packet_identical`, `fragmented_source_remuxes_into_a_progressive_output`, `remux_refuses_codecs_outside_the_stream_copy_set`, `submit_refuses_an_empty_remux`, `mid_remux_cancel_leaves_an_honest_playable_prefix`); graph validation unit `mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs` ok (:5831); doctest compiles (log tail: `record.rs - record (line 50) - compile … ok`, `no_run` honestly marked).
— Read-traced the "inert default" claim (the one comment that would hide a real bug): `Remuxer::remux` sets `selection: TrackSelection::DefaultAudio` (:302) and runner.rs:160-171 replaces it with `TrackSelection::Tracks(ids-from-configs)` inside `submit` before the pump sees it — a video-track remux is NOT filtered out by the default; proven live by the video-bearing e2e.
— Read-traced the DefaultAudio contract: runner.rs:1037-1047 — 0 audio → `audio_track_missing` (pinned by `recorder_without_audio_fails_typed`), >1 → `audio_track_ambiguous` (pinned by runner.rs:1172 unit assert on `transcode.source.audio-track-ambiguous`); `require_audio` refusal for explicit non-audio selections is the "never silently" half (:1053-1056).
— Doc-link targets verified real: `demux::mp4::Mp4Demuxer::sample_entry_bytes` exists (demux/src/mp4/mod.rs:669) and `demux` is a normal dependency of transcode (Cargo.toml:57) — the link resolves.

## 3. Findings

### F-001 [S4]
Where: crates/transcode/src/record.rs:191-200 (with runner.rs:171→465→501 as the plumbing)
Code:
```
    /// Out-of-band decoder configuration bytes (the WAV `fmt `
    /// payload, an AAC AudioSpecificConfig, …) when the container
    /// separates them from the stream — handed to the decoder at
    /// configure time. The host that opened the source knows whether
    /// its container carries any; the recorder never invents them.
    #[must_use]
    pub fn extra_data(mut self, extra: media_types::packet::SharedBytes) -> Recorder {
        self.extra_data = Some(extra);
        self
    }
```
What is wrong: Zero callers repo-wide (`rg "\.extra_data\("` → no hits in apps/, crates/, tools/) and zero test coverage: every `SourceSpec` constructed in the transcode test suites sets `extra_data: None` (job_lifecycle.rs :249/:468/:526, wav_roundtrip.rs :369/:470/:519/:548/:577, and both record/remux suites). The plumbing the doc describes exists (runner.rs:171 `build_plan(sink, decode, source.extra_data)` → :465 `run_decode(factory, extra_data, …)`), but no test anywhere passes bytes through it — the AAC-with-ASConfig shape the doc itself names as the use case is exactly the one never exercised.
Why it matters: The method's entire contract is the wiring; if `run_decode` stopped forwarding the bytes to `configure`, every test in the repo would still pass, and the first real host to hand an ASC to the recorder would get a decoder configured without it (garbage or refusal, discovered in production of the future host).
What correct looks like: One integration test in `tests/record.rs`: an AAC source whose ASC travels via `extra_data`, asserting the decoded output matches the inline-ASC control — the crate's own fixture machinery (codecs dev-dep) supports it.
Confidence: HIGH — Verification: read + repo-wide caller/test census (commands listed in header).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Recorder::new | 156-169 | REAL | — | Defaults: DefaultAudio selection, Source-flavor target. Pinned by every record e2e. |
| Recorder::select | 175-178 | REAL | — | Builder; non-audio explicit selection → typed `track_not_audio` (runner.rs:1053-1056). |
| Recorder::target | 186-189 | REAL | — | Widening-only legality lives in WavPcmTarget (wav.rs, other agent); `recorder_widening_target_records_losslessly` pins the legal case. |
| Recorder::extra_data | 197-200 | REAL (uncalled, untested) | S4 (F-001) | Zero callers, zero coverage; plumbing exists untested. |
| Recorder::record | 212-232 | REAL | — | Pure assembly: filters empty, WavPcm sink, decode Some. #Errors doc matches submit's real refusal set (verified against graph.rs validation + :5831 unit). |
| Remuxer::new / track | 260-277 | REAL | — | Builder; empty config list refused at submit (pinned by `submit_refuses_an_empty_remux`). |
| Remuxer::remux | 291-310 | REAL | — | decode None (stream copy); "inert default" selection override verified true (runner.rs:160-171); codec-set + missing-track refusals pinned by 2 e2e tests. |

## 5. Test honesty
No inline tests; the file is covered by two integration suites. Baseline: record 5/5, remux 7/7 ok.
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| recorder_output_redemuxes_and_redecodes_bitexactly | YES | Output re-demuxed and re-decoded by independent readers; byte-exactness across the whole chain. |
| recorder_surfaces_the_factory_refusal_typed | YES | Factory refusal propagates as job `Failed`, typed. |
| recorder_widening_target_records_losslessly | YES | Lossless-widening arithmetic checked on real decoded samples. |
| recorder_without_audio_fails_typed | YES | Pins the DefaultAudio-zero branch. |
| mid_record_cancel_leaves_an_honest_playable_prefix | YES | The honest-partial contract (RIFF/data sizes == written samples) verified on a real canceled job. |
| mp4_remux suite (7) | YES (each) | Packet-identical redemux (the strongest possible pin — compares through our own demuxer), fragmented→progressive, typed refusals for missing track / out-of-set codec / empty configs, honest partial on cancel, edit-list re-anchor. |
| extra_data wiring | NO TEST | F-001 — nothing would fail if the forwarding were dropped. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the sink's config list derives the real selection inside submit" (inert default) | record.rs:300-301 | REAL | runner.rs:160-171; video-bearing e2e green. |
| "zero or several audio tracks are typed failures asking the host to name one" | record.rs:141-143 | REAL | runner.rs:1037-1047 (both branches) + runner unit :1172 + `recorder_without_audio_fails_typed`. |
| "every other codec fails the job typed at the worker's cross-check, never a silent transcode" | record.rs:237-239 | REAL | `remux_refuses_codecs_outside_the_stream_copy_set` ok. |
| "on cancel the writer finalizes best-effort… a playable prefix, never a lie about its own length" | record.rs:32-35, 250-251 | REAL | Both mid-cancel e2e tests ok. |
| "handed to the decoder at configure time" (extra_data) | record.rs:191-195 | REAL-UNPROVEN | F-001: plumbing read-verified (runner.rs:465→501), never executed with data. |
| playback-record "not_started" all platforms, evidence empty | feature-matrix.toml:1630-1639 | REAL (honest) | No app links transcode (census); code is Phase-7 library work. |
| Live-capture gap "honestly-recorded, not hidden" | record.rs:21-28 | REAL (honest gap) | No tap trait exists; doc says so. |

## 7. Dead / duplicated / unreachable code
- Whole crate unreachable from shipped binaries (zero external references; matrix honestly not_started).
- `Recorder::extra_data` is the only uncalled public method in this file (F-001) — host-facing API awaiting its first consumer.

## 8. Dependencies & flags
`codec-api`, `demux` (normal dep — doc links resolve), `media-types`, `plugin-api` (all workspace-internal); dev: codecs/input/testkit for the fixture e2es. No feature gates, no unsafe, no I/O in this file (delegated to the sink trait object). `#[must_use]` on all builders.

## 9. Open questions for the next agent
- runner.rs / wav.rs owners: the `run_decode` extra_data forwarding (:465-501) — confirm the configure call actually consumes the bytes (my read says yes, but the file is yours); F-001's test belongs in your suite if not here.
- mp4mux owner: the `Mp4TrackConfig` verbatim-`stsd` contract ("the host harvests; the muxer never rewrites") — the e2e proves it for the two fixtures; breadth (edit-list, fragmented) is your half.
