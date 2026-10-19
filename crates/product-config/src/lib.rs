//! # product-config — the config-driven product model
//!
//! Typed, validated model of `product.toml` against `product.schema.json`,
//! with actionable diagnostics and layered precedence resolution
//! (doc 07; registry entry doc 01 §2.3: layer L1, no internal deps).
//!
//! ## Pipeline (doc 07 §3.4)
//!
//! 1. **Parse** — TOML syntax; failures carry the parser's line.
//! 2. **Structural** — data-driven validation against the schema file
//!    (required keys, types, ranges, enums, patterns, unknown-key typo
//!    detection with did-you-mean hints); *every* violation is reported.
//! 3. **Typed** — deserialize into [`ProductConfig`].
//! 4. **Semantic** — [`semantic::validate_semantics`]: cross-field rules,
//!    injected enums from directory listings, catalog-key existence,
//!    feature-registry intersection (F1/F2), brand-asset existence + magic
//!    bytes. Inputs that live in files owned by other cards are explicit
//!    ([`semantic::SemanticInputs`]); the `cargo xtask config-validator`
//!    wiring is card 1.E2.
//!
//! ## Precedence (doc 07 §4)
//!
//! v1 implements *product defaults + user*: load the base with [`load_str`],
//! the user layer with [`load_partial_str`], then [`merge::resolve_layers`]
//! (or the [`merge::layer`] convenience). The six-layer list is the
//! extension point for the Phase-2+ profile/admin/flags/session cards.
//!
//! ## Diagnostics (doc 07 §3.5)
//!
//! Every failure carries file + line + fully-qualified key path + the rule
//! as a constraint + expected vs found + a fix hint; near-miss keys get a
//! "did you mean" hint. Invalid critical config is never silently replaced
//! (MP §5).
//!
//! ## What this crate never does
//!
//! No playback logic, no silent fallback to defaults on invalid critical
//! config, no hardcoded user-visible strings (doc 01 §2.3 (d)); no
//! `unsafe`/`unwrap`/`expect`/`panic` (workspace lints).

pub mod diagnostic;
pub mod lines;
pub mod load;
pub mod merge;
pub mod model;
pub mod partial;
pub mod patterns;
pub mod schema;
pub mod semantic;
pub mod suggest;

pub use diagnostic::{Diagnostic, Severity, Stage, render_all};
pub use load::{load_file, load_partial_file, load_partial_str, load_str};
pub use merge::{Layer, LayerSource, layer, resolve_layers};
pub use model::ProductConfig;
pub use partial::PartialProduct;
pub use schema::{RequiredMode, Schema, SchemaError};
pub use semantic::{FeatureRegistry, SemanticInputs, validate_semantics};
