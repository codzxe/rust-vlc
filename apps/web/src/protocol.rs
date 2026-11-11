//! The Worker MessagePort contract (card 7.H1; doc 09 §7, doc 05
//! §5.2): typed messages both directions over the port, with the
//! catalog's own envelopes riding inside — command in, event out.
//!
//! # Shape
//!
//! - Downstream (host → worker): the boot handshake, the blob
//!   pre-stage, the **catalog command envelope** ([`WireRequest`] —
//!   the same `{requestId, cmd, args}` object every other host
//!   sends), the cooperative **pump tick** (transport-level: the
//!   host's scheduler arm, doc 09 §3.2 — not a catalog command), and
//!   shutdown.
//! - Upstream (worker → host): the handshake reply carrying
//!   `EngineInfo` + the honest [`HostCapabilities`] snapshot (doc 09
//!   §7's boot sequence), blob confirmations, the **catalog response
//!   envelope** ([`WireResponse`]), the **event stream**
//!   ([`WireEvent`] batches, flushed per pump under the slice
//!   budget), per-pump reports, and the shutdown acknowledgment.
//!
//! The catalog's 64 commands / 23 events are NOT re-defined here —
//! they ride as the commands crate's own serde forms (the one
//! catalog, ARCH-5). Only the worker-transport control messages are
//! this crate's.
//!
//! # The blob lane (doc 09 §6.1)
//!
//! v1 pre-stages blob bytes through a dedicated `loadBlob` message:
//! the catalog's `SourceSpec` has no blob variant yet (doc 05 §5.2
//! plans it; adding one is a Lead-owned catalog edit), so the web
//! host accepts `SourceSpec::Url { url: "blob:<name>" }` and
//! resolves the name against the registry this message fills.
//! `[PROPOSAL: fold into the catalog's SourceSpec when doc 05 §5.2's
//! blob variant lands.]` The bytes ride the JSON contract in v1 (a
//! serde byte array) — a raw linear-memory blob lane is the H4
//! follow-up.
//!
//! # Wire form
//!
//! camelCase, `"type"`-tagged, mirroring the catalog's own serde
//! conventions (doc 05 §5.1). Round-trip fidelity is pinned by the
//! tests at the bottom of this file: every variant survives
//! `to_string`/`from_str` exactly — the contract the JS glue and the
//! wasm ABI both depend on.

use commands::catalog::types::EngineInfo;
use commands::{WireEvent, WireRequest, WireResponse};
use serde::{Deserialize, Serialize};

use crate::capabilities::HostCapabilities;
use crate::render::backend::{LaneTransition, LossReason, RenderBackend, RendererCaps};

/// One host → worker message (the downstream port direction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HostToWorker {
    /// The boot handshake: construct the engine host, answer
    /// `Booted` with `EngineInfo` + the capability snapshot. The
    /// page's renderer probe facts ride along (doc 09 §4.4: the
    /// probe runs BEFORE the boot report — the active backend is
    /// what the snapshot reports, never a guess); `None` is the
    /// honest "the page did not probe" (old glue, tests).
    Boot {
        /// The page's raw renderer probe facts, when it probed.
        #[serde(default)]
        renderer: Option<RendererCaps>,
    },
    /// Pre-stage one blob's bytes (doc 09 §6.1's File/Blob no-upload
    /// path — the file never leaves the page).
    LoadBlob {
        /// The registry name (referenced by `blob:<name>` URIs).
        name: String,
        /// The container bytes (JSON array in v1 — see the module
        /// docs).
        bytes: Vec<u8>,
    },
    /// One catalog command envelope (the `{requestId, cmd, args}`
    /// object — answered by [`WorkerToHost::Response`]).
    Command(WireRequest),
    /// One cooperative slice: run `ticks` bounded pipeline pumps,
    /// flush events, return control (doc 09 §3.2 — the worker loop
    /// is never blocked; the host re-arms while the report says
    /// busy).
    Pump { ticks: u32 },
    /// A live render surface died (the glue's `webglcontextlost` /
    /// `device.lost` report — doc 09 §4.4): the lane walks its
    /// fallback ladder and answers [`WorkerToHost::RenderDegraded`]
    /// with the transition (a stale report for a backend the lane is
    /// not presenting is ignored).
    SurfaceLost {
        /// Which backend's surface died.
        backend: RenderBackend,
        /// Why.
        reason: LossReason,
    },
    /// The page's `AudioContext` state report (card 7.H3, doc 09
    /// §5.3's autoplay lane): the page created the context on its
    /// play attempt and observed `context.state` — `running: false`
    /// is the suspended-and-not-gesture-authorized report (the host
    /// parks the audio lane and surfaces the typed
    /// [`WorkerToHost::GestureRequired`] event); `running: true` is
    /// the gesture-satisfied report (the gate clears and playback
    /// truly flows — the truth rule: never fake playing).
    AudioContext {
        /// Whether `context.state === "running"`.
        running: bool,
    },
    /// `system.shutdown`'s transport twin (graceful: closes the
    /// session, flushes events, answers `ShutdownComplete`).
    Shutdown,
}

