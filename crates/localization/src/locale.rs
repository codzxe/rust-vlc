//! Locale tags — minimal BCP-47 well-formedness with canonical casing
//! (doc 07 §6.3: "file name = BCP-47 tag").
//!
//! Validation follows the shape pinned by the product schema
//! (doc 07 §3.2 `$defs.bcp47`): `^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$`,
//! matched case-insensitively and canonicalized to standard casing
//! (language lowercase, 4-letter scripts Titlecase, 2–3-letter regions
//! uppercase). Full registry validation (extlangs, extensions, private-use,
//! grandfathered tags, registered variants) is deferred to the Phase-4
//! i18n engine decision `[VERIFY — doc 07 §6.2]`.

use std::fmt;

/// Ways a locale tag can be malformed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LocaleError {
    /// The tag is empty.
    #[error("empty locale tag")]
    Empty,
    /// The primary language subtag must be 2–3 ASCII letters.
    #[error("primary language subtag must be 2-3 letters, found {found:?}")]
    InvalidLanguage { found: String },
    /// A subsequent subtag must be 1–8 ASCII alphanumerics.
    #[error(
        "subtag {found:?} must be 1-8 ASCII alphanumerics (no underscore; BCP-47 uses hyphens)"
    )]
    InvalidSubtag { found: String },
    /// Subtags must be separated by single hyphens; `--` produces an empty
    /// subtag.
    #[error("empty subtag (check for `--` or a trailing hyphen)")]
    EmptySubtag,
}

/// A validated, canonically-cased locale tag (doc 07 §6.3).
///
/// # Examples
///
/// ```
/// use localization::Locale;
///
/// let locale = Locale::parse("pt-br")?;
/// assert_eq!(locale.as_str(), "pt-BR");           // canonical casing
/// assert_eq!(locale.language_primary().as_str(), "pt"); // fallback parent
///
/// assert!(Locale::parse("en_US").is_err()); // underscore, not BCP-47
/// assert!(Locale::parse("x").is_err());     // 1-letter language
/// # Ok::<(), localization::LocaleError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locale {
    tag: String,
}

impl Locale {
    /// The reference locale (doc 07 §6.3: en-US, always available as the
    /// last link of the fallback chain). A constructor, not a `const` —
    /// `Locale` owns a heap-allocated tag — carrying the same literal as
    /// [`crate::catalog::REFERENCE_LOCALE`], the single source of truth
    /// for the tag string.
    pub fn reference() -> Locale {
        Locale {
            tag: crate::catalog::REFERENCE_LOCALE.to_string(),
        }
    }

    /// Parse and canonicalize a BCP-47-shaped tag.
    pub fn parse(tag: &str) -> Result<Self, LocaleError> {
        if tag.is_empty() {
            return Err(LocaleError::Empty);
        }
        let subtags: Vec<&str> = tag.split('-').collect();
        let language = subtags[0];
        if language.len() < 2
            || language.len() > 3
            || !language.chars().all(|c| c.is_ascii_alphabetic())
        {
            // The classic `en_US` mistake (underscore separator): report it
            // through the subtag rule, whose message names BCP-47 hyphens —
            // the actionable hint for the most common real-world error.
            if language.contains('_') {
                return Err(LocaleError::InvalidSubtag {
                    found: language.to_string(),
                });
            }
            return Err(LocaleError::InvalidLanguage {
                found: language.to_string(),
            });
        }
        let mut canonical = String::with_capacity(tag.len());
        canonical.push_str(&language.to_ascii_lowercase());
        for subtag in &subtags[1..] {
            if subtag.is_empty() {
                return Err(LocaleError::EmptySubtag);
            }
            if subtag.len() > 8 || !subtag.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(LocaleError::InvalidSubtag {
                    found: (*subtag).to_string(),
                });
            }
            canonical.push('-');
            if subtag.len() == 4 && subtag.chars().all(|c| c.is_ascii_alphabetic()) {
                // Script subtag: Titlecase (Hans, Latn).
                let mut chars = subtag.chars();
                let first = chars.next().map(|c| c.to_ascii_uppercase());
                if let Some(first) = first {
                    canonical.push(first);
                }
                canonical.push_str(&subtag[1..].to_ascii_lowercase());
            } else if (subtag.len() == 2 || subtag.len() == 3)
                && subtag.chars().all(|c| c.is_ascii_alphabetic())
            {
                // Region subtag: uppercase (US, BRA, 419 stays as-is below).
                canonical.push_str(&subtag.to_ascii_uppercase());
            } else {
                // Variants, numeric regions (419), extensions: lowercase.
                canonical.push_str(&subtag.to_ascii_lowercase());
            }
        }
        Ok(Self { tag: canonical })
    }

    /// The canonical tag, e.g. `pt-BR`.
    pub fn as_str(&self) -> &str {
        &self.tag
    }

    /// The language-primary parent used by the fallback chain (doc 07
    /// §6.3: `pt-BR` → `pt`). Always a valid single-subtag locale.
    pub fn language_primary(&self) -> Locale {
        // Invariant from `parse`: the tag begins with a 2–3 lowercase-letter
        // language subtag (never empty, never a leading hyphen), so the first
        // split item is itself a valid canonical tag — no re-validation
        // needed. `unwrap_or_default` only guards the impossible empty-tag
        // case.
        let primary = self.tag.split('-').next().unwrap_or_default().to_string();
        Locale { tag: primary }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.tag)
    }
}

impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        &self.tag
    }
}

#[cfg(test)]
mod tests {
    use super::{Locale, LocaleError};

    #[test]
    fn parses_and_canonicalizes() -> Result<(), LocaleError> {
        let cases: &[(&str, &str)] = &[
            ("en-US", "en-US"),
            ("EN-us", "en-US"),
            ("pt-br", "pt-BR"),
            ("de", "de"),
            ("zh-hans-cn", "zh-Hans-CN"),
            ("es-419", "es-419"),
            ("en-us-posix", "en-US-posix"),
            ("fr-FR", "fr-FR"),
        ];
        for (input, canonical) in cases {
            assert_eq!(&Locale::parse(input)?.as_str(), canonical, "case {input:?}");
        }
        Ok(())
    }

    #[test]
    fn rejects_malformed_tags() {
        let cases: &[(&str, LocaleError)] = &[
            ("", LocaleError::Empty),
            (
                "x",
                LocaleError::InvalidLanguage {
                    found: "x".to_string(),
                },
            ),
            (
                "engl",
                LocaleError::InvalidLanguage {
                    found: "engl".to_string(),
                },
            ),
            (
                "en_US",
                LocaleError::InvalidSubtag {
                    found: "en_US".to_string(),
                },
            ),
            ("en-", LocaleError::EmptySubtag),
            ("en--US", LocaleError::EmptySubtag),
            (
                "en-UltraLongSubtag",
                LocaleError::InvalidSubtag {
                    found: "UltraLongSubtag".to_string(),
                },
            ),
        ];
        for (input, expected) in cases {
            let got = Locale::parse(input).err();
            assert_eq!(got.as_ref(), Some(expected), "case {input:?}");
        }
    }

    #[test]
    fn language_primary_parent() -> Result<(), LocaleError> {
        assert_eq!(Locale::parse("pt-BR")?.language_primary().as_str(), "pt");
        assert_eq!(
            Locale::parse("zh-Hans-CN")?.language_primary().as_str(),
            "zh"
        );
        assert_eq!(Locale::parse("en-US")?.language_primary().as_str(), "en");
        assert_eq!(Locale::parse("de")?.language_primary().as_str(), "de");
        Ok(())
    }
}
