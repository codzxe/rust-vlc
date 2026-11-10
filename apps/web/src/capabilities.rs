//! The honest capability report (card 7.H1; doc 09 §3.3/§4.4/§10).
//!
//! The web host reports what it can actually do *right now, on this
//! build* — the same truth rule as hardware decode (MP §2, doc 05
//! `system.get-engine-info`). Nothing here is aspirational: every
//! field names the build's real state, and every gap is a TYPED
//! [`BlockedCapability`] row (doc 09 §10's table, rendered as data —
//! "typed, not silent"), never a crash and never a missing entry.
//!
//! # Threading (doc 09 §3)
//!
//! v1 is the engine-in-a-Worker, cooperatively scheduled model
//! (§3.2): every engine role runs on one thread behind the pump, so
//! the snapshot always says [`Threading::Single`] on this card. The
//! pooled(n) mode is §3.3's v2 (SharedArrayBuffer + COOP/COEP) — it
//! does not exist yet and is not claimed.
//!
//! # The decoder split (this card's honest limit)
//!
//! The wasm32 build ships the engine core WITHOUT the builtin
//! decoders: the `codecs` crate's external decode engines do not
//! build for `wasm32-unknown-unknown` (rav1d 1.1.0 fails to compile
//! for the target — measured, this card's worklog), so the target-
//! gated dependency leaves the wasm module decoder-less and the
//! snapshot says so. Native builds carry the real MJPEG + PCM
//! decoders (the first vertical). The profile is chosen by
//! `#[cfg]`, and both profiles' DATA is unit-tested below — the
//! browser runtime itself is `[VERIFY]` (no browser in this
//! sandbox).

use serde::{Deserialize, Serialize};

/// The threading mode the engine actually runs in (doc 09 §3.3's
/// negotiation outcome — reported, never assumed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Threading {
    /// v1: all engine roles on the Worker thread, cooperatively
    /// scheduled by the pump (doc 09 §3.2).
    Single,
}

impl Threading {
    /// The snapshot's wire string (doc 09 §3.3's vocabulary:
    /// `single` | `pooled(n)`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Threading::Single => "single",
        }
    }
}

/// One honestly-missing capability (doc 09 §10's table as data — the
/// feature-matrix vocabulary of doc 02 §4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedCapability {
    /// The capability's stable name (feature-matrix row id shape).
    pub capability: String,
    /// The vocabulary status.
    pub status: CapabilityStatus,
    /// Why — citing doc 09's section, the same text the UI shows.
    pub reason: String,
}

/// The status vocabulary (doc 02 §4.3; only the rows this host uses).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityStatus {
    /// No browser substrate exists; refused forever on this host.
    Unsupported,
    /// A substrate exists but the engine lane has not landed.
    Blocked,
    /// Planned work (named card), not yet started.
    NotStarted,
}

impl CapabilityStatus {
    /// The wire string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityStatus::Unsupported => "unsupported",
            CapabilityStatus::Blocked => "blocked",
            CapabilityStatus::NotStarted => "not-started",
        }
    }
}

/// The boot-time capability snapshot (doc 09 §7: the handshake reply
/// carries `EngineInfo` + this).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    /// `single` on this card (doc 09 §3.2's truth rule).
    pub threading: Threading,
    /// The video output lane — none yet: the canvas surface is card
    /// 7.H2 (v1's video path presents to the headless counting
    /// presenter).
    pub renderer: String,
    /// The audio output lane — the null device: the AudioWorklet
    /// sink is card 7.H3.
    pub audio_output: String,
    /// The registry descriptors this build can actually decode
    /// (empty on wasm32 — see the module docs).
    pub decoders: Vec<String>,
    /// The typed gap table (doc 09 §10).
    pub blocked: Vec<BlockedCapability>,
}

impl HostCapabilities {
    /// The blocked-row lookup by capability name (the open path's
    /// typed refusals cite the same rows — one source of truth).
    #[must_use]
    pub fn blocked_row(&self, capability: &str) -> Option<&BlockedCapability> {
        self.blocked.iter().find(|row| row.capability == capability)
    }
}

/// Which build the snapshot describes. The cfg selects the profile;
/// BOTH profiles' data is covered by unit tests (the wasm profile's
/// runtime behavior in a real browser is `[VERIFY]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProfile {
    /// A native host build: the real builtin decoders are linked.
    Native,
    /// The `wasm32-unknown-unknown` module: engine core, no decoders
    /// (the codecs crate does not build for the target).
    Wasm32,
}

/// The profile this build actually is.
#[must_use]
pub fn build_profile() -> TargetProfile {
    if cfg!(target_arch = "wasm32") {
        TargetProfile::Wasm32
    } else {
        TargetProfile::Native
    }
}

/// The registry descriptors the linked decoders answer for (the
/// provider's dispatch table, kept in one place so the snapshot and
/// the provider cannot drift).
#[must_use]
pub fn decoder_descriptors(profile: TargetProfile) -> Vec<String> {
    match profile {
        TargetProfile::Native => vec!["mjpeg".to_string(), "pcm_s16le".to_string()],
        TargetProfile::Wasm32 => Vec::new(),
    }
}