impl HostToWorker {
    /// The message's `"type"` wire tag (diagnostics + tests).
    #[must_use]
    pub fn type_tag(&self) -> &'static str {
        match self {
            HostToWorker::Boot { .. } => "boot",
            HostToWorker::LoadBlob { .. } => "loadBlob",
            HostToWorker::Command(_) => "command",
            HostToWorker::Pump { .. } => "pump",
            HostToWorker::SurfaceLost { .. } => "surfaceLost",
            HostToWorker::AudioContext { .. } => "audioContext",
            HostToWorker::Shutdown => "shutdown",
        }
    }
}

/// One worker → host message (the upstream port direction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkerToHost {
    /// The handshake reply (doc 09 §7: `EngineInfo` + capability
    /// snapshot — the client transitions `connecting → ready`).
    Booted {
        /// The engine identity snapshot (`system.get-engine-info`'s
        /// own shape).
        engine_info: EngineInfo,
        /// The honest capability snapshot (doc 09 §3.3/§4.4/§10).
        capabilities: HostCapabilities,
    },
    /// A blob pre-stage confirmation.
    BlobLoaded {
        /// The registered name.
        name: String,
        /// The registered size in bytes.
        size: u64,
    },
    /// One catalog response envelope (echoes `requestId`).
    Response(WireResponse),
    /// The event stream batch flushed under the pump's slice budget
    /// (doc 05 §4.4: in-process hosts deliver all events; filtering
    /// is client-side).
    Events {
        /// The events, in engine `seq` order.
        events: Vec<WireEvent>,
    },
    /// One pump slice's report (the re-arm truth: the host keeps
    /// scheduling while `busy`).
    Pumped {
        /// The report.
        report: PumpReport,
    },
    /// The pump slice's present: ONE frame per pump message (the
    /// latest-wins mailbox's drain — the LEAN message; the RGBA
    /// bytes stage RAW in linear memory through the ABI's frame
    /// lane [`crate::abi`], valid until the next dispatch, never
    /// JSON-encoded).
    PresentFrame {
        /// Pixels across.
        width: u32,
        /// Pixels down.
        height: u32,
        /// The frame's pts (ns), when the demuxer carried one.
        pts_ns: Option<i64>,
    },
    /// The render lane's explicit degrade (doc 09 §4.4: audio-only
    /// when `transition.to` is `None` — never a frozen last frame
    /// pretending to play).
    RenderDegraded {
        /// The loss + fallback transition.
        transition: LaneTransition,
    },
    /// The autoplay gate's typed event (card 7.H3, doc 09 §5.3):
    /// the page reported the `AudioContext` suspended without a
    /// user gesture, so audio cannot flow — the presentation parks
    /// as *paused-pending-gesture* (an explicit substate, never a
    /// lie). `[PROPOSAL — event shape: doc 09 §12 OQ-3; the command
    /// layer's `playback.gesture-required` catalog row is the
    /// Lead's additive decision (doc 05 §6). This transport-level
    /// twin carries the same condition without editing the shared
    /// catalog.]`
    GestureRequired,
    /// A control-lane failure (typed, never a crash): a `loadBlob`
    /// registry refusal, an ABI decode failure, a message the
    /// worker could not apply. Command-lane errors ride `Response`
    /// — this is the control lane's twin (doc 05 §5.2's "typed,
    /// not silent", applied to the transport itself).
    ControlError {
        /// The stable dotted code (branchable, like `CommandError`'s).
        code: String,
        /// The human sentence (V7 — render as text, never markup).
        message: String,
    },
    /// The graceful shutdown acknowledgment.
    ShutdownComplete,
}

