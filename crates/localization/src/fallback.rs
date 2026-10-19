//! The fallback chain and locale selection (doc 07 §6.3).
//!
//! ```text
//! requested locale
//!   → its language-primary parent (pt-BR → pt, when such a catalog exists)
//!   → the product default locale
//!   → en-US (the reference catalog — always available)
//!   → (debug builds only) the key itself, rendered as missing:<key>
//! ```
//!
//! The chain is total: a missing translation can never panic rendering.
//! Every miss records a [`FallbackDiagnostic`] — CI (doc 07 §6.4) makes
//! the final links unreachable in shipped builds, but the runtime still
//! degrades honestly instead of crashing.
//!
//! Locale selection (doc 07 §6.3): `global.language` setting → OS locale
//! → `defaults.locale`. [`select_locale`] is a pure function over those
//! hints; reading the OS locale is the host application's job.

use crate::catalog::{Catalog, REFERENCE_LOCALE};
use crate::key::MessageKey;
use crate::locale::Locale;
use crate::message::{Arg, RenderDiagnostic};

/// Choose the requested locale from the doc 07 §6.3 preference list:
/// the language setting, else the OS locale, else the product default.
pub fn select_locale(setting: Option<&Locale>, os: Option<&Locale>, default: &Locale) -> Locale {
    setting.or(os).unwrap_or(default).clone()
}

/// A fallback-chain problem, recorded (never panicked on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackDiagnostic {
    /// The key was not found in any catalog of the chain.
    MissingKey { key: MessageKey },
}

impl std::fmt::Display for FallbackDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallbackDiagnostic::MissingKey { key } => {
                write!(
                    f,
                    "missing key {key} in every catalog of the fallback chain"
                )
            }
        }
    }
}

/// The result of a translation lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    /// The rendered text. Always present; degraded visibly on misses.
    pub text: String,
    /// The locale whose catalog resolved the key (`None` = not found
    /// anywhere — the text is the debug `missing:<key>` form or the raw
    /// key in release builds).
    pub resolved_locale: Option<Locale>,
    /// Render-time diagnostics (missing args, type mismatches) plus
    /// fallback diagnostics (missing keys).
    pub diagnostics: Vec<RenderOrFallback>,
}

/// Render or fallback diagnostics, together for the caller's log sink.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderOrFallback {
    /// A message-render diagnostic (see [`RenderDiagnostic`]).
    Render(RenderDiagnostic),
    /// A fallback-chain diagnostic (see [`FallbackDiagnostic`]).
    Fallback(FallbackDiagnostic),
}

impl std::fmt::Display for RenderOrFallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderOrFallback::Render(diagnostic) => write!(f, "{diagnostic}"),
            RenderOrFallback::Fallback(diagnostic) => write!(f, "{diagnostic}"),
        }
    }
}

/// An ordered chain of catalogs implementing the doc 07 §6.3 fallback.
///
/// Keys are resolved per-key: the first catalog in the chain that contains
/// the key renders it (with that catalog's locale, so plural rules follow
/// the *translating* locale, not the requested one).
#[derive(Debug, Clone)]
pub struct Localizer {
    chain: Vec<Catalog>,
}

impl Localizer {
    /// Build the fallback chain from the requested locale, the available
    /// catalogs, and the product default locale (doc 07 §6.3). The
    /// reference catalog (`en-US`) is always appended last when present
    /// in `available`; duplicates are collapsed, order preserved.
    pub fn build(requested: &Locale, available: &[Catalog], default: &Locale) -> Localizer {
        let mut preferences: Vec<Locale> = vec![requested.clone()];
        let parent = requested.language_primary();
        if parent != *requested {
            preferences.push(parent);
        }
        preferences.push(default.clone());
        if default.as_str() != REFERENCE_LOCALE {
            preferences.push(Locale::reference());
        }
        let mut chain: Vec<Catalog> = Vec::new();
        for preference in preferences {
            for catalog in available {
                if *catalog.locale() == preference
                    && !chain
                        .iter()
                        .any(|chosen| chosen.locale() == catalog.locale())
                {
                    chain.push(catalog.clone());
                }
            }
        }
        Localizer { chain }
    }

    /// The chain's locales, in fallback order (for tests and debugging).
    pub fn locales(&self) -> impl Iterator<Item = &Locale> {
        self.chain.iter().map(|catalog| catalog.locale())
    }

