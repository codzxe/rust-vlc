//! The command/event catalog v1 (doc 05 §2–§3 — card 1.D4).
//!
//! # Layout
//!
//! - [`ids`]     — the §2.1/§3.2 stable kebab-case ids and their
//!   mechanical Rust variants (`CommandId`, `EventId`, `FromStr`).
//! - [`types`]   — the §2.2 common wire types + the payload shapes the
//!   §2/§3 tables define (`SeekTarget`, `TrackInfo`, `MediaInfo`, …).
//! - [`command`] — [`Command`] (one variant per §2.1 id, args verbatim
//!   from the §2 tables) and [`CommandResult`] (the class-S completed
//!   payloads).
//! - [`event`]   — [`EventKind`] (one variant per §3.2 row).
//! - [`spec`]    — the catalog as DATA (`catalog_data()`,
//!   `event_catalog_data()`): class (§1.5), target (§1.1.3),
//!   gen-bumping (§4.2), shape descriptors, and the FNV-1a checksum.
//! - [`codegen`] — the §5.1 schema-first codegen (JSON Schema, TS
//!   types, checksum file) the `catalog-gen` bin and the golden tests
//!   share.
//!
//! # The contract this module IS
//!
//! One command layer (§1.1): every host — Tauri shell, web page, CLI,
//! contract tests, control server — consumes THIS catalog through
//! [`crate::envelope::CommandRequest<Command>`],
//! [`crate::envelope::CommandResponse<CommandResult>`] and
//! [`crate::envelope::Event<EventKind>`]. Ids are stable forever
//! within major 1 (§6); adding an entry is additive (§6.2), and the
//! committed schema/TS/checksum artifacts regenerate via
//! `cargo xtask codegen` (CI fails on drift).

pub mod codegen;
pub mod command;
pub mod event;
pub mod ids;
pub mod spec;
pub mod types;

pub use codegen::CatalogDocument;
pub use codegen::WireEvent;
pub use codegen::WireRequest;
pub use codegen::WireResponse;
pub use command::Command;
pub use command::CommandListing;
pub use command::CommandResult;
pub use ids::CommandId;
pub use ids::EventId;
pub use ids::UnknownId;
pub use spec::catalog_data;
pub use spec::event_catalog_data;
pub use spec::CompletionClass;
pub use spec::CommandSpec;
pub use spec::CommandTarget;
pub use spec::EventSpec;
pub use spec::PayloadDesc;
pub use types::PlaybackState;
