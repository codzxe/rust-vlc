# AUDIT: apps/cli/src/lib.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 83 / 10 module declarations + 1 re-export (documentation-only crate root) |
| File verdict | REAL-UNPROVEN (structure real; crate-level docs rotted in three places) |
| Reachable from shipped entry point? | YES — it IS the library root: main.rs:13/19/22 `lumen_cli::bench::mark_process_start` / `lumen_cli::run::run` reach it; every module it declares is the shipped surface. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n` quote verification (all quotes verbatim); cross-checks against native_seek.rs, containers/mod.rs, pipeline_host.rs. NO cargo (adaptation). |

## 1. One-paragraph verdict

An 83-line composition root that is 90% documentation. The structure is real and honest about its architecture (single dispatch entry, module map, pacing model — the pacing claims are corroborated by the W00 smoke: clock pinned at 0.000000s until pumped). But the crate-level documentation rotted in three places: it still describes the seek as the host-side `stop → close → open-at-target` composition that the native-seek suite (W1-06) explicitly kills; it still names `AviDemuxer` as THE demuxer of the open path (15 container families are wired today); and it still says the decoder set is "MJPEG + PCM" (the provider dispatches 14 codecs, including the h264 arm at the center of this audit). Doc rot at the crate root is the map every new reader starts from.

## 2. What actually works (verified)

- Module declarations match the file tree exactly (exit, parse, help, demux_host, pipeline_host, bench, containers, host, dispatch, run — all present in apps/cli/src/).
- The pacing documentation (lib.rs:57-63) is TRUE per W00 evidence: mp4-smoke2-stderr.txt shows `ok: time 0.000000s` until the script's explicit verbs move it — "the engine advances only when the host pumps it" is the observed behavior.
- The ARCH-5 claim (every surface through `lumen_dispatch`) matches run.rs (all three modes call it — verified in the run.rs read for this audit's chain proofs).

## 3. Findings

### F-001 [S3] — Crate doc describes the dead reopen-seek composition

Where: apps/cli/src/lib.rs:52-57

Code (verbatim):
```rust
//! - **Seek** (doc 05 §2.4) is composed host-side as
//!   `stop → close → open-at-target` through the real engine
//!   transitions, because the engine-level seek input is card 3.A1.
//!   The demuxer's real seek-to-time (`demux` 2.B2) resolves the
//!   landing; a wrapper drops pre-target packets. `[PROPOSAL]` — see
//!   [`host::CliHost::dispatch_seek`]'s docs.
```

Code (native_seek.rs:10-14, verbatim):
```rust
//! - **No reopen** (D-4/D-7): the session id and every session-scoped
//!   half (device, ring, clock, decoders, renderer) survive the seek;
//!   the state trail shows the in-pipeline transitions only — a
//!   `Stopped`/`Idle`/`Opening` after the seek would be the forbidden
//!   stop→close→reopen composition.
```

What is wrong: the crate root documents seek as the forbidden composition. W1-06 landed the in-pipeline seek (`host.dispatch_seek`, pinned by nine tests asserting `teardown_edges == 0`); this paragraph (and help.rs:57, and the `SelectDemuxer` comment at demux_host.rs:38-39 that still says "In-session position jumps are the engine's native seek flow (card 3.A1, wired by W1-06)" — that one is correct) was not updated.
Why it matters: the first doc a maintainer reads states the opposite of the tested behavior; help.rs F-001 shows the rot already reached user-facing output.
Confidence: HIGH — Verification: read (contradiction is between two in-repo texts; the native_seek pins are themselves runtime-UNPROVEN, but dispatch.rs:131 routes `PlaybackSeek` to `dispatch_seek` regardless).

### F-002 [S4] — Crate doc's open-path story is AVI-only, three years of landings stale

Where: apps/cli/src/lib.rs:5-6, 25-27, 36-38

Code (lib.rs:25-27, verbatim):
```rust
//! - [`demux_host`] — the open path: file URI → `FileAccess` →
//!   `probe_stream` → `AviDemuxer`, plus the seek/start-time skip
//!   wrapper and the track-facts pre-pass the decoder provider needs.
```
Code (lib.rs:36-38, verbatim):
```rust
//! - [`containers`] — THE container-host registry (6.OPW-PREP): the
//!   post-probe format dispatch — one `containers/<fmt>.rs` per wired
//!   family (avi today), the honest typed refusal for the rest.
```

What is wrong: "AviDemuxer" is the open path's demuxer only for AVI files; the registry wires 15 probe ids (containers/mod.rs:52-73: adts, aiff, avi, ivf, matroska, flac, hevc, h264, mp2, mp3, mp4, ogg, mpegps, mpegts, wav). "(avi today)" is FALSE. Line 6's "decode (`codecs`, MJPEG + PCM)" is FALSE (pipeline_host.rs:241-380 dispatches 14 codecs). The doc describes the D-018/2.D2 era.
Confidence: HIGH — Verification: read + mod.rs arm count.

### F-003 [S4] — Module map's `demux_host` line buries the HTTP lane

Where: apps/cli/src/lib.rs:25-27 vs demux_host.rs:399-440

What is wrong: the crate-root summary omits that `open_source` also opens `http(s)` URLs through the streaming crate (w31.3) — the open path's second access lane is invisible at the crate root. Minor incompleteness, same rot class as F-002.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc (architecture claims) | 1-66 | REAL-UNPROVEN | S3/S4 (F-001/F-002/F-003) | Pacing claims verified against W00; seek/container/codec claims stale. |
| `pub mod` declarations ×10 | 68-77 | REAL | — | Match the file tree exactly. |
| `pub use media_clock::MonotonicSource` | 83 | REAL-UNPROVEN | — | Re-export for hosts injecting a deterministic source; used by first-vertical evidence harness (tests/integration — W00 baseline ran that crate). |

## 5. Test honesty

No tests in this file (the crate's tests live in the modules and tests/ — audited separately).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every host surface … builds a CommandRequest and calls lumen_dispatch" | :10-15 | REAL | run.rs three modes verified. |
| "decode (codecs, MJPEG + PCM)" | :6 | FALSE | pipeline_host.rs:241-380. |
| "(avi today)" for the container registry | :38 | FALSE | containers/mod.rs:52-73 (15 arms). |
| "Seek … composed host-side as stop → close → open-at-target" | :52-54 | FALSE (stale) | native_seek.rs:10-14; dispatch.rs:131. |
| "the engine advances only when the host pumps it" | :58-59 | REAL | W00 mp4-smoke2-stderr.txt (time 0.000000s until pumped). |
| "The app's own logic is std-only; its two external dependencies (serde_json, toml)" | :63-66 | REAL-UNPROVEN | apps/cli/Cargo.toml dependency list corroborates (serde_json + toml the only non-path externals). |

## 7. Dead / duplicated / unreachable code

- None.

## 8. Dependencies & flags

- Declares the crate's modules unconditionally; no feature gates at this level.

## 9. Open questions for the next agent

- None beyond F-001's cross-file sweep (lib.rs, help.rs, and any other file repeating the reopen story should be fixed together).
