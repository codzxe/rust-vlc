//! Typed error constructors for the input layer (card 2.A1).
//!
//! The error *taxonomy* is media-types' seven kinds (doc 01 §3.4) — this
//! module never redefines or wraps it. It mints
//! [`EngineError`](media_types::EngineError) values with input-specific
//! context codes (stage `Open`, per doc 03 §4.1(d)) and folds
//! [`std::io::Error`] into the taxonomy with the OS error preserved in
//! the `source` chain (MP §12: the chain carries the OS truth; the kind
//! slot is the closest of the five open-failure reasons).
//!
//! # No secrets
//!
//! Constructors take no path or URI text: `Display` stays structured
//! data and redaction stays `SourceUri::redacted`'s job (MP §12).
