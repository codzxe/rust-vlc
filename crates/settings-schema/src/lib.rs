//! # settings-schema — one settings schema, many generators
//!
//! The single source of truth for every Lumen setting (doc 07 §5; registry
//! entry doc 01 §2.3: layer L1, no internal deps). The authored artifact is
//! `schemas/settings.toml` (TOML, human-written, doc 07 §5.2 entry format —
//! ten mandated fields per entry); everything else is generated from it and
//! committed for reviewability:
//!
//! * **G1** [`emit::generate`] with [`emit::GenTarget::TsTypes`] →
//!   `schemas/generated/settings.d.ts` (the TS *type surface*; the
//!   value-bearing table for the React screens lands with card 4.E1).
//! * **G2** [`emit::GenTarget::CliHelp`] → `schemas/generated/settings-help.txt`
//!   (CLI `--help` golden text).
//! * **G3** [`emit::GenTarget::Docs`] → `schemas/generated/settings-docs.md`
//!   (settings reference page).
//! * **G4** [`runtime`] — the schema embedded via `include_str!`;
//!   [`runtime::validate_value`] is the `config.set-setting` seam
//!   (type/range/enum, `invalid-argument`, never silent coercion) and
//!   [`runtime::schema_meta`] backs `config.get-setting`'s `schema_meta`
//!   field.
//!
//! ## Single-definition rules (doc 07 §5.2)
//!
//! [`validate::validate`] enforces D1–D6: defaults stated once (D1),
//! globally unique keys (D2), CLI flags unique and disjoint from reserved
//! names (D3), description msgids 1:1 with entries (D4), placement orders
//! unique per `(screen, section)` with known screens (D5), one entry per
//! product-decision slot (D6). Failures are actionable diagnostics with
//! file + line + entry path + rule + fix hint.
//!
//! ## The product.toml seam (doc 01 §2.3)
//!
//! D1 joins the schema to `product.toml` (`default_ref` targets must exist
//! and type-check; same-area literal duplication heuristic). The join is
//! **data-level, not a crate edge**: both crates have no internal deps, and
//! the cross-check consumes the parsed `product.toml` as a `toml::Value`
//! tree. The `cargo xtask schema-gen` bridge and the 1.E2
//! `config-validator` wiring feed both files; a future direct call from
//! `product-config` at load time would pass its own parsed tree the same
//! way `[PROPOSAL — Lead decision, doc 01 §6.3]`.
//!
//! ## What this crate never does
//!
//! No UI construction, no option duplication, no defaults that contradict
//! `product.toml` (the validator's job to catch), no hardcoded user-visible
//! strings (doc 01 §2.3 (d)); no `unsafe`/`unwrap`/`expect`/`panic`
//! (workspace lints).

pub mod diagnostic;
pub mod emit;
pub mod model;
pub mod parse;
pub mod runtime;
pub mod validate;

pub use diagnostic::{SchemaDiagnostic, Severity, render_all};
pub use emit::{GenError, GenTarget, generate, generate_str};
pub use model::{
    ArrayPolicy, Control, DefaultValue, Domain, ListElement, Scope, SettingDef, SettingType,
    SettingsSchema, UiPlacement, ValueRange, check_value,
};
pub use parse::parse_str;
pub use runtime::{SchemaError, ValueError, embedded, schema_meta, validate_value};
pub use validate::validate;

/// Load errors for the file-level helpers.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Reading a file failed.
    #[error("reading {path}: {source}")]
    Io {
        /// The file being read.
        path: String,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The content failed parse and/or validation (all diagnostics).
    #[error("invalid settings schema:\n{0}")]
    Invalid(String),
}

/// Parse and fully validate schema source text (structural + D-rules,
/// joining `product.toml` when provided).
pub fn parse_and_validate_str(
    src: &str,
    product: Option<&toml::Value>,
) -> Result<SettingsSchema, LoadError> {
    let schema = parse_str(src).map_err(|diags| LoadError::Invalid(render_all(&diags)))?;
    let diags = validate(&schema, product);
    if diags.is_empty() {
        Ok(schema)
    } else {
        Err(LoadError::Invalid(render_all(&diags)))
    }
}

