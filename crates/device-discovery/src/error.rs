//! Typed discovery errors: the honest-degradation vocabulary of this
//! crate (card 7.D1; the wave-22 platform crates' `PlatformError`
//! posture, mirrored deliberately so the Lead's L5/L2 seam
//! unification [PROPOSAL] can collapse them mechanically).
//!
//! [`DiscoveryError`] is [`Clone`] on purpose: the typed fallback
//! ([`crate::volume::UnavailableVolumes`]) stores one reason and
//! returns a copy from every method — the reason is data, not a
//! consumed one-shot. I/O failures are captured as
//! [`std::io::ErrorKind`] + rendered detail (io::Error itself is not
//! Clone; the kind is the typed half, the detail the diagnostic).

use std::fmt;

/// Why a discovery call failed, typed — never a bare string guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// The lane cannot work here at all. The reason is typed; see
    /// [`UnavailableReason`].
    Unavailable {
        /// The typed cause.
        reason: UnavailableReason,
    },
    /// A source that should answer could not (the watcher's poll
    /// failed; the failure is per-pass, not a capability verdict).
    EnumerationFailed {
        /// What went wrong, rendered at the failure site.
        detail: String,
    },
    /// A network peer answered with bytes that do not parse as the
    /// protocol requires (SSDP responses; dirty-world tolerant —
    /// the searcher SKIPS these and counts them, this error is
    /// the parse-level vocabulary).
    MalformedResponse {
        /// The protocol that rejected the bytes.
        protocol: &'static str,
        /// What was wrong with them.
        detail: String,
    },
    /// A bound was exceeded: which limit, its size, the attempt
    /// (engineering directive D-3 — bounded memory everywhere).
    CapacityExceeded {
        /// What was bounded.
        what: &'static str,
        /// The bound.
        limit: usize,
        /// The attempt that crossed it.
        attempted: usize,
    },
    /// An I/O failure with its typed kind and rendered detail.
    Io {
        /// Where the I/O happened.
        context: &'static str,
        /// The typed error kind.
        kind: std::io::ErrorKind,
        /// The rendered original (diagnostic only).
        detail: String,
    },
    /// A bounded wait elapsed before the expected answer arrived.
    TimedOut {
        /// What was being waited for.
        what: &'static str,
    },
}

/// Why a capability is unavailable — the typed core of honest
/// degradation (mirrors the wave-22 `UnavailableReason` shape; this
/// copy adds the web-target verdict doc 09 §10 mandates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableReason {
    /// Not implemented in this card — an honest gap with a recorded
    /// follow-up owner, never a silent stub that pretends.
    NotYetImplemented {
        /// What is missing and where it is tracked.
        detail: &'static str,
    },
    /// The target platform cannot ever provide this (browsers expose
    /// no UDP API — doc 09 §10 blocked-capability policy).
    BlockedOnWeb {
        /// What is blocked and why.
        detail: &'static str,
    },
    /// A runtime dependency the lane needs is absent (library,
    /// service, socket API).
    DependencyMissing {
        /// The dependency's name.
        what: &'static str,
        /// The loader/probe diagnosis.
        detail: String,
    },
}

