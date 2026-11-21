# WAVE 8 — SUMMARY (audio-output: null backend, cpal seam, ring, pump, render)

Date: 2026-11-21 (session 2) · Agents W08-A01…A10 · Domain: audio-output crate.
Gate: **PASS — 11/11 assignments audited, 0 missing, 0 thin, citation
spot-checks clean** (2 citations used bare filenames instead of full paths —
resolved against the audited source and verified verbatim; recorded here as an
audit-hygiene note, not a fabrication).

## Verdicts (file level)
REAL 6 · UNREACHABLE 3 · STUB 2

## Findings
**S0:1 S1:1 S2:7 S3:22 S4:20 — total 51**

## The wave's hostile question: how does a user hear sound?
**They cannot. Ever.** Convergent verdict of all 10 agents:
- Every shipped audio path (CLI, web) terminates in the **null discard sink**:
  decoders → NullBackend → ring → dropped scratch buffer (host.rs:613-617 /
  1270-1276 render loop discards).
- The **cpal gate is compile-time closed** (cpal.rs:43; AUDIO_ADR005_ADMITTED=false);
  `cpal_backend()` is a typed refusal with zero production callers; the
  documented "one change" admission recipe still yields a silent player.
- **product.toml:150 claims "default audio device output" — FALSE** (corroborates
  W01-A08 F-003).
- The plumbing itself (ring, render, counter, pump) is real and baseline-tested,
  but `AudioBackend`/`builtin_backends`/`PcmPump`/`RenderCallback` are dead
  surface: zero shipped call chains; the ring's cross-thread contract is never
  exercised by two threads anywhere.

## Worst findings (citations in per-file audits; sampled at gate)
1. [S0] null.rs — every shipped audio path ends in this discard sink; no
   platform backend is even linked; no user can hear sound in any shipped
   build (W08-A01).
2. [S1] Crafted-MP4 `stsd` channelcount flows unbounded into a 131,072-frame
   ring → ~34 GB allocation → abort at `lumen open` (W08-A01; REQUEST-ORCH-RUN
   filed for an executed proof).
3. [S2] pump.rs:196-217 — any buffer larger than `high_water` can never be
   accepted: permanent Backpressure livelock or 100% Drop loss; `with_base`
   device-switch clocks make every push fail forever (W08-A02).
4. [S2] `audio.partial_frame` minted in two crates at different pipeline
   stages — breaks the stable-code "one auditable home" contract (W08-A09).

## Blocked / UNVERIFIED
- F-002 abort proof not executed (agents run no cargo; REQUEST-ORCH-RUN filed).
- tauri audio wiring UNVERIFIED-BUILD (standing baseline limitation).
- cpal/alsa-sys sandbox experiment: record-cited only.
