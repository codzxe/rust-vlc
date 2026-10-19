//! Plural categories — the CLDR cardinal-category data backing audit rule 5
//! (doc 07 §6.4) and runtime plural selection.
//!
//! **Honest data scope**: only category SETS that are stable, public CLDR
//! facts are seeded. For languages outside the table, audits enforce only
//! the universal `other` category. The full CLDR plural-rule tables (and
//! the exact float/visible-fraction semantics) land with the Phase-4
//! i18n-engine decision `[VERIFY — doc 07 §6.2]`; extending this table is
//! a data change, not an API change.

use std::fmt;

/// The six CLDR cardinal plural categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralCategory {
    /// The category's CLDR name, as written inside a `{n, plural, …}` arm.
    pub fn as_str(&self) -> &'static str {
        match self {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other",
        }
    }

    /// Parse a plural-arm name. `exact` (CLDR `=n` forms) is not part of
    /// the v1 subset (doc 07 §6.2) and is rejected as unknown.
    pub fn parse(name: &str) -> Option<PluralCategory> {
        match name {
            "zero" => Some(PluralCategory::Zero),
            "one" => Some(PluralCategory::One),
            "two" => Some(PluralCategory::Two),
            "few" => Some(PluralCategory::Few),
            "many" => Some(PluralCategory::Many),
            "other" => Some(PluralCategory::Other),
            _ => None,
        }
    }

    /// All six categories, in canonical order.
    pub const ALL: [PluralCategory; 6] = [
        PluralCategory::Zero,
        PluralCategory::One,
        PluralCategory::Two,
        PluralCategory::Few,
        PluralCategory::Many,
        PluralCategory::Other,
    ];
}

impl fmt::Display for PluralCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The required cardinal-category set for a language (primary subtag),
/// or `None` when the language is not in the Phase-1 seed table.
///
/// When `None`, audits enforce only `other` — the one category every
/// language requires — and do not reject unused arms.
pub fn categories_for_language(language: &str) -> Option<&'static [PluralCategory]> {
    // Stable CLDR cardinal sets (unicode.org cldr plural rules); seeded
    // conservatively — see the module docs.
    match language {
        "en" | "de" | "es" | "fr" | "it" | "nl" | "pt" => {
            Some(&[PluralCategory::One, PluralCategory::Other])
        }
        "ru" | "pl" => Some(&[
            PluralCategory::One,
            PluralCategory::Few,
            PluralCategory::Many,
            PluralCategory::Other,
        ]),
        "ar" => Some(&[
            PluralCategory::Zero,
            PluralCategory::One,
            PluralCategory::Two,
            PluralCategory::Few,
            PluralCategory::Many,
            PluralCategory::Other,
        ]),
        "ja" | "zh" | "ko" | "th" | "vi" => Some(&[PluralCategory::Other]),
        _ => None,
    }
}

/// Select the cardinal category for `value` under `language`, for message
/// rendering.
///
/// v1 rule (documented simplification): integer `1` selects `one`,
/// everything else selects `other`. This is exact for the `one/other` and
/// `other`-only languages seeded above; it is *not* the full CLDR
/// algorithm for `ru`/`pl`/`ar` (e.g. Russian 2 → `few`), which is why
/// those languages' runtime plural *rendering* is `[VERIFY]` until the
/// engine decision — while their audit-side category sets are already
/// enforced. Callers that need exact categories before then must not ship
/// pluralized copy in those languages.
pub fn select_plural_category(language: &str, value: i64) -> PluralCategory {
    match categories_for_language(language) {
        Some(set) if !set.contains(&PluralCategory::One) => PluralCategory::Other,
        _ if value == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{PluralCategory, categories_for_language, select_plural_category};

    #[test]
    fn category_roundtrip() {
        for category in PluralCategory::ALL {
            assert_eq!(PluralCategory::parse(category.as_str()), Some(category));
        }
        assert_eq!(PluralCategory::parse("manys"), None);
        assert_eq!(PluralCategory::parse("=1"), None);
        assert_eq!(PluralCategory::parse(""), None);
    }

    #[test]
    fn seeded_sets() {
        assert_eq!(
            categories_for_language("en"),
            Some(&[PluralCategory::One, PluralCategory::Other][..])
        );
        assert_eq!(
            categories_for_language("ru"),
            Some(
                &[
                    PluralCategory::One,
                    PluralCategory::Few,
                    PluralCategory::Many,
                    PluralCategory::Other
                ][..]
            )
        );
        assert_eq!(
            categories_for_language("ar"),
            Some(
                &[
                    PluralCategory::Zero,
                    PluralCategory::One,
                    PluralCategory::Two,
                    PluralCategory::Few,
                    PluralCategory::Many,
                    PluralCategory::Other
                ][..]
            )
        );
        assert_eq!(
            categories_for_language("ja"),
            Some(&[PluralCategory::Other][..])
        );
        assert_eq!(categories_for_language("xx"), None);
    }

    #[test]
    fn v1_selection_rule() {
        assert_eq!(select_plural_category("en", 1), PluralCategory::One);
        assert_eq!(select_plural_category("en", 0), PluralCategory::Other);
        assert_eq!(select_plural_category("en", 2), PluralCategory::Other);
        // one-less languages never select `one` under the v1 rule.
        assert_eq!(select_plural_category("ja", 1), PluralCategory::Other);
        // Documented limitation: ru 2 should be `few` under full CLDR; the
        // v1 render rule says `other` — audit rule 5 still requires the
        // `few` ARM to exist in ru catalogs.
        assert_eq!(select_plural_category("ru", 2), PluralCategory::Other);
    }
}