impl UnavailableReason {
    /// Collapse a probe error into the reason class it proves.
    #[must_use = "pure conversion"]
    pub fn from_error(err: &DiscoveryError) -> UnavailableReason {
        match err {
            DiscoveryError::Unavailable { reason } => reason.clone(),
            DiscoveryError::Io { context, .. } => UnavailableReason::DependencyMissing {
                what: "host I/O",
                detail: format!("{context}: the socket/filesystem API refused the probe"),
            },
            other => UnavailableReason::DependencyMissing {
                what: "discovery integration",
                detail: other.to_string(),
            },
        }
    }
}

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnavailableReason::NotYetImplemented { detail } => write!(f, "{detail}"),
            UnavailableReason::BlockedOnWeb { detail } => {
                write!(f, "blocked on web targets: {detail}")
            }
            UnavailableReason::DependencyMissing { what, detail } => {
                write!(f, "dependency missing ({what}): {detail}")
            }
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoveryError::Unavailable { reason } => write!(f, "discovery unavailable: {reason}"),
            DiscoveryError::EnumerationFailed { detail } => {
                write!(f, "enumeration pass failed: {detail}")
            }
            DiscoveryError::MalformedResponse { protocol, detail } => {
                write!(f, "malformed {protocol} response: {detail}")
            }
            DiscoveryError::CapacityExceeded {
                what,
                limit,
                attempted,
            } => write!(f, "{what} bound exceeded: {attempted} > {limit}"),
            DiscoveryError::Io {
                context,
                kind,
                detail,
            } => {
                write!(f, "i/o failure at {context}: {kind:?} ({detail})")
            }
            DiscoveryError::TimedOut { what } => write!(f, "timed out waiting for {what}"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

impl DiscoveryError {
    /// The stable dotted context code for the engine-boundary
    /// translation (doc 01 §3.4 — codes are additive-only surface).
    #[must_use = "pure accessor"]
    pub fn context_code(&self) -> &'static str {
        match self {
            DiscoveryError::Unavailable { .. } => "discovery.unavailable",
            DiscoveryError::EnumerationFailed { .. } => "discovery.enumeration-failed",
            DiscoveryError::MalformedResponse { .. } => "discovery.malformed-response",
            DiscoveryError::CapacityExceeded { .. } => "discovery.capacity-exceeded",
            DiscoveryError::Io { .. } => "discovery.io",
            DiscoveryError::TimedOut { .. } => "discovery.timed-out",
        }
    }

    /// Translate into the engine error currency (stage
    /// [`media_types::PipelineStage::Discovery`]) — the boundary host
    /// relays crate errors into engine error reporting through this.
    ///
    /// Taxonomy honesty: the MP §8 kinds are a fixed seven; none is
    /// "generic OS-service failure". The precise typed cause always
    /// rides the source chain (`self`, verbatim) — the kind below is
    /// the nearest fixed class, chosen the same way plugin-api's
    /// discovery claim-mismatch chose `Corrupt` for a contract
    /// violation.
    #[must_use = "pure conversion"]
    pub fn into_engine_error(self) -> media_types::EngineError {
        use media_types::error::{CorruptDetail, ErrorContext, LimitKind};
        use media_types::{ErrorKind, PipelineStage};
        let kind = match &self {
            DiscoveryError::Unavailable { .. } | DiscoveryError::EnumerationFailed { .. } => {
                // "The pass could not be served here" — the typed
                // cause (and, for enumeration, the per-pass detail)
                // rides the source chain.
                ErrorKind::Unsupported { format: None }
            }
            DiscoveryError::MalformedResponse { .. } => ErrorKind::Corrupt {
                detail: CorruptDetail::InvalidStructure { offset: None },
            },
            DiscoveryError::Io { .. } => ErrorKind::Network {
                cause: media_types::error::NetworkCause::ConnectionLost,
            },
            DiscoveryError::TimedOut { .. } => ErrorKind::Network {
                cause: media_types::error::NetworkCause::Timeout,
            },
            DiscoveryError::CapacityExceeded {
                limit, attempted, ..
            } => ErrorKind::ResourceLimit {
                limit: LimitKind::QueueSize,
                budget: u64::try_from(*limit).unwrap_or(u64::MAX),
                attempted: u64::try_from(*attempted).unwrap_or(u64::MAX),
            },
        };
        media_types::EngineError::new(
            PipelineStage::Discovery,
            kind,
            ErrorContext::new(self.context_code()),
        )
        .with_source(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(err: &DiscoveryError) -> String {
        err.to_string()
    }

    #[test]
    fn unavailable_displays_its_reason() {
        let err = DiscoveryError::Unavailable {
            reason: UnavailableReason::NotYetImplemented {
                detail: "the platform cards own the follow-up",
            },
        };
        let text = render(&err);
        assert!(
            text.contains("the platform cards own the follow-up"),
            "{text}"
        );
    }

    #[test]
    fn blocked_on_web_reason_displays_the_policy() {
        let reason = UnavailableReason::BlockedOnWeb {
            detail: "browsers expose no UDP API (doc 09 §10)",
        };
        let text = reason.to_string();
        assert!(text.starts_with("blocked on web targets"), "{text}");
    }

    #[test]
    fn capacity_error_names_the_bound() {
        let err = DiscoveryError::CapacityExceeded {
            what: "volume list",
            limit: 1024,
            attempted: 2000,
        };
        let text = render(&err);
        assert!(text.contains("1024") && text.contains("2000"), "{text}");
    }

    #[test]
    fn engine_error_carries_stage_context_and_source() {
        let err = DiscoveryError::TimedOut {
            what: "ssdp responses",
        }
        .into_engine_error();
        assert_eq!(err.stage, media_types::PipelineStage::Discovery);
        assert_eq!(err.context.code, "discovery.timed-out");
        let source = match std::error::Error::source(&err) {
            Some(s) => s.to_string(),
            None => unreachable!("the typed cause rides the source chain"),
        };
        assert!(source.contains("ssdp responses"), "{source}");
    }

    #[test]
    fn reason_from_probe_error_collapses_io() {
        let err = DiscoveryError::Io {
            context: "ssdp socket bind",
            kind: std::io::ErrorKind::PermissionDenied,
            detail: "EPERM".to_string(),
        };
        match UnavailableReason::from_error(&err) {
            UnavailableReason::DependencyMissing { what, .. } => assert_eq!(what, "host I/O"),
            other => unreachable!("io probes collapse to DependencyMissing, got {other:?}"),
        }
    }
}
