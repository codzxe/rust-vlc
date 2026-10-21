//! # lumen-cli — the command-line player (card 2.D2; doc 05 §5.3)
//!
//! The headless host of the first vertical: a composition root that
//! drives the **real** pipeline — access (`input`) → probe
//! (`media-probe`) → demux (`demux`, AVI per D-018) → packet queues →
//! decode (`codecs`, MJPEG + PCM) → audio (`audio-engine` →
//! `audio-output`'s null device: no audio hardware in this sandbox) →
//! video (`video-engine` scheduler → `renderer-wgpu`'s software
//! compositor: no GPU in this sandbox) — all orchestrated by the
//! `playback-engine` state machine, and reached **only through the one
//! command layer** (ARCH-5): every host surface (argv subcommands, the
//! interactive REPL, the `--control-stdio` machine mode) builds a
//! [`commands::CommandRequest`] and calls [`lumen_dispatch`] — the
//! single dispatch entry, the CLI's shape of doc 05 §5.1's
//! `lumen_dispatch`.
//!
//! # Module map
//!
//! - [`exit`]     — `CommandError` → process exit codes (a total,
//!   tested table) + stage-tagged honest printing.
//! - [`parse`]    — the argv/REPL grammar: subcommand → catalog
//!   `Command`, the seek-time grammar (`90s`, `+10s`, `-1m`, `50%`).
//! - [`help`]     — `--help`/`--version` text, generated from the
//!   embedded `settings-schema` (per the card) + the command catalog.
//! - [`demux_host`] — the open path: file URI → `FileAccess` →
//!   `probe_stream` → `AviDemuxer`, plus the seek/start-time skip
//!   wrapper and the track-facts pre-pass the decoder provider needs.
//! - [`pipeline_host`] — the pipeline composition the engine drives:
//!   the `DecoderProvider` (mjpeg/pcm from track facts), the
//!   `FrameConsumer` (audio → `AudioPipeline` → null sink; video →
//!   I420→RGBA host conversion → `PresentationScheduler` → the
//!   software `Renderer`).
//! - [`host`]     — the [`host::CliHost`]: owns the `Engine`, pumps it
//!   (the host's poll IS the session thread, 2.C2's invariant S1),
//!   correlates receipts to requests, records events and state
//!   history, and answers the class-S snapshot commands.
//! - [`dispatch`] — [`dispatch::lumen_dispatch`]: the one entry; the
//!   served-command table and the honest typed refusals for the rest
//!   of the catalog.
//! - [`run`]      — the three modes: argv one-shot, the interactive
//!   REPL (doc 05 §5.3's controller mode), `--control-stdio` NDJSON
//!   machine mode.
//!
//! # Honest scope notes (recorded for the Lead)
//!
//! - **Seek** (doc 05 §2.4) is composed host-side as
//!   `stop → close → open-at-target` through the real engine
//!   transitions, because the engine-level seek input is card 3.A1.
//!   The demuxer's real seek-to-time (`demux` 2.B2) resolves the
//!   landing; a wrapper drops pre-target packets. `[PROPOSAL]` — see
//!   [`host::CliHost::dispatch_seek`]'s docs.
//! - **Pacing**: the engine advances only when the host pumps it
//!   (2.C2's synchronous single-writer model). The REPL/machine modes
//!   pump at command boundaries plus the explicit `advance` verb —
//!   fully deterministic, zero wall-clock sleeps. Continuous wall-time
//!   pumping joins with the session-thread card (3.B3).
//! - The app's own logic is std-only; its two external dependencies
//!   (`serde_json`, `toml`) are the command layer's and the settings
//!   validator's own wire formats — see `Cargo.toml`'s header for the
//!   ARCH-5 rationale.

pub mod demux_host;
pub mod dispatch;
pub mod exit;
pub mod help;
pub mod host;
pub mod parse;
pub mod pipeline_host;
pub mod run;

/// The event-throttle seam's source trait, re-exported for the hosts
/// that inject a deterministic source (card 3.B4 — the first-vertical
/// evidence harness) without taking a media-clock dependency of
/// their own.
pub use media_clock::MonotonicSource;
