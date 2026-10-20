//! The Lumen command layer (doc 05; cards 1.D3 + 1.D4).
//!
//! **One command layer** (doc 05 §1.1): the desktop UI (Tauri IPC), the
//! web UI (wasm binding), the CLI, the contract tests and the control
//! server all consume the same envelope, the same request/response/event
//! semantics, and — with card 1.D4 — the same catalog. Defined once
//! here, re-exported (never re-implemented) by every host.
//!
//! # Card split (Lead decision, recorded in the worklog)
//!
//! 1.D3 (this crate's first landing) delivers the payload-independent
//! contract of doc 05 §1/§4:
//!
//! - [`error`]     — the §2.13 common error set (`Stage`, `ErrorCode`)
//!   with the §1.2 V8 redaction rule enforced at construction.
//! - [`envelope`]  — the §1.6 envelope types (`RequestId`, `SessionId`,
//!   `Generation`, request/response/outcome), **generic over the catalog
//!   payloads** (`CommandRequest<C>`, `CommandResponse<R>`,
//!   `Event<E>`). Card 1.D4 instantiates them with the concrete
//!   catalog enums and adds the serde wire forms (§5.1: camelCase,
//!   internally-tagged, kebab-case variants).
//! - [`correlation`] — the §4 machinery: request-id allocation,
//!   generation staleness (the §4.2 host rule, unit-tested as the card
//!   AC demands), `seq` ordering, and the delivery-log validator that
//!   checks the §4.3 invariants on an observed stream.
//!
//! 1.D4 adds the catalog itself — doc 05 §2–§3 as types AND as data,
//! plus the §5.1 schema-first codegen:
//!
//! - [`catalog`]   — the ~61-command / 23-event catalog: stable ids
//!   (§2.1/§3.2), payload wire types (§2.2), [`catalog::Command`]/
//!   [`catalog::CommandResult`]/[`catalog::EventKind`], the
//!   `catalog_data()` metadata table (class, target, gen-bumping),
//!   and the `catalog-gen` generator behind `cargo xtask codegen`
//!   (committed artifacts: `schemas/commands.schema.json`,
//!   `ui/src/generated/commands.ts`, the checksum file).
//!
//! # The truthful-UI rule (doc 05 §1.4 — hard contract)
//!
//! `Accepted ≠ Completed`. Class-A commands answer
//! [`CommandOutcome::Accepted`] and complete via events; class-S commands
//! answer [`CommandOutcome::Completed`]. The correlation module is where
//! the host-side half of rules C1–C8 lives (the staleness filter and the
//! ordering checks); the engine-side half is the playback engine's
//! conformance suite (Phase 2+).

pub mod catalog;
pub mod correlation;
pub mod envelope;
pub mod error;

pub use catalog::{
    Command, CommandId, CommandResult, EventId, EventKind, PlaybackState, WireEvent, WireRequest,
    WireResponse,
};
pub use correlation::{DeliveryLog, RequestIdAllocator, StalenessFilter};
pub use envelope::{
    CommandOutcome, CommandRequest, CommandResponse, Event, Generation, RequestId, SessionId,
};
pub use error::{CommandError, ErrorCode, Stage};

/// Prelude: the full 1.D3 + 1.D4 surface in one `use`.
pub mod prelude {
    pub use crate::catalog::{
        Command, CommandId, CommandResult, EventId, EventKind, PlaybackState,
    };
    pub use crate::correlation::{DeliveryLog, RequestIdAllocator, StalenessFilter};
    pub use crate::envelope::{
        CommandOutcome, CommandRequest, CommandResponse, Event, Generation, RequestId, SessionId,
    };
    pub use crate::error::{CommandError, ErrorCode, Stage};
}
