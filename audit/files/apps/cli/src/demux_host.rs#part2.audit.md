# AUDIT: apps/cli/src/demux_host.rs#part2 (lines 500–999 of 999)
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-21 |
| Lines / functions | 500 (chunk of 999; part1 lines 1–499 → W13-A02) / 20 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` open/play → `Host::open_session` (apps/cli/src/host.rs:890-904) → `open_source` (demux_host.rs:362, part1) → every container `open()` constructs `SelectDemuxer` (e.g. ogg.rs:102) → engine consumes it as `Box<dyn Demuxer>`; proven live by baseline open-session transcripts (mp4-smoke-open-session.txt: tracks ready → Playing) and the 10 passing demux_host tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -n "start_position_ns|reanchor" apps/cli/src crates/playback-engine/src` → engine seek flow + open anchor (host.rs:906); Read crates/plugin-api/src/demux.rs:65-119 (trait: `seek` = pure query, `reanchor` = compute-then-commit — wrapper forwarding is contract-correct); Read crates/demux/src/ogg/mod.rs:1300-1312 (inner `start()` idempotent — cached tracks on 2nd call, so the host-pre-pass + engine double-start is safe); Read crates/media-types/src/time.rs:175-180 (`Timestamp::convert` uses the timestamp's own embedded base); `rg demux_host::tests audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 10/10 ok (lines 2844-2853); `ls tests/fixtures/generated/malformed` → avi_mjpeg_pcm__flip16.avi exists |

## 1. One-paragraph verdict
The shared open-path wrapper is small, correct, and honest: the `--no-video`/`--no-audio` selection is a track-list filter (disabled kinds' packets still flow and die at the engine's unwired-track boundary, documented), the `--start-time` watermark drops only judgeable pre-target packets (never on a guess), `seek` forwards because the trait's `seek` is a pure query, and `reanchor` correctly kills the watermark on landing (otherwise a backward seek below the start time would starve — the comment explains exactly this). The ten tests are real and all passed in the baseline no-fail-fast run, including a genuine loopback-HTTP ranged-open test. The only defects are hygiene: a doc comment describing a per-track conversion "through the harvested bases" that the code does not perform (the harvested `TimeBase` values are dead data — stamps convert through their own embedded base), an unobservable silent packet drop, and a swallowed `fs::write` error in a test setup.

## 2. What actually works (verified)
- All 10 `demux_host::tests` passed — verified by `rg "demux_host::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → lines 2844-2853 (the primary test-workspace.log.txt never ran lumen-cli's unit tests — it aborted at desktop's zero_alloc_transport failure first; same caveat as my ogg.rs audit).
- `http_url_opens_through_the_ranged_resource` does real loopback-socket work and asserts `Range` headers were sent — the http lane is exercised, not mocked past the seam.
- Wrapper correctness cross-checked against the trait contract (plugin-api/src/demux.rs:70-103): `seek` never moves position → not clearing the watermark in `SelectDemuxer::seek` (592-594) is right; `reanchor` commits → clearing the watermark there (606-618) is right.
- Double-`start()` safety: container hosts call `inner.start()` at open (e.g. ogg.rs:82), the engine calls `wrapper.start()` later; OggDemuxer::start returns cached tracks on repeat calls (ogg/mod.rs:1301-1304) — no state corruption.

## 3. Findings

### F-001 [S4] Doc claims the skip conversion runs "through the harvested bases" — the harvested `TimeBase` values are never read
Where: apps/cli/src/demux_host.rs:513-516 (claim) and 555-561 (code)
Code (claim):
```rust
/// subtitle/data tracks. Packets that predate [`Self::skip_until_ns`]
/// (converted per-track through the harvested bases) are dropped here
/// — they are the pre-landing rewalk the seek query described.
```
Code (reality):
```rust
        if !self.bases.iter().any(|(id, _)| *id == packet.track) {
            return false;
        }
        match pts.convert(&media_clock::NANO_BASE) {
            Ok(ns) => ns.ticks < skip_ns,
            Err(_) => false,
        }
```
What is wrong: `|id, _|` discards the `TimeBase`; the only use of `self.bases` is a membership check (packets of tracks unknown at open — e.g. chained-Ogg later links — pass unjudged). The actual conversion is `pts.convert(&NANO_BASE)`, which uses the timestamp's own embedded base (media-types/src/time.rs:175-180), not any harvested base. The behavior is correct (stamps self-describe), but the doc describes a mechanism that does not exist and the `bases: Vec<(TrackId, TimeBase)>` field carries dead data in its second element.
Why it matters: a reader auditing timestamp handling would trace a per-track base lookup that never happens; dead fields in the wrapper every container populates (each `open()` builds this vec) invite a future "fix" that double-converts.
What correct looks like: either drop the field to `Vec<TrackId>` (membership only) and fix the doc, or actually convert through the harvested base and document why the stamp's own base cannot be trusted.
Confidence: HIGH — Verification: read (both sites + `Timestamp::convert` semantics verified; `rg "\.bases"` in file — only `new`/`before_skip` touch it).

### F-002 [S4] Watermark drops are silent — no counter, no event, no log line
Where: apps/cli/src/demux_host.rs:581-585
Code:
```rust
                DemuxEvent::Packet(packet) => {
                    if self.before_skip(&packet) {
                        continue;
                    }
                    return Ok(DemuxEvent::Packet(packet));
```
What is wrong: packets eaten by the `--start-time` watermark vanish with zero observability — no dropped counter (contrast the mailbox's `dropped_overflow`/`dropped_stale` stats and the demuxers' damage counters), no debug log, no event. For Ogg this can be a non-trivial fraction of the head of the file (see my ogg.rs audit: only page-terminal stamps are judgeable, so the drop count is small there, but for AVI it is every pre-target packet).
Why it matters: diagnostics gap, not wrong behavior — but this repo's own discipline (every drop counted elsewhere: mailbox.rs stats, ogg damage totals) makes this the one drop path with no accounting; a user reporting "nothing plays after --start-time" has no signal distinguishing watermark drops from a broken demux.
What correct looks like: a `dropped_by_watermark: u64` counter on the wrapper surfaced through the same diagnostics channels as the other drop counters.
Confidence: HIGH — Verification: read (no counter in struct 517-523; no log call in range).

### F-003 [S4] Garbage-file test swallows its own setup error
Where: apps/cli/src/demux_host.rs:781-782
Code:
```rust
        let garbage = std::env::temp_dir().join("lumen-cli-garbage.bin"); // lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface
        std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64)).ok();
