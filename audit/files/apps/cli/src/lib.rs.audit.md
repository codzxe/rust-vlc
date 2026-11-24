# AUDIT: apps/cli/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 83 / 0 (crate root: 10 `pub mod` + 1 `pub use`) |
| File verdict | REAL (code) / crate-root doc STALE (first-vertical era, 4 false statements) |
| Reachable from shipped entry point? | YES — this IS the shipped crate: `lumen` bin (main.rs:19 `lumen_cli::run::run`) and `lumen-desktop` bin (apps/desktop/Cargo.toml:40 depends on lumen-cli); tests/compat + tests/integration/first-vertical also link it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | ls apps/cli/src/containers/ (11 modules); rg for `lumen_cli::MonotonicSource` (used at tests/integration/first-vertical/src/harness.rs:56); sed verifications; cross-executions from the sibling audits (h264+aac played via `lumen open`, exit 0). |

## 1. One-paragraph verdict
The code surface is 10 module declarations (all files exist, verified) plus one live re-export (`MonotonicSource`, consumed by the first-vertical harness at harness.rs:56) — nothing to break. The crate-root documentation, the first thing any reader of the shipped CLI opens, is frozen at first-vertical time and now makes four false statements: decode is "MJPEG + PCM" (12 more decoder families are wired and one pair — h264+aac — was executed playing this session), the open path ends at "AviDemuxer" (the probe dispatches 11 container families), the decoder provider is "mjpeg/pcm from track facts" (pipeline_host wires 14 codec descriptors), and containers are "avi today" (11 modules: adts, aiff, avi, es, ivf, mkv, mp4, ogg, ps, ts, wav). This is the crate-root member of the stale-registry family my assignment targets: three separate "first vertical" capability statements, none updated as waves 4-7 landed codecs and containers.

## 2. What actually works (verified)
- All 10 declared modules exist (ls apps/cli/src/): bench, containers, demux_host, dispatch, exit, help, host, parse, pipeline_host, run.
- `pub use media_clock::MonotonicSource` — live: tests/integration/first-vertical/src/harness.rs:56 `impl lumen_cli::MonotonicSource for FrozenThrottle`.
- ARCH-5 claim (every surface dispatches through `lumen_dispatch`) holds for the three player modes (verified in run.rs audit); decode-bench is the documented exception (lib.rs:33-35 says so honestly).

## 3. Findings
F-001 [S3]
Where: apps/cli/src/lib.rs:3-9, 25-32, 36-38
Code:
```
//! The headless host of the first vertical: a composition root that
//! drives the **real** pipeline — access (`input`) → probe
//! (`media-probe`) → demux (`demux`, AVI per D-018) → packet queues →
//! decode (`codecs`, MJPEG + PCM) → audio (`audio-engine` →
```
```
//! - [`demux_host`] — the open path: file URI → `FileAccess` →
//!   `probe_stream` → `AviDemuxer`, plus the seek/start-time skip
```
```
//! - [`containers`] — THE container-host registry (6.OPW-PREP): the
//!   post-probe format dispatch — one `containers/<fmt>.rs` per wired
//!   family (avi today), the honest typed refusal for the rest.
```
What is wrong: four stale capability statements, all first-vertical era: (a) L6 "decode (codecs, MJPEG + PCM)" — pipeline_host.rs:239-381 wires mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_* (executed this session: h264+aac MP4 played, exit 0); (b) L26 "probe_stream → AviDemuxer" — the open path dispatches 11 container families; (c) L29 "the DecoderProvider (mjpeg/pcm from track facts)" — same 14-descriptor reality; (d) L38 "(avi today)" — 11 container modules today. Why it matters: the crate root of the shipped binary understates the product by 12 decoder families and 10 containers; a reader auditing capability (or writing a matrix row) starts from this text and is misinformed — same defect family as engine-info's 2-entry list and bench's mjpeg+pcm gate. What correct looks like: one registry-truth sentence ("decoders and containers are enumerated by pipeline_host's provider and containers/mod.rs's match — see those"), or generated-from-code module docs. Confidence: HIGH — Verification: read + executed (sibling-file runs) + ls.
(Noted, not a separate finding: L13 says surfaces build a `commands::CommandRequest` — the code type is `WireRequest` (run.rs:161); naming drift, same stale-doc family.)

## 4. Per-function table
No functions; inventory: 10 `pub mod` (all verified present), 1 `pub use` (live). File is 83 LOC (<100) — table not mandatory, included for completeness.

## 5. Test honesty
No tests in this file (nothing testable — declarations only).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode (`codecs`, MJPEG + PCM)" | lib.rs:6 | FALSE (stale) | pipeline_host.rs:239-381; executed h264+aac playback this session |
| "probe_stream → AviDemuxer" | lib.rs:26 | FALSE (stale) | containers/mod.rs:50-66 dispatches 11 formats |
| "the DecoderProvider (mjpeg/pcm from track facts)" | lib.rs:29 | FALSE (stale) | pipeline_host decoder_for wires 14 descriptors |
| "one containers/<fmt>.rs per wired family (avi today)" | lib.rs:38 | FALSE (stale) | ls containers/: adts aiff avi es ivf mkv mp4 ogg ps ts wav |
| "every host surface … builds a CommandRequest and calls lumen_dispatch" | lib.rs:10-15 | REAL (modulo type-name drift CommandRequest→WireRequest) | run.rs one_shot/repl/machine all build WireRequest → lumen_dispatch |
| Seek composed host-side stop→close→open-at-target; pacing = command boundaries + advance verb | lib.rs:52-62 | REAL | verified in run.rs audit (settle/is_class_a/advance; host.rs settle 288-298) |
| "the app's own logic is std-only; its two external dependencies (serde_json, toml)" | lib.rs:63-66 | REAL | apps/cli/Cargo.toml [dependencies]: path deps + serde_json + toml only |

## 7. Dead / duplicated / unreachable code
None: every declared module exists; the single re-export has a consumer.

## 8. Dependencies & flags
- Declares nothing itself; the re-export pulls media_clock (already a path dep). No feature gates. Release lane compiles this crate (baseline exit 0).

## 9. Open questions for the next agent
- If the Lead fixes one of the three registries (engine-info list, bench gate, "landed:" string), the crate-root doc here should be regenerated in the same pass — it is the fourth member of the stale-set.
