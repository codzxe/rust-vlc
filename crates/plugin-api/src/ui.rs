//! The ui-panel host contract (doc 06 §1.2 kind 15): a UI surface
//! talking **only to the command layer** (doc 01 §2.8).
//!
//! **Extension-only in v1** (§1.2 row 15): no builtin is ever a
//! ui-panel. The guest-side wire protocol lives in
//! `plugin-host-wasm` (L6, doc 06 §5.5); this host trait is the seam
//! the host uses to represent a registered panel to the layout system
//! [PROPOSAL: placement — doc 06 §1.2 flags it].
//!
//! The cardinal rule (MP §12, doc 06 §5.3): a panel renders through
//! the layout system and issues **commands**; it never touches the
//! engine directly, never gets a process handle, never gets a socket.
//! Its `ui-surface` permission is the gate, and escalation attempts
//! trap the instance (§5.3).

use media_types::{EngineError, PipelineStage};

/// Host-side representation of one registered panel (doc 06 §1.2
/// row 15): what it is and where it docks, never how it renders —
/// the panel's own view tree crosses the wasm boundary as data
/// through the command layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelDescriptor {
    /// Stable panel id (namespaced under the plugin id by the host,
    /// e.g. `ext.acme.lyrics-panel:main`).
    pub panel_id: String,
    /// Human-readable panel title (the panel ships its own localized
    /// strings; the host stores the message key, never a translation
    /// — localization is doc 08's layer, not the plugin's).
    pub title_key: String,
    /// Where the panel docks (the §2.2 `surfaces` capability claim —
    /// e.g. `side-panel`).
    pub surface: String,
}

/// The ui-panel host contract (doc 06 §1.2 row 15, §5.3).
///
/// The host (not the panel) owns the surface lifecycle: it registers
/// the descriptor with the layout system, routes command envelopes
/// from the panel to the command layer, and delivers events back.
/// Implementations of this trait are host-side shims.
///
/// # Calling context
///
/// [`Send`] + [`Sync`]: app-singleton surface state the host shares
/// across the UI and command threads. All methods are bounded and
/// non-blocking; the panel's own event loop runs inside the sandbox
/// (§5.8).
pub trait UiPanel: Send + Sync {
    /// The panel's registration data (surfaced to the layout system
    /// under the `ui-surface` permission).
    fn descriptor(&self) -> PanelDescriptor;

    /// Deliver one command-layer event to the panel (serialized by
    /// the host; the panel's guest protocol defines the encoding).
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Plugin`]) when the
    /// panel refuses or the guest protocol fails; the host then
    /// applies the §5.6 isolation policy — playback survives.
    fn on_event(&self, event: &[u8]) -> Result<(), EngineError>;

    /// Tear the panel's surface down (user removed it, plugin
    /// disabled, or crash isolation). Idempotent.
    ///
    /// # Errors
    /// [`EngineError`] if the layout system rejects the removal —
    /// never for a second call.
    fn teardown(&self) -> Result<(), EngineError>;
}

/// Typed panel error: the guest protocol violated the surface
/// contract (e.g. attempted an engine call outside the command
/// envelope). Logged as a security event per doc 06 §5.3.
pub fn surface_contract_violation(panel: &str, what: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Plugin,
        media_types::ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::PermissionDenied,
        },
        media_types::error::ErrorContext::new("plugin.ui.surface-contract-violation"),
    )
    .with_source(Violation {
        panel: panel.to_owned(),
        what: what.to_owned(),
    })
}

#[derive(Debug)]
struct Violation {
    panel: String,
    what: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "panel `{}` violated the surface contract: {}",
            self.panel, self.what
        )
    }
}
impl std::error::Error for Violation {}

#[cfg(test)]
mod tests {
    use super::*;

    struct LyricsPanel {
        torn_down: std::sync::atomic::AtomicBool,
    }

    impl UiPanel for LyricsPanel {
        fn descriptor(&self) -> PanelDescriptor {
            PanelDescriptor {
                panel_id: "ext.acme.lyrics-panel:main".to_string(),
                title_key: "panel.lyrics.title".to_string(),
                surface: "side-panel".to_string(),
            }
        }
        fn on_event(&self, event: &[u8]) -> Result<(), EngineError> {
            // The guest protocol: events are length-prefixed; a bad
            // length is a contract violation.
            if event.first() == Some(&0) {
                return Err(surface_contract_violation(
                    "ext.acme.lyrics-panel:main",
                    "zero-length event",
                ));
            }
            Ok(())
        }
        fn teardown(&self) -> Result<(), EngineError> {
            self.torn_down
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn descriptor_is_data_not_ui_copy() {
        let panel = LyricsPanel {
            torn_down: std::sync::atomic::AtomicBool::new(false),
        };
        let d = panel.descriptor();
        assert_eq!(
            d.surface, "side-panel",
            "matches the §2.3 (d) manifest claim"
        );
        assert!(
            d.title_key.starts_with("panel."),
            "message key, not literal copy"
        );
    }

    #[test]
    fn violations_are_typed_and_teardown_idempotent() {
        let panel = LyricsPanel {
            torn_down: std::sync::atomic::AtomicBool::new(false),
        };
        let err = match panel.on_event(&[0]) {
            Ok(()) => unreachable!("bad event must violate"),
            Err(e) => e,
        };
        assert_eq!(err.stage, PipelineStage::Plugin);
        let src = match std::error::Error::source(&err) {
            Some(s) => s.to_string(),
            None => unreachable!("source carries the violation"),
        };
        assert!(src.contains("surface contract"));
        // Idempotent teardown.
        for _ in 0..2 {
            match panel.teardown() {
                Ok(()) => {}
                Err(e) => unreachable!("{e}"),
            }
        }
        assert!(panel.torn_down.load(std::sync::atomic::Ordering::SeqCst));
    }
}