```
What is wrong: `.ok()` discards the write result; if the temp write fails (read-only tmp, quota), the subsequent `open_source` fails at stage `Input` (missing file) and the test's `assert_eq!(err.stage, Stage::Probe)` reports a confusing probe-vs-input mismatch instead of "setup failed".
Why it matters: swallowed errors in tests are exactly the checklist's `let _ =` class; the failure mode is a lie about which layer broke.
What correct looks like: `.unwrap_or_else(|e| unreachable!("garbage write: {e}"))` — the file's own convention two lines up in other tests.
Confidence: HIGH — Verification: read.

## 4. Per-function table (lines 500–999 only)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SelectDemuxer` struct + docs | 502-523 | REAL | S4 | F-001 doc claim. Design documented: selection = track-list filter; disabled packets flow and die at the engine boundary; subtitle/data kinds always filtered (`_ => false` at 573 — matches "subtitles refused" reality). |
| `SelectDemuxer::new` | 529-543 | REAL | — | Plain constructor; `pub(crate)` — called by every container open. |
| `SelectDemuxer::before_skip` | 548-562 | REAL | S4 | Never-drop-on-a-guess rule: missing pts → pass, unknown track → pass, unconvertible → pass. Strict `<` keeps at-target packets (ogg test pins ≥). F-001 dead bases. No overflow path (`convert` is checked). |
| `Demuxer::start` (impl) | 566-576 | REAL | — | Filters by kind; all-disabled → empty list → engine's honest `NoDecodableTrack`. Idempotent inner start verified for Ogg (cached). |
| `Demuxer::next_event` (impl) | 578-590 | REAL | S4 | Loop drops watermarked packets (F-002 silence); all other events (EOS/Recovered/Discontinuity) forwarded untouched; errors propagate. |
| `Demuxer::seek` (impl) | 592-594 | REAL | — | Forward-only; correct because trait `seek` is a pure query (plugin-api/demux.rs:70-80) — watermark intentionally survives. |
| `Demuxer::seekability` (impl) | 596-604 | REAL | — | Forwards inner capability; comment explains the default-`None` trap avoided. |
| `Demuxer::reanchor` (impl) | 606-618 | REAL | — | Compute-then-commit forwarded; watermark cleared ONLY on success (error → position and watermark both unchanged — correct). The backward-seek starvation hazard is documented and handled. |
| `tests::fixture` | 625-629 | REAL | — | Repo-root tests/fixtures (exists). |
| `tests::parent_fixture_opens_with_expected_facts` | 634-674 | REAL | — | Real AVI fixture; pins container/tracks/duration 2s/mjpeg frame_ns 33,333,333..4/pcm (2,44100). Mutation-breaking. Passed (nofailfast:2850). |
| `tests::start_time_clamps_to_duration` | 684-724 | REAL | — | 9.999999s → clamp 2s; asserts 0 packets survive. Breaks if clamp or watermark breaks. Passed (…:2853). |
| `tests::select_demuxer_is_generic_over_a_non_avi_demuxer` | 733-775 | REAL | — | WAV instantiation proves genericity; i64::MAX/2 watermark drops all. Uses `demux::avi::test_support::mem_stream` (AVI helper for a WAV test — cross-module borrow, harmless). Passed (…:2852). |
| `tests::garbage_file_is_a_probe_stage_refusal` | 780-798 | REAL | S4 | F-003 `.ok()`. Asserts stage Probe/code Unsupported. Passed (…:2846). |
| `tests::magic_broken_derivative_is_refused_at_probe` | 803-819 | REAL | — | Fixture exists (verified); "sniffed" in message. Passed (…:2848). |
| `tests::missing_file_is_inaccessible` | 823-838 | REAL | — | Input/Inaccessible. Passed (…:2849). |
| `tests::blocked_scheme_urls_refuse_naming_the_lane` | 845-866 | REAL | — | rtsp → UnsupportedForHost naming the lane; asserts NOT "Phase 6" (drift guard). Passed (…:2844). |
| `tests::http_url_opens_through_the_ranged_resource` | 876-921 | REAL | — | Loopback mockhttp server; asserts URI identity, wav facts, non-empty request log, `Range` header present. Strongest test in the file. Passed (…:2847). |
| `tests::file_url_sources_open_like_path_sources` | 931-958 | REAL | — | Both RFC 8089 spellings; canonical URI equality. Passed (…:2845). |
| `tests::queue_plan_join_maps_product_limits_or_refuses_typed` | 969-998 | REAL | — | Exercises part1's `queue_plan_from_product_limits` (line 85, A02's range) — floor/ceiling typed refusals with axis named. Passed (…:2851). |
| retired-test comment block | 960-963 | REAL | — | Honest pointer to where the moved vectors now live (media-types w30.1). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parent_fixture_opens_with_expected_facts | YES | Break TrackFact harvest (wrong base/pcm) → asserts fail. |
| start_time_clamps_to_duration | YES | Remove clamp → start_position_ns ≠ 2e9; remove watermark → packets > 0. |
| select_demuxer_is_generic… | YES | Watermark no-op → packets > 0; AVI-coupled wrapper → compile failure. |
| garbage_file / magic_broken / missing_file / blocked_scheme | YES | Each pins stage+code+message; wrong layer or wrong message fails. |
| http_url_opens_through_the_ranged_resource | YES | Break the ranged resource → unreachable!; no Range header → assert. |
| file_url_sources… | YES | URI mismatch or open failure → unreachable!/assert. |
| queue_plan_join… | YES | Clamp instead of refuse → `unreachable!("below the floor must refuse")`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Packets that predate skip_until_ns (converted per-track through the harvested bases) are dropped here" | demux_host.rs:514-516 | FALSE (mechanism) / REAL (outcome) | F-001: conversion uses the stamp's own base; harvested bases are membership-only. |
| "an all-disabled media lands in the engine's honest NoDecodableTrack failure" | demux_host.rs:510-511 | REAL-UNPROVEN | Filter traced (`_ => false`); the engine-side failure name is playback-engine territory — not executed in this file's tests. |
| "The open-time skip watermark dies here [reanchor]… a --start-time filter left standing would drop every packet of a backward seek" | demux_host.rs:609-614 | REAL | Code at 615-617 clears on success only; trait D4 verified. |
| "the wrapper adds no capability the inner demuxer does not have" | demux_host.rs:597-603 | REAL | Pure forward at 603. |

