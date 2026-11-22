AUDIT: apps/cli/src/containers/avi.rs
======================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 142 / 2 (both production) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open x.avi` → run.rs → dispatch → host.rs open path → `demux_host::open_source` → probe names `avi` → this file's `open` (line 35). Exercised end-to-end by session_script.rs (21 tests) and unsupported_path.rs through the real binary. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |

Commands run (static; no cargo per adaptations):
- Read file complete (single pass)
- Cross-read: pipeline_host.rs:241 (`match track.codec.as_str()` — the provider dispatches on the DEMUXER's track codec), bench.rs:300-313 (the bench dispatches on `fact.codec`), mp4.rs:106 (the same duration-default idiom).
- W00 baseline: `mjpeg-smoke-bench.txt` ("75/75 v + 130/130 a frames" on mjpeg+pcm AVI — this file's harvest feeding the bench) and `advance-test.txt` (AVI session behavior).

## 1. One-paragraph verdict

The first wired container host, and the thinnest: 78 lines of open-wiring (construct, start, harvest, clamp, seek-validate, wrap) plus a 58-line facts harvest that reads the demuxer's own `strf` mapping instead of duplicating tag tables — the right shape. The start-time clamp handles negative and past-end inputs correctly (`.clamp(0, duration_us)`, line 45 — the guard that also defuses parse.rs's overflow finding downstream). No fakeness, no arithmetic hazards (one saturating multiply per path). The one design-level defect is inherited from the facts model: any AVI stream that is neither MJPEG nor PCM harvests as codec `"unknown"` with no geometry, while the demuxer's own `Track.codec` may carry a real descriptor — two codec strings for one track, consumed by two different dispatchers (the pipeline reads `Track.codec`, the bench reads `fact.codec`), which is the same seam that lets the bench claim "no decoder" for codecs the pipeline decodes.

## 2. What actually works (verified)

— verified by static trace + W00 baseline: the full open path (`open`, 35-78) is the one session_script.rs drives 21 times through the real binary (accepted/open/pause/seek/stop transcripts on exactly this host's fixture); `harvest_facts` (84-142) feeds the strf pair that W00's `mjpeg-smoke-bench.txt` shows decoding 75/75 v + 130/130 a frames.
— verified by reading: MJPEG frame cadence `num*1e9/den` (90-91) matches the ivf host's (scale, rate) tick convention (ivf.rs:337-343 asserts the same math); PCM codec descriptor delegated to the demuxer's own mapping (109, "not a local copy" — the comment is accurate to the code); no `extra_data` invention (132, documented).
— UNVERIFIED at runtime here (no cargo); the binary-level tests above are the runtime evidence.

## 3. Findings

### F-001 [S3] — non-MJPEG/PCM AVI streams harvest as codec "unknown": dual codec truth between fact and track
Where: apps/cli/src/containers/avi.rs:117 (and the consumers' split: pipeline_host.rs:241 vs bench.rs:300-313)
Code:
```rust
            None => ("unknown".to_string(), None, None, None),
        };
```
What is wrong: `demuxer.stream_format(track.id)` only answers `MotionJpeg` or `Pcm` (the two `Some` arms at 88-116); every other AVI video codec (XviD/DivX/h264-in-AVI, common in real corpora) falls to the `None` arm: `fact.codec = "unknown"`, no pcm, no frame_ns, no dimensions. Meanwhile the demuxer's own `Track.codec` may carry a real descriptor, and the two are consumed by DIFFERENT dispatchers: the pipeline's `HostDecoderProvider` matches `track.codec` (pipeline_host.rs:241) while `decode-bench` matches `fact.codec` (bench.rs:300, 313).
Why it matters: for any non-mjpeg/pcm AVI stream the two surfaces can disagree about what is decodable (in-session dispatch on `Track.codec` vs bench refusal on `"unknown"`) — the same seam that already produces the false "no decoder in this build" bench message for h264 (see the W13-A01 log trace). The fact harvest also loses dimensions/cadence for those streams, degrading scheduler config (pipeline_host F-003's guessed cadence).
What correct looks like: harvest `track.codec` as the fact's codec when `stream_format` is None (or extend `StreamFormat`), so fact and track name the same truth.
Confidence: MEDIUM — Verification: read (the AVI demuxer's `Track.codec` minting for non-mjpeg streams is another wave's file; UNVERIFIED what it says — if it also says "unknown", the divergence shrinks to the dimension/cadence loss).
### F-002 [S4] — duplicated magic duration default
Where: apps/cli/src/containers/avi.rs:43
Code:
```rust
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
```
What is wrong: the `i64::MAX / 4` "effectively no clamp" default is copy-pasted per host (avi.rs:43, mp4.rs:106; ivf.rs documents the same shape differently at 85-89). No shared constant.
Why it matters: drift risk if one host tunes the bound; hygiene.
Confidence: HIGH — Verification: read (both files).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 35-78 | REAL | — | clamp(0, duration) at 45 (negative/past-end safe); saturating µs→ns at 53/59/75; seek query pre-validates; no panics. |
| `harvest_facts` | 84-142 | REAL | S3/S4 | F-001 `None` arm; F-002 is in `open`'s line 43; MJPEG cadence math correct per ivf convention; PCM pair from demuxer mapping (no local tag table — verified against the code). |

## 5. Test honesty

No tests in this file. Its behavior is covered by:
| External test | Would it fail if this file broke? | Reason |
|---|---|---|
| session_script.rs (21 tests) | YES | exact transcripts through this host's fixture (open/pause/seek/stop/teardown). |
| unsupported_path.rs flip16/missing/directory | YES | probe/input refusals on this family's shapes. |
| W00 mjpeg-smoke-bench | YES | 75/75 + 130/130 frame counts exercise the strf harvest. |
| No test covers a non-mjpeg/pcm AVI stream through this host | — | F-001's blind spot. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything else (skip wrapper, engine wiring, codec dispatch) is shared code" | avi.rs:8-10 | REAL | SelectDemuxer/demux_host shared; codec dispatch in pipeline_host. |
| "PCM channels/rate through the demuxer's own descriptor mapping — not a local copy of the tag/bits table" | avi.rs:81-83 | REAL | code at 109 delegates to `stream_format.codec_descriptor()`. |
| "AVI carries no out-of-band codec blob" | avi.rs:127-131 | REAL | `extra_data: None` (132); AVI strf has no codec-config payload for the first vertical's codecs. |

## 7. Dead / duplicated / unreachable code

- F-002's duplicated duration-default idiom (cross-host duplication, not intra-file).
- No dead code.

## 8. Dependencies & flags

- `demux::avi::{AviDemuxer, header::StreamFormat}`, `plugin_api`, crate `demux_host` types. No features. No unsafe.

## 9. Open questions for the next agent

1. crates/demux/src/avi — what does `Track.codec` carry for a non-mjpeg/pcm stream handler (fourcc? "unknown"?)? Decides F-001's blast radius.
2. demux_host::SelectDemuxer — the skip watermark's exact-drop semantics shared by all hosts (this file relies on it for `--start-time`); its audit belongs to whoever owns demux_host.rs.
