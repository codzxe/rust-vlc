//! Scenarios B — doc 10 §6.4 scenarios 9–16 (card 3.E2 part 2),
//! driven through the command layer over the real engine.
//!
//! The eight scenario modules each own one §6.4 row:
//!
//! | Module | Scenario | Pass criteria (doc 10 §6.4) |
//! |---|---|---|
//! | `scenario_09_corrupt` | corrupt file | typed `Corrupt`; bounded recovery count; no hang/crash |
//! | `scenario_10_missing` | missing file | typed `Inaccessible` with redacted message |
//! | `scenario_11_interrupted_network` | interrupted network | `Buffering` → resume or typed error; no hang (deadlines fire) |
//! | `scenario_12_device_loss` | device loss | `DeviceLost` event; position preserved or explicit suspension; recovery on return |
//! | `scenario_13_long_media` | long media | `Ended` reached; memory growth within budget |
//! | `scenario_14_short_media` | short media | edge cases per doc 03 §4.6 (Ended vs Buffering race) |
//! | `scenario_15_reopen_cycles` | reopen cycles | counters return to baseline (leak counters) |
//! | `scenario_16_exit_during_playback` | exit during playback | teardown within deadline; no temp state; resources released |
//!
//! Every scenario dispatches catalog [`commands::Command`]s through
//! [`host::lumen_dispatch`] — the local shape of doc 05 §5.1's entry
//! (see `host.rs` for the ARCH-5/ARCH-8 honesty note: the production
//! entry lives in `lumen-cli`, which this crate cannot dev-depend on
//! without the cycle boundaries.toml rejects). The engine, the state
//! machine, the AVI demuxer and the MJPEG/PCM decoders are the real
//! production stack; the named fakes are the scripted access source,
//! the scripted demux stage, and the observing/vanishing sink.
//!
//! # The counting allocator
//!
//! The process installs [`CountingGlobalAlloc`] so the D-2
//! bounded-memory assertions (reopen cycles, long media, exit during
//! playback) are exact in-process: allocator live-bytes deltas stand
//! in for RSS, which is not measurable in-proc. Every test takes the
//! whole-binary serializer first (the testkit discipline) so no
//! sibling test thread allocates inside a measured window.

#[global_allocator]
static ALLOC: testkit::CountingGlobalAlloc = testkit::CountingGlobalAlloc::new();

/// The counting allocator's counters (snapshot/guard access).
pub fn alloc_state() -> &'static testkit::AllocState {
    ALLOC.state()
}

#[path = "../common/mod.rs"]
mod common;

mod host;
mod probe;
mod scenario_09_corrupt;
mod scenario_10_missing;
mod scenario_11_interrupted_network;
mod scenario_12_device_loss;
mod scenario_13_long_media;
mod scenario_14_short_media;
mod scenario_15_reopen_cycles;
mod scenario_16_exit_during_playback;