## 7. Dead / duplicated / unreachable code
- `bases`' `TimeBase` payload — dead data (F-001).
- No unreachable functions: `SelectDemuxer::new` is called from every container module (adts/aiff/avi/es/ivf/mkv/mp4/ogg/ps/ts/wav — spot-verified ogg.rs:102 and the generic test's direct use).

## 8. Dependencies & flags
- No feature gates in range; `demux`, `media_types`, `plugin_api`, `media_clock` — all workspace crates on the default build. Tests additionally use `streaming::mockhttp` (loopback server — real sockets, no network egress; sandbox-safe) and `commands::catalog::types::SourceSpec`.

## 9. Open questions for the next agent
- W13-A02 (part1): `open_source_with`'s production `None`-transport arm (line 371-378) — what HTTP client ships when no test client is injected, and does the rtsp/blanket refusal table live there? My tests only prove the injected-client lane.
- Engine wave: for Ogg, pre-`--start-time` UNSTAMPED packets pass this watermark by design and reach the decoders; the anchor-epoch machinery (pipeline_host.rs `audio_floor`/`seek_boundary`, `open_audio_session(…, anchor_ns)`) is the backstop. No test in THIS file pins that open-time (non-seek) unstamped packets are gated downstream — the ogg tests only assert the watermark contract. If the audio ring accepts unstamped-decoded frames before the anchor, `--start-time` on Ogg plays pre-target audio. Worth one engine-side test.
- The `language: Some("voe")` garbage default flagged in the baseline INDEX is NOT in this file's range (Track minting is demux-side) — confirm with the demux wave.
