//! `apps/web` — the engine-in-Worker wasm host (card 7.H1, issue
//! #47; doc 09 §1/§2/§3/§7).
//!
//! # What this crate is
//!
//! The wasm32 composition root of the Lumen engine: the SAME std-only
//! engine core the CLI drives (probe → demux → engine → queues →
//! clock → scheduler), exposed to a Web Worker through
//!
//! 1. a typed [`HostToWorker`]/[`WorkerToHost`] MessagePort contract
//!    ([`protocol`]) whose command lane carries the ONE command
//!    layer's own envelopes (ARCH-5 — the 64-command / 23-event
//!    catalog rides inside, it is never re-defined here), and
//! 2. a hand-rolled `extern "C"` ABI over shared linear memory
//!    ([`abi`]) — **zero new dependencies**: doc 09 §2.2 recommends
//!    wasm-bindgen as a `[PROPOSAL]` (its OQ-1 is undecided), but the
//!    card's envelope prefers none, and nothing in doc 09 §3 (the
//!    threading model) requires it: v1 is the single-threaded
//!    cooperative lane, and `wasm-bindgen-rayon` is only the §3.3
//!    v2 (SharedArrayBuffer + COOP/COEP) path. The choice is
//!    recorded here and in the crate manifest; if wasm-bindgen is
//!    ever ratified, it slots in behind this same [`abi`] boundary
//!    without touching an engine crate.
//!
//! # The engine under it
//!
//! [`host::WebHost`] is the CLI host's web twin at v1 scope: it owns
//! the [`playback_engine::Engine`], correlates receipts to requests,
//! converts engine events to the wire stream through the staleness
//! filter, and — the web-specific half — drives everything through
//! **cooperative pump slices** (doc 09 §3.2): one [`HostToWorker::Pump`]
//! message is one bounded slice (audio fill first, then a bounded
//! engine poll batch, then a bounded event flush), after which
//! control RETURNS to the Worker event loop. Nothing blocks, nothing
//! spins: the JS glue re-arms the pump from `setTimeout(0)` ticks
//! while the [`protocol::PumpReport`] says `busy`.
//!
//! # Honest posture (doc 09 §10)
//!
//! The boot handshake answers `EngineInfo` + the capability snapshot
//! ([`capabilities`]): `threading: single` (the truth rule), the
//! canvas/AudioWorklet/fetch lanes named as typed blocked rows (cards
//! 7.H2/7.H3/7.H4), and — this build's own limit — the wasm32 module
//! ships the engine core WITHOUT the builtin decoders (the `codecs`
//! crate's external decode engines do not build for
//! `wasm32-unknown-unknown`; measured — see [`capabilities`]'s module
//! docs). Native builds link the real MJPEG + PCM decoders so the
//! headless lane proof decodes for real.
//!
//! # Evidence labels
//!
//! The CI proof is **headless-native**: a `cargo test` lane drives the
//! SAME message paths (boot → `loadBlob` → `session.open` → pump
//! slices → events) through the wasm ABI's dispatch core with an
//! in-memory MessagePort double, and `cargo build --target
//! wasm32-unknown-unknown --locked` proves the real module artifact.
//! Browser-runtime behavior (worker instantiation, `postMessage`
//! transport, `performance.now` arming) is `[VERIFY]` — no browser
//! exists in this sandbox, and nothing here claims it was exercised.

pub mod abi;
pub mod capabilities;
pub mod host;
pub mod io;
pub mod open;
pub mod protocol;
pub mod source;

pub use capabilities::{HostCapabilities, TargetProfile, capabilities};
pub use host::{SliceBudgets, WebHost};
pub use protocol::{HostToWorker, PumpReport, WorkerToHost};
pub use source::{BlobRegistry, BlobSource};