/// Load and fully validate a schema file (and optionally `product.toml`
/// from another path) — the generator binary's loader.
pub fn load(
    schema_path: &std::path::Path,
    product_path: Option<&std::path::Path>,
) -> Result<SettingsSchema, LoadError> {
    let schema_src = std::fs::read_to_string(schema_path).map_err(|source| LoadError::Io {
        path: schema_path.display().to_string(),
        source,
    })?;
    let product = match product_path {
        None => None,
        Some(path) => {
            let src = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
                path: path.display().to_string(),
                source,
            })?;
            // A TOML document parses into a table value (the rule-D1 join
            // walks it as `product.<table>.<key>` paths).
            let table = toml::from_str::<toml::Table>(&src).map_err(|err| {
                LoadError::Invalid(format!(
                    "{}: product.toml syntax error: {err}",
                    path.display()
                ))
            })?;
            Some(toml::Value::Table(table))
        }
    };
    parse_and_validate_str(&schema_src, product.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed seed + the committed product.toml must round-trip the
    /// FULL pipeline (parse + D1 product join + D2–D6) with zero findings —
    /// the Phase-1 integration invariant.
    #[test]
    fn committed_seed_validates_against_committed_product() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let schema = load(
            &root.join("schemas/settings.toml"),
            Some(&root.join("product.toml")),
        );
        let failure = match &schema {
            Ok(_) => String::new(),
            Err(err) => err.to_string(),
        };
        assert_eq!(
            failure, "",
            "committed seed must validate against committed product.toml"
        );
        let schema = match schema {
            Ok(schema) => schema,
            Err(_) => return,
        };
        // Phase-7 exit gate OF-2 guard: the seed's own rows are the single
        // source of truth for these counts. They are DERIVED here with an
        // independent raw-TOML read — not pinned as a second literal — so
        // a seed that legitimately grows (c71cd5a grew it 27→32) can never
        // strand this assertion stale again; only a loader that drops or
        // invents entries can fail it.
        let seed_text = std::fs::read_to_string(root.join("schemas/settings.toml"))
            .unwrap_or_else(|err| unreachable!("reading the committed seed: {err}"));
        let seed = toml::from_str::<toml::Table>(&seed_text)
            .unwrap_or_else(|err| unreachable!("parsing the committed seed: {err}"));
        let seed_rows = seed
            .get("setting")
            .and_then(toml::Value::as_array)
            .map_or(0, Vec::len);
        let seed_areas = seed
            .get("areas")
            .and_then(toml::Value::as_table)
            .and_then(|areas| areas.get("known"))
            .and_then(toml::Value::as_array)
            .map_or(0, Vec::len);
        assert_eq!(
            schema.len(),
            seed_rows,
            "the loader must carry every [[setting]] row of the committed seed"
        );
        assert_eq!(
            schema.areas.len(),
            seed_areas,
            "the loader must carry the whole [areas] known registry \
             (the doc 07 §5.4 screens + engine)"
        );
    }

    #[test]
    fn every_seed_default_ref_resolves_in_product_toml() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let schema = match load(
            &root.join("schemas/settings.toml"),
            Some(&root.join("product.toml")),
        ) {
            Ok(schema) => schema,
            Err(err) => {
                assert_eq!(String::new(), err.to_string(), "seed must load");
                return;
            }
        };
        let refs: Vec<&str> = schema
            .settings
            .iter()
            .filter_map(|d| match &d.default {
                model::DefaultValue::Ref(r) => Some(r.as_str()),
                model::DefaultValue::Literal(_) => None,
            })
            .collect();
        // The load above already proves D1 resolution; assert the shape:
        // the seed mirrors product.toml areas heavily via default_ref.
        assert!(refs.len() >= 20, "expected >=20 default_refs, got {refs:?}");
    }
}