impl WorkerToHost {
    /// The message's `"type"` wire tag (diagnostics + tests).
    #[must_use]
    pub fn type_tag(&self) -> &'static str {
        match self {
            WorkerToHost::Booted { .. } => "booted",
            WorkerToHost::BlobLoaded { .. } => "blobLoaded",
            WorkerToHost::Response(_) => "response",
            WorkerToHost::Events { .. } => "events",
            WorkerToHost::Pumped { .. } => "pumped",
            WorkerToHost::PresentFrame { .. } => "presentFrame",
            WorkerToHost::RenderDegraded { .. } => "renderDegraded",
            WorkerToHost::GestureRequired => "gestureRequired",
            WorkerToHost::ControlError { .. } => "controlError",
            WorkerToHost::ShutdownComplete => "shutdownComplete",
        }
    }
}

/// One pump slice's observable outcome (doc 09 §3.2: what the host
/// scheduler needs to decide the next arm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PumpReport {
    /// The wire session state after the slice.
    pub state: commands::PlaybackState,
    /// The position in µs after the slice (audio-master truth).
    pub position_us: i64,
    /// Whether the engine wants more pump slices (transient or
    /// playing states with work outstanding) — the host's re-arm
    /// condition.
    pub busy: bool,
    /// The events flushed by this slice (the budgeted count — see
    /// [`crate::host::SliceBudgets`]).
    pub events_flushed: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::envelope::{CommandOutcome, Generation, RequestId, SessionId};
    use commands::{Command, CommandResult, EventKind};
    use testkit::Must;

    /// Every downstream variant round-trips through the JSON wire
    /// form — the contract the JS glue and the wasm ABI share.
    #[test]
    fn downstream_wire_roundtrips() {
        let messages = vec![
            HostToWorker::Boot { renderer: None },
            HostToWorker::Boot {
                renderer: Some(RendererCaps {
                    webgpu_api: true,
                    webgpu_adapter: true,
                    webgl2_context: false,
                }),
            },
            HostToWorker::LoadBlob {
                name: "first-vertical".to_string(),
                bytes: vec![0x52, 0x49, 0x46, 0x46],
            },
            HostToWorker::Command(WireRequest {
                request_id: RequestId(7),
                command: Command::PlaybackPlay,
            }),
            HostToWorker::Pump { ticks: 1 },
            HostToWorker::SurfaceLost {
                backend: RenderBackend::WebGl2,
                reason: crate::render::backend::LossReason::ContextLost,
            },
            HostToWorker::AudioContext { running: false },
            HostToWorker::Shutdown,
        ];
        for message in &messages {
            let json = serde_json::to_string(message).must();
            let back: HostToWorker = serde_json::from_str(&json).must();
            assert_eq!(&back, message, "wire form drifted: {json}");
        }
    }

    /// The legacy boot form — `{"type":"boot"}` with no renderer
    /// facts — still decodes (the `#[serde(default)]` lane: old glue
    /// and the ABI tests boot exactly this shape).
    #[test]
    fn boot_without_renderer_facts_decodes() {
        let message: HostToWorker = serde_json::from_str("{\"type\":\"boot\"}").must();
        assert_eq!(message, HostToWorker::Boot { renderer: None });
    }

    /// Every upstream variant round-trips through the JSON wire form.
    #[test]
    fn upstream_wire_roundtrips() {
        let engine_info = EngineInfo {
            engine_version: "0.1.0".to_string(),
            catalog_version: "1".to_string(),
            protocol_version: "unwired".to_string(),
            host: "web".to_string(),
            features: vec!["first-vertical:avi-mjpeg-pcm".to_string()],
            plugins: Vec::new(),
            decoders: Vec::new(),
            sessions: Vec::new(),
        };
        let messages = vec![
            WorkerToHost::Booted {
                engine_info,
                capabilities: crate::capabilities::capabilities(
                    crate::capabilities::TargetProfile::Native,
                ),
            },
            WorkerToHost::BlobLoaded {
                name: "first-vertical".to_string(),
                size: 425_516,
            },
            WorkerToHost::Response(WireResponse {
                request_id: RequestId(7),
                session: Some(SessionId(1)),
                generation: Generation(2),
                result: Ok(CommandOutcome::Accepted {
                    session: Some(SessionId(1)),
                    generation: Generation(2),
                }),
            }),
            WorkerToHost::Events {
                events: vec![commands::Event {
                    seq: 3,
                    session: Some(SessionId(1)),
                    generation: Some(Generation(2)),
                    origin: None,
                    kind: EventKind::TimingTimeTick {
                        time: commands::catalog::types::Timestamp(123_456),
                        position: 0.25,
                    },
                }],
            },
            WorkerToHost::Pumped {
                report: PumpReport {
                    state: commands::PlaybackState::Playing,
                    position_us: 123_456,
                    busy: true,
                    events_flushed: 2,
                },
            },
            WorkerToHost::PresentFrame {
                width: 64,
                height: 48,
                pts_ns: Some(1_000_000),
            },
            WorkerToHost::RenderDegraded {
                transition: LaneTransition {
                    from: RenderBackend::WebGpu,
                    to: Some(RenderBackend::WebGl2),
                    reason: crate::render::backend::LossReason::DeviceLost,
                },
            },
            WorkerToHost::GestureRequired,
            WorkerToHost::ControlError {
                code: "web.blob_registry".to_string(),
                message: "blob registry full (16 entries)".to_string(),
            },
            WorkerToHost::ShutdownComplete,
        ];
        for message in &messages {
            let json = serde_json::to_string(message).must();
            let back: WorkerToHost = serde_json::from_str(&json).must();
            assert_eq!(&back, message, "wire form drifted: {json}");
        }
    }

    /// The command envelope rides the transport with its catalog
    /// shape intact: the `"type":"command"` tag merges with the
    /// flattened `{requestId, cmd, args}` object (doc 05 §5.1's
    /// example shape, one nesting level shallower than a wrapper).
    #[test]
    fn command_envelope_rides_the_transport_flat() {
        let message = HostToWorker::Command(WireRequest {
            request_id: RequestId(42),
            command: Command::PlaybackPause,
        });
        let json = serde_json::to_string(&message).must();
        assert!(json.contains("\"type\":\"command\""), "{json}");
        assert!(json.contains("\"requestId\":42"), "{json}");
        assert!(json.contains("\"cmd\":\"playback.pause\""), "{json}");
    }

    /// A completed class-S payload round-trips through the response
    /// arm (the `result_wire` adapter under the transport tag).
    #[test]
    fn completed_response_roundtrips() {
        let message = WorkerToHost::Response(WireResponse {
            request_id: RequestId(9),
            session: None,
            generation: Generation(0),
            result: Ok(CommandOutcome::Completed(CommandResult::Unit)),
        });
        let json = serde_json::to_string(&message).must();
        let back: WorkerToHost = serde_json::from_str(&json).must();
        assert_eq!(back, message);
        assert!(json.contains("\"completed\""), "{json}");
    }

    /// The type-tag helpers name the wire tags (diagnostic parity).
    #[test]
    fn type_tags_match_the_wire() {
        assert_eq!(HostToWorker::Pump { ticks: 0 }.type_tag(), "pump");
        assert_eq!(
            WorkerToHost::ShutdownComplete.type_tag(),
            "shutdownComplete"
        );
    }
}
