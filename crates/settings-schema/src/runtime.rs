//! G4 — runtime validation (doc 07 §5.3): the schema is embedded via
//! `include_str!` and every `config.set-setting`-style write is validated
//! against it (type/range/enum) before applying — `invalid-argument` on
//! violation, never silent coercion (doc 05 §2.11).
//!
//! The embedded copy is self-checked (parse + D2–D6) on first use; the D1
//! `product.toml` join needs the wider world and runs in the generator
//! binary / the 1.E2 `config-validator` wiring, not at process start.

use std::sync::OnceLock;

use crate::model::SettingDef;
use crate::model::SettingsSchema;
use crate::model::ValueViolation;
use crate::model::check_value;

/// The committed schema source, embedded at compile time (G4).
const EMBEDDED_SOURCE: &str = include_str!("../../../schemas/settings.toml");

static EMBEDDED: OnceLock<Result<SettingsSchema, String>> = OnceLock::new();

/// The embedded settings schema (parsed and self-validated on first use).
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// The committed `schemas/settings.toml` failed self-validation — a
    /// build-breaking state (the committed file must always be valid).
    #[error("the embedded settings schema is invalid:\n{0}")]
    EmbeddedInvalid(String),
}

/// A runtime value-check failure (the `invalid-argument` surface).
#[derive(Debug, thiserror::Error)]
pub enum ValueError {
    /// The key is not a setting.
    #[error("invalid-argument: unknown setting key `{key}` — {hint}")]
    UnknownKey {
        /// The rejected key.
        key: String,
        /// "Did you mean" hint (empty when nothing is close).
        hint: String,
    },
    /// The value violates the entry's type/domain.
    #[error("invalid-argument: setting `{key}`: {violation}")]
    Violation {
        /// The setting key.
        key: String,
        /// What went wrong (V-TYPE / V-RANGE / V-ENUM / V-FORMAT).
        #[source]
        violation: ValueViolation,
    },
}

/// The embedded schema: parsed and self-validated exactly once per process.
pub fn embedded() -> Result<&'static SettingsSchema, SchemaError> {
    let init = EMBEDDED.get_or_init(|| match crate::parse::parse_str(EMBEDDED_SOURCE) {
        Ok(schema) => {
            let diags = crate::validate::validate(&schema, None);
            if diags.is_empty() {
                Ok(schema)
            } else {
                Err(crate::diagnostic::render_all(&diags))
            }
        }
        Err(diags) => Err(crate::diagnostic::render_all(&diags)),
    });
    init.as_ref()
        .map_err(|msg| SchemaError::EmbeddedInvalid(msg.clone()))
}

/// Validate a write for one setting key (the `config.set-setting` seam).
/// Every layer's value for a known key is validated against the same
/// schema entry before merging (doc 07 §4.4 rule 4).
pub fn validate_value(key: &str, value: &toml::Value) -> Result<(), ValueError> {
    let schema = embedded().map_err(|err| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: err.to_string(),
    })?;
    let def = schema.entry(key).ok_or_else(|| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: unknown_key_hint(schema, key),
    })?;
    check_value(def.ty, &def.domain, value).map_err(|violation| ValueError::Violation {
        key: key.to_owned(),
        violation,
    })
}

/// The schema metadata for one key — backs `config.get-setting`'s
/// `schema_meta` result field (doc 07 §5.3 G4).
pub fn schema_meta(key: &str) -> Result<&'static SettingDef, ValueError> {
    let schema = embedded().map_err(|err| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: err.to_string(),
    })?;
    schema.entry(key).ok_or_else(|| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: unknown_key_hint(schema, key),
    })
}

/// "Did you mean" hint over the known keys (Levenshtein ≤ 3).
fn unknown_key_hint(schema: &SettingsSchema, key: &str) -> String {
    let mut best: Option<(usize, &str)> = None;
    for candidate in &schema.settings {
        let distance = edit_distance(key, &candidate.key);
        if distance <= 3 && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate.key.as_str()));
        }
    }
    match best {
        Some((_, near)) => format!("did you mean `{near}`?"),
        None => format!(
            "the schema defines {} setting keys; `cargo xtask schema-gen --check` \
             regenerates the reference",
            schema.len()
        ),
    }
}

/// Plain Levenshtein distance over bytes (keys are ASCII).
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        core::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_schema_parses_and_self_validates() {
        let failure = match embedded() {
            Ok(_) => String::new(),
            Err(err) => err.to_string(),
        };
        assert_eq!(failure, "", "the committed seed must self-validate");
        let schema = match embedded() {
            Ok(schema) => schema,
            Err(_) => return,
        };
        assert!(schema.len() >= 20, "Phase-1 seed has ~27 entries");
        assert!(schema.areas.contains(&"engine".to_string()));
    }

    #[test]
    fn validate_value_accepts_in_domain_values() {
        assert!(validate_value("playback.subtitle-mode", &"on".into()).is_ok());
        assert!(validate_value("engine.tick-interval-ms", &250.into()).is_ok());
        assert!(
            validate_value(
                "playback.preferred-audio-languages",
                &toml::Value::Array(vec!["en".into(), "de".into()])
            )
            .is_ok()
        );
        assert!(validate_value("privacy.telemetry", &false.into()).is_ok());
    }

    #[test]
    fn validate_value_rejects_out_of_domain_values() {
        let msg = match validate_value("playback.subtitle-mode", &"bogus".into()) {
            Err(err) => err.to_string(),
            Ok(()) => String::new(),
        };
        assert!(msg.contains("V-ENUM"), "enum violation must fail: {msg}");
        assert!(msg.contains("one of auto, forced-only, on, off"), "{msg}");
        assert!(msg.contains("invalid-argument"), "{msg}");

        let msg = match validate_value("engine.tick-interval-ms", &10.into()) {
            Err(err) => err.to_string(),
            Ok(()) => String::new(),
        };
        assert!(
            msg.contains("V-RANGE") && msg.contains("50..=2000"),
            "{msg}"
        );

        let msg = match validate_value("privacy.telemetry", &"yes".into()) {
            Err(err) => err.to_string(),
            Ok(()) => String::new(),
        };
        assert!(msg.contains("V-TYPE"), "{msg}");
    }

    #[test]
    fn unknown_key_gets_a_did_you_mean_hint() {
        let msg = match validate_value("playback.subtitle-mod", &"on".into()) {
            Err(err) => err.to_string(),
            Ok(()) => String::new(),
        };
        assert!(
            msg.contains("unknown setting key `playback.subtitle-mod`"),
            "{msg}"
        );
        assert!(
            msg.contains("did you mean `playback.subtitle-mode`"),
            "{msg}"
        );
    }

    #[test]
    fn schema_meta_backs_get_setting() {
        let meta = match schema_meta("playback.hw-decode-policy") {
            Ok(meta) => meta,
            Err(err) => {
                assert_eq!(
                    String::new(),
                    err.to_string(),
                    "hw-decode-policy must exist"
                );
                return;
            }
        };
        assert_eq!(meta.feature_gate, "hw-decode");
        assert_eq!(meta.placement.screen, "playback");
        assert_eq!(meta.ty.as_str(), "enum");
    }

    #[test]
    fn edit_distance_is_symmetric_and_small_for_near_misses() {
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("abc", "xyz"), 3);
    }
}