    /// Translate `key` with `args`, walking the fallback chain per key.
    /// Never panics (doc 07 §6.3).
    pub fn translate(&self, key: &MessageKey, args: &[(&str, Arg)]) -> Translation {
        for catalog in &self.chain {
            if let Some(message) = catalog.get(key) {
                let rendered = message.render(catalog.locale(), args);
                return Translation {
                    text: rendered.text,
                    resolved_locale: Some(catalog.locale().clone()),
                    diagnostics: rendered
                        .diagnostics
                        .into_iter()
                        .map(RenderOrFallback::Render)
                        .collect(),
                };
            }
        }
        // Chain exhausted: debug builds render the miss visibly as
        // `missing:<key>` (doc 07 §6.3); release builds render the raw
        // key — CI makes both unreachable, but the runtime stays honest.
        let text = if cfg!(debug_assertions) {
            format!("missing:{key}")
        } else {
            key.as_str().to_string()
        };
        Translation {
            text,
            resolved_locale: None,
            diagnostics: vec![RenderOrFallback::Fallback(FallbackDiagnostic::MissingKey {
                key: key.clone(),
            })],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Localizer, select_locale};
    use crate::catalog::Catalog;
    use crate::key::MessageKey;
    use crate::locale::Locale;
    use crate::message::Arg;

    const PT_BR: &str = "\
\"fixture.greeting\" = \"Olá, {name}!\"
\"fixture.two\" = \"Segunda chave\"
";
    const PT: &str = "\
\"fixture.greeting\" = \"Olá, {name}!\"
\"fixture.two\" = \"Segunda chave\"
\"fixture.only-in-pt\" = \"Só em pt\"
";
    const EN_US: &str = "\
\"fixture.greeting\" = \"Hello, {name}!\"
\"fixture.two\" = \"Second key\"
\"fixture.only-in-pt\" = \"Only in pt (en fallback)\"
\"fixture.only-in-en\" = \"Only in en\"
";

    fn catalogs() -> Result<Vec<Catalog>, Box<dyn std::error::Error>> {
        Ok(vec![
            Catalog::from_toml_str(Locale::parse("pt-BR")?, "pt-BR.toml", PT_BR)?,
            Catalog::from_toml_str(Locale::parse("pt")?, "pt.toml", PT)?,
            Catalog::from_toml_str(Locale::parse("en-US")?, "en-US.toml", EN_US)?,
        ])
    }

    #[test]
    fn chain_order_matches_doc07() -> Result<(), Box<dyn std::error::Error>> {
        let available = catalogs()?;
        let requested = Locale::parse("pt-BR")?;
        let default = Locale::parse("en-US")?;
        let localizer = Localizer::build(&requested, &available, &default);
        let chain: Vec<String> = localizer
            .locales()
            .map(|l| l.as_str().to_string())
            .collect();
        // requested → parent → default(=en-US reference, collapsed)
        assert_eq!(chain, ["pt-BR", "pt", "en-US"]);
        Ok(())
    }

    #[test]
    fn chain_with_distinct_default() -> Result<(), Box<dyn std::error::Error>> {
        let available = vec![
            Catalog::from_toml_str(
                Locale::parse("es")?,
                "es.toml",
                "\"fixture.greeting\" = \"¡Hola, {name}!\"\n",
            )?,
            Catalog::from_toml_str(Locale::parse("en-US")?, "en-US.toml", EN_US)?,
        ];
        let requested = Locale::parse("fr-FR")?; // no catalog
        let default = Locale::parse("es")?;
        let localizer = Localizer::build(&requested, &available, &default);
        let chain: Vec<String> = localizer
            .locales()
            .map(|l| l.as_str().to_string())
            .collect();
        // requested(fr-FR, absent) → parent(fr, absent) → default(es) → en-US
        assert_eq!(chain, ["es", "en-US"]);
        Ok(())
    }

    #[test]
    fn translates_with_requested_catalog() -> Result<(), Box<dyn std::error::Error>> {
        let available = catalogs()?;
        let localizer = Localizer::build(
            &Locale::parse("pt-BR")?,
            &available,
            &Locale::parse("en-US")?,
        );
        let key = MessageKey::new("fixture.greeting")?;
        let translation = localizer.translate(&key, &[("name", Arg::from("Lumen"))]);
        assert_eq!(translation.text, "Olá, Lumen!");
        assert_eq!(
            translation.resolved_locale.as_ref().map(|l| l.as_str()),
            Some("pt-BR")
        );
        assert!(translation.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn falls_back_per_key_to_parent_then_reference() -> Result<(), Box<dyn std::error::Error>> {
        let available = catalogs()?;
        let localizer = Localizer::build(
            &Locale::parse("pt-BR")?,
            &available,
            &Locale::parse("en-US")?,
        );

        // Missing in pt-BR, present in pt → parent resolves.
        let key = MessageKey::new("fixture.only-in-pt")?;
        let translation = localizer.translate(&key, &[]);
        assert_eq!(translation.text, "Só em pt");
        assert_eq!(
            translation.resolved_locale.as_ref().map(|l| l.as_str()),
            Some("pt")
        );

        // Missing everywhere except the reference → en-US resolves.
        let key = MessageKey::new("fixture.only-in-en")?;
        let translation = localizer.translate(&key, &[]);
        assert_eq!(translation.text, "Only in en");
        assert_eq!(
            translation.resolved_locale.as_ref().map(|l| l.as_str()),
            Some("en-US")
        );
        Ok(())
    }

    #[test]
    fn missing_everywhere_records_diagnostic_and_never_panics()
    -> Result<(), Box<dyn std::error::Error>> {
        let available = catalogs()?;
        let localizer = Localizer::build(
            &Locale::parse("pt-BR")?,
            &available,
            &Locale::parse("en-US")?,
        );
        let key = MessageKey::new("fixture.does-not-exist")?;
        let translation = localizer.translate(&key, &[]);
        // Debug builds (tests run in debug) show the miss visibly.
        assert_eq!(translation.text, "missing:fixture.does-not-exist");
        assert!(translation.resolved_locale.is_none());
        assert_eq!(translation.diagnostics.len(), 1);
        assert!(
            translation.diagnostics[0]
                .to_string()
                .contains("missing key")
        );
        Ok(())
    }

    #[test]
    fn locale_selection_prefers_setting_then_os_then_default()
    -> Result<(), Box<dyn std::error::Error>> {
        let setting = Locale::parse("de")?;
        let os = Locale::parse("fr-FR")?;
        let default = Locale::parse("en-US")?;
        assert_eq!(
            select_locale(Some(&setting), Some(&os), &default).as_str(),
            "de"
        );
        assert_eq!(select_locale(None, Some(&os), &default).as_str(), "fr-FR");
        assert_eq!(select_locale(None, None, &default).as_str(), "en-US");
        Ok(())
    }
}
