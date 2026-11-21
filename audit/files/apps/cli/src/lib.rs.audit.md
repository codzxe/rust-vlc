# AUDIT: apps/cli/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2026-11-21 |
| Lines / functions | 83 / 12 items (1 module-doc block, 10 `pub mod` declarations, 1 re-export) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — this IS the shipped crate root: `lumen` binary → main.rs:19 `lumen_cli::run::run` and every `pub mod` here is on that path; the re-export is consumed by tests/integration/first-vertical/src/harness.rs:56 (`impl lumen_cli::MonotonicSource for FrozenThrottle`). EXECUTED: every baseline smoke + 129-test lumen_cli unit binary (orchrun log 2769-2770) + the first-vertical harness lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `Read` over the full file; `Read` apps/cli/src/containers/mod.rs (registry: 11 modules at 22-32, dispatch arms 50-75 — vs the doc's "avi today"); `rg "pub enum PushOutcome" …` chain from my part3 audit (provider codec table pipeline_host.rs:241-380 — vs the doc's "MJPEG + PCM"); `rg -n "fn dispatch_seek" -A50 apps/cli/src/host.rs` (1059-1130: the ENGINE's native `handle.seek` at 1106-1108 — vs the doc's stop→close→open-at-target note); `rg "MonotonicSource" repo` (re-export user: harness.rs:56 — live); `rg "pub struct CommandRequest|WireRequest" crates/commands/src apps/cli/src/dispatch.rs` (envelope.rs:49 `CommandRequest<C>`; dispatch.rs:62 takes `&WireRequest` — the doc's ARCH-5 claim verified); `rg "serde_json" apps/cli/src/run.rs` (332/350/378/513 — the runner itself uses serde_json, contra the "externals are the command layer's" attribution); `rg "lumen_cli" tests/integration/first-vertical/src/harness.rs` (the in-memory harness drives the real dispatch). |

## 1. One-paragraph verdict
The crate root is a real, accurate-as-to-architecture front door whose single-dispatch claim (ARCH-5) is verified true — all three host modes and the teardown path build a request envelope and call `lumen_dispatch` (run.rs:165/198/272/349/488), and the `MonotonicSource` re-export has exactly its documented consumer (the first-vertical harness, harness.rs:56). What has rotted is scope: the module map still describes the first vertical as the present — "AVI per D-018", "`AviDemuxer`", "one containers/<fmt>.rs per wired family (avi today)", "decode (codecs, MJPEG + PCM)" — while the registry wires 11 container families and the provider dispatches 13+ codec families including the h264 software decoder whose existence Wave-0 itself briefly disbelieved; and the "Honest scope notes" describe seek as the retired host-side stop→close→open-at-target composition when `dispatch_seek` has been submitting the engine's native seek (host.rs:1106-1108) for some time. Underclaiming docs are less dangerous than overclaiming ones, but this exact underclaim pattern (mirroring the hard-coded `engine-info` literal, INDEX standing defect) already produced one false audit headline; the crate's front door should not be a second instance of it.

## 2. What actually works (verified)
— ARCH-5 single dispatch TRUE: every mode funnels through `lumen_dispatch` — verified by my run.rs part1 audit (one_shot:165, repl_mode:198/272, machine_mode:349, teardown_step:488) and dispatch.rs:62's signature (`&WireRequest`); the first-vertical harness drives the same entry in-memory (harness.rs:40).
— The module map's 10 entries match the 10 `pub mod` declarations 1:1 (68-77) — no phantom or missing modules.
— `pub use media_clock::MonotonicSource` (83) is live for exactly its documented purpose: tests/integration/first-vertical/src/harness.rs:56 implements it (`FrozenThrottle`) without a media-clock dependency of its own.
— The pacing scope note (58-62) is TRUE and was verified in my run.rs audit: command-boundary pumping + the explicit `advance` verb, zero wall-clock sleeps, bounded settles (AWAIT_PUMPS=64).
— "The app's own logic is std-only" holds for the runner core (std BufRead/Write only); the two externals are indeed just serde_json + toml (per apps/cli/Cargo.toml audit) — though the attribution of serde_json solely to "the command layer" is imprecise (F-003).

## 3. Findings

### F-001 [S3] The module map is months stale: "avi today" / "AviDemuxer" / "MJPEG + PCM" describe the first vertical, not the present registry and provider
Where: apps/cli/src/lib.rs:4-7, 25-32, 36-38
Code:
```
//! drives the **real** pipeline — access (`input`) → probe
//! (`media-probe`) → demux (`demux`, AVI per D-018) → packet queues →
//! decode (`codecs`, MJPEG + PCM) → audio (`audio-engine` →
```
(lines 4-6, verbatim)
```
//! - [`demux_host`] — the open path: file URI → `FileAccess` →
//!   `probe_stream` → `AviDemuxer`, plus the seek/start-time skip
```
(lines 25-27)
```
//! - [`containers`] — THE container-host registry (6.OPW-PREP): the
//!   post-probe format dispatch — one `containers/<fmt>.rs` per wired
//!   family (avi today), the honest typed refusal for the rest.
```
(lines 36-38)
What is wrong: three scope claims are false against the code they document. (a) The registry wires ELEVEN container families — containers/mod.rs:22-32 declares adts, aiff, avi, es, ivf, mkv, mp4, ogg, ps, ts, wav, and the dispatch (50-75) serves 15 probe ids — not "avi today". (b) `demux_host`'s product is not `AviDemuxer`: the open path dispatches by probed format through that registry (demux_host → containers::open). (c) The decode stage is not "MJPEG + PCM": `HostDecoderProvider::decoder_for` dispatches mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_* (pipeline_host.rs:241-380) — the h264 software decoder included, whose existence the INDEX correction v2 had to re-establish after Wave-0 disbelieved it (the same underclaim pattern, there via the hard-coded engine-info literal).
Why it matters: the crate's front door underclaims by an order of magnitude; a reader (or the next auditor) tracing "what does this CLI decode/demux" from the crate docs gets the 2026-10 first vertical, not the present. Doc drift of the exact species that already produced one false project headline.
What correct looks like: the map's demux/decode lines name the registry and the provider's real dispatch table (or stay codec-agnostic: "every wired container family" / "the builtin decoder registry"), with the first-vertical framing moved to a history note.
Confidence: HIGH — Verification: read of this file + containers/mod.rs:22-75 + pipeline_host.rs:241-380 (my part1-part3 reads this session) + INDEX CORRECTION v2.

### F-002 [S3] The "Honest scope notes" describe the RETIRED seek composition as current — the native engine seek has landed
Where: apps/cli/src/lib.rs:50-57
Code:
```
//! - **Seek** (doc 05 §2.4) is composed host-side as
//!   `stop → close → open-at-target` through the real engine
//!   transitions, because the engine-level seek input is card 3.A1.
```
What is wrong: `CliHost::dispatch_seek` (host.rs:1059-1130) does not compose stop→close→open-at-target — it converts the wire target and submits the ENGINE's native seek: `handle.seek(request, engine_target)` (host.rs:1106-1108), then performs the seek-boundary host duties (`seek_boundary`, the ring's exact-prefix discard, the consumer's generation publish — pipeline_host's `ConsumerState::seek_boundary`, my part3 range). The engine-level seek input (card 3.A1) exists; the doc says it does not and describes the old workaround as the present mechanism. (The note's own pointer — "see [`host::CliHost::dispatch_seek`]'s docs" — leads to the current truth, contradicting the note above it.)
Why it matters: the two mechanisms have completely different observable behavior (session/generation continuity vs teardown-and-reopen; different events, different latency). A developer debugging or extending seek from the crate docs would be tracing an architecture that no longer exists. Doc-only, no runtime effect — S3.
What correct looks like: the note states the native engine seek is wired (host.rs `dispatch_seek` + the seek-boundary duties) and keeps only the genuinely-open part (e.g., the MP4 demuxer's `reanchor` gap noted in containers/mp4.rs:173-178).
Confidence: HIGH — Verification: read of host.rs:1059-1130 (the native `handle.seek` submission) + pipeline_host.rs seek-boundary machinery + containers/mp4.rs:173-178 (the reanchor honest gap).

### F-003 [S4] "its two external dependencies (serde_json, toml) are the command layer's and the settings validator's own wire formats" — the runner itself calls serde_json directly
Where: apps/cli/src/lib.rs:63-66
Code:
```
//! - The app's own logic is std-only; its two external dependencies
//!   (`serde_json`, `toml`) are the command layer's and the settings
//!   validator's own wire formats — see `Cargo.toml`'s header for the
//!   ARCH-5 rationale.
```
What is wrong: `run.rs` — unambiguously the app's own logic — calls `serde_json::from_str`/`to_string` directly (run.rs:332, 350, 378, 513: the machine-mode NDJSON loop and the `--json` envelopes). The externals list is right; the attribution ("the command layer's and the settings validator's own") is not.
Why it matters: a reader reasoning about the app's std-only boundary would exempt run.rs's serde dependency from scrutiny it demonstrably needs (the machine-mode parse path is attack surface W2-17 reasoned about).
What correct looks like: "…serde_json (the command layer's envelopes AND the runner's machine-mode/`--json` printing) and toml (the settings validator)".
Confidence: HIGH — Verification: read of run.rs's serde_json call sites (my part1 audit).

## 4. Per-item table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc block | 1-66 | REAL (architecture) / STALE (scope) | S3 (F-001, F-002, F-003) | ARCH-5 and pacing claims verified true; scope claims stale (F-001), retired-seek note false (F-002), externals attribution imprecise (F-003). |
| `pub mod bench` | 68 | REAL | — | decode-bench: argv-only verb (run.rs:144); A04's file this wave. |
| `pub mod containers` | 69 | REAL | — | The 11-family registry (containers/mod.rs:22-75); executed in every open smoke. |
| `pub mod demux_host` | 70 | REAL | — | The open path + facts pre-pass; A02/A03's files this wave. |
| `pub mod dispatch` | 71 | REAL | — | The single entry (dispatch.rs:62); A07's file this wave. |
| `pub mod exit` | 72 | REAL | — | Exit-code table; A10's file. |
| `pub mod help` | 73 | REAL | — | Generated help/version; A10's file. |
| `pub mod host` | 74 | REAL | — | CliHost/pump; A06/A08/A10 parts this wave. |
| `pub mod parse` | 75 | REAL | — | argv/REPL grammar; A01's file. |
| `pub mod pipeline_host` | 76 | REAL | — | Provider + consumer (my part3; A06/A07 parts 1-2). |
| `pub mod run` | 77 | REAL | — | The three modes (my part1; A07 part2). |
| `pub use media_clock::MonotonicSource` | 79-83 | REAL | — | Live re-export; sole documented consumer verified (harness.rs:56); prevents a media-clock dep in the harness — the stated purpose holds. |

## 5. Test honesty
No tests in this file. The crate-root's claims are tested indirectly by: the 129-test lumen_cli binary (orchrun log 2769+), the run::tests (2887-2898), the container suites, and the first-vertical harness — all cited in my other five audit files this wave.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every host surface … builds a CommandRequest and calls lumen_dispatch — the single dispatch entry" (ARCH-5) | this file 10-15 | REAL | run.rs:165/198/272/349/488 (my part1 audit) + dispatch.rs:62 + harness.rs:40. |
| "demux (demux, AVI per D-018)" / "→ AviDemuxer" / "(avi today)" | this file 5, 26, 38 | FALSE (stale) | containers/mod.rs:22-75 wires 11 families / 15 probe ids (F-001). |
| "decode (codecs, MJPEG + PCM)" / "the DecoderProvider (mjpeg/pcm …)" | this file 6, 29 | FALSE (stale) | pipeline_host.rs:241-380 dispatches 13+ codec families incl. h264 sw (F-001). |
| "Seek … composed host-side as stop → close → open-at-target … engine-level seek input is card 3.A1" | this file 52-54 | FALSE (stale) | host.rs:1106-1108 submits the native `handle.seek` (F-002). |
| "The app's own logic is std-only; its two external dependencies … are the command layer's and the settings validator's own" | this file 63-66 | PARTLY FALSE | serde_json is called by run.rs itself (332/350/378/513) — F-003. |
| "the `advance` host verb is the run-to-end driver (… zero wall-clock sleeps)" | this file 15-16, 58-62 | REAL | run.rs part1 audit (bounded settles, no sleeps; AWAIT_PUMPS=64). |

## 7. Dead / duplicated / unreachable code
- None: all 10 modules have consumers; the single re-export has its consumer (harness.rs:56).

## 8. Dependencies & flags
- The root declares no dependencies itself (module decls only); the crate's dependency posture (std-only app logic + serde_json/toml externals + the internal crate graph) is covered by apps/cli/Cargo.toml's audit. No feature gates, no unsafe, no cfg tricks at this level.

## 9. Open questions for the next agent
- Whoever owns the docs-refresh pass: F-001/F-002 here are the same drift family as mp4.rs's module-doc h264 claim (my file-2 audit, flagged for A07) and the engine-info literal (INDEX standing defect) — a sweep for first-vertical-era scope prose across crate roots would retire the whole class.
- A10 (host.rs parts): the seek note's replacement text (F-002) should cross-reference dispatch_seek's own docs (host.rs:1041+) — your file holds the current truth; please make sure your audit's seek section states the native flow explicitly so the contradiction is adjudicated in one place.