/// Build the honest snapshot for `profile` (doc 09 §3.3/§4.4/§10).
#[must_use]
pub fn capabilities(profile: TargetProfile) -> HostCapabilities {
    let decoders = decoder_descriptors(profile);
    let mut blocked = vec![
        BlockedCapability {
            capability: "rtsp-rtp-udp-access".to_string(),
            status: CapabilityStatus::Unsupported,
            reason: "no raw sockets in browsers (doc 09 §6.3/§10) — the typed \
                     `unsupported-for-host` refusal, never a crash"
                .to_string(),
        },
        BlockedCapability {
            capability: "optical-discs".to_string(),
            status: CapabilityStatus::Unsupported,
            reason: "no disc APIs in browsers (doc 09 §10)".to_string(),
        },
        BlockedCapability {
            capability: "dvb-capture-hardware".to_string(),
            status: CapabilityStatus::Blocked,
            reason: "no hardware access in browsers (doc 09 §10); the \
                     getUserMedia substitute is Phase 7+"
                .to_string(),
        },
        BlockedCapability {
            capability: "capture-getusermedia".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "browser substitute exists, MediaStream source adapter is \
                     Phase 7+ (doc 09 §10)"
                .to_string(),
        },
        BlockedCapability {
            capability: "hardware-decode".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "WebCodecs ADR pending (doc 09 §10); until it passes, \
                     software decode only"
                .to_string(),
        },
        BlockedCapability {
            capability: "local-filesystem".to_string(),
            status: CapabilityStatus::Unsupported,
            reason: "by design on web (doc 09 §9.2/§10) — user-selected \
                     blobs travel the no-upload path instead"
                .to_string(),
        },
        BlockedCapability {
            capability: "http-streaming".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "fetch + Range seekability detection is card 7.H4 \
                     (doc 09 §6.1)"
                .to_string(),
        },
        BlockedCapability {
            capability: "canvas-render-surface".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "WebGPU/WebGL2 present is card 7.H2 (doc 09 §4); v1 \
                     presents to the headless counting presenter"
                .to_string(),
        },
        BlockedCapability {
            capability: "audioworklet-output".to_string(),
            status: CapabilityStatus::NotStarted,
            reason: "the AudioWorklet sink + sample-counter clock is card \
                     7.H3 (doc 09 §5); v1 runs the null device"
                .to_string(),
        },
    ];
    if profile == TargetProfile::Wasm32 {
        blocked.push(BlockedCapability {
            capability: "builtin-software-decoders".to_string(),
            status: CapabilityStatus::Blocked,
            reason: "the codecs crate's external engines do not build for \
                     wasm32-unknown-unknown (rav1d); the wasm module ships \
                     the engine core decoder-less — session decode refuses \
                     typed until the crate splits (Lead decision)"
                .to_string(),
        });
    }
    HostCapabilities {
        threading: Threading::Single,
        renderer: "none (canvas surface is card 7.H2)".to_string(),
        audio_output: "null device (AudioWorklet sink is card 7.H3)".to_string(),
        decoders,
        blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// Threading is ALWAYS single on this card — the pooled(n) mode
    /// is doc 09 §3.3's v2 and must not be claimed (the truth rule).
    #[test]
    fn threading_is_single_on_both_profiles() {
        for profile in [TargetProfile::Native, TargetProfile::Wasm32] {
            let caps = capabilities(profile);
            assert_eq!(caps.threading, Threading::Single);
            assert_eq!(caps.threading.as_str(), "single");
        }
    }

    /// The native profile carries the first vertical's real decoders;
    /// the wasm32 profile honestly ships none (the codecs/target
    /// split — the snapshot must not claim decoders the module
    /// cannot construct).
    #[test]
    fn decoder_claims_match_the_profile() {
        let native = capabilities(TargetProfile::Native);
        assert_eq!(native.decoders, vec!["mjpeg", "pcm_s16le"]);
        let wasm = capabilities(TargetProfile::Wasm32);
        assert!(wasm.decoders.is_empty());
        // And the wasm gap is a TYPED blocked row, not silence.
        let row = wasm.blocked_row("builtin-software-decoders");
        assert_eq!(row.map(|row| row.status), Some(CapabilityStatus::Blocked));
        assert!(
            row.is_some_and(|row| row.reason.contains("rav1d")),
            "the reason names the actual blocker"
        );
        assert!(native.blocked_row("builtin-software-decoders").is_none());
    }

    /// Doc 09 §10's table is present as data on both profiles: every
    /// row names its status and a reason that cites the doc.
    #[test]
    fn doc_09_table_rows_are_typed_data() {
        for profile in [TargetProfile::Native, TargetProfile::Wasm32] {
            let caps = capabilities(profile);
            for capability in [
                "rtsp-rtp-udp-access",
                "optical-discs",
                "dvb-capture-hardware",
                "http-streaming",
            ] {
                assert!(
                    caps.blocked_row(capability).is_some(),
                    "{capability} missing on {profile:?}"
                );
            }
            for row in &caps.blocked {
                assert!(!row.reason.is_empty());
                assert!(!row.capability.is_empty());
            }
        }
    }

    /// The open path's typed refusals cite the same rows — one
    /// source of truth (the lookup answers for both directions).
    #[test]
    fn blocked_row_lookup_answers_by_name() {
        let caps = capabilities(TargetProfile::Native);
        let row = caps.blocked_row("rtsp-rtp-udp-access");
        assert_eq!(
            row.map(|row| row.status),
            Some(CapabilityStatus::Unsupported)
        );
        assert!(caps.blocked_row("no-such-capability").is_none());
    }

    /// The snapshot serializes (the handshake reply's wire form).
    #[test]
    fn snapshot_serializes() {
        let caps = capabilities(TargetProfile::Wasm32);
        let json = serde_json::to_string(&caps).must();
        assert!(json.contains("\"threading\":\"single\""));
        assert!(json.contains("builtin-software-decoders"));
    }
}
