//! # Lumen localization (doc 07 §6)
//!
//! Stable message keys, parameterized messages, an explicit fallback chain,
//! and catalog auditing. This crate is the *mechanism*; catalogs are data in
//! `locales/` (doc 01 §2.3): `locales/en-US.toml` is the reference catalog,
//! other catalogs are one file per BCP-47 tag (`de.toml`, `pt-BR.toml`).
//!
//! ## Key grammar (doc 07 §6.1)
//!
//! A message key is **2–5 dotted segments**, each matching `[a-z0-9-]+`
//! (kebab-case; no uppercase, no underscores):
//!
//! ```text
//! player.controls.play          ← 3 segments, valid
//! settings.playback.subtitle-mode.label ← 5 segments, valid
//! common.reset                  ← 2 segments, valid
//! ```
//!
//! Invalid: `a` (one segment), `a.b.c.d.e.f` (six), `Player.Play`
//! (uppercase), `player_controls.play` (underscore — placeholders use
//! snake_case, keys do not). Keys are *API*: they name the concept
//! (`player.controls.play`), never the wording
//! (`player.controls.play-button-green`), and renaming one is a breaking
//! change for every catalog. Reserved top-level namespaces (doc 07 §6.1):
//! `product`, `window`, `player`, `settings`, `menu`, `dialog`, `error`,
//! `common`. The Phase-1 reference seed additionally uses the
//! Lead-approved non-UI namespaces `xtask` (gate report labels) and `ci`
//! (CI stub naming); see `locales/en-US.toml`.
//!
//! ## Message format v1 subset (doc 07 §6.2)
//!
//! ```text
//! {name}                              simple placeholder
//! {name, number, percent}             percentage (fraction input, 0.45 → 45%)
//! {name, select, a {…} other {…}}     choice by string value; `other` required
//! {name, plural, one {…} other {…}}   choice by count; `other` required
//! ```
//!
//! Placeholder names are snake_case (`[A-Za-z_][A-Za-z0-9_]*`) and are part
//! of each key's contract. `{{` / `}}` escape literal braces; `#` inside a
//! plural arm renders the count. The full ICU-MessageFormat feature set
//! (and the pure-Rust engine choice) is a documented Phase-0/Phase-4
//! decision `[PROPOSAL, doc 07 §6.2]` — this subset is deliberately small
//! and self-contained.
//!
//! ## Fallback chain (doc 07 §6.3)
//!
//! requested locale → its language-primary parent (`pt-BR` → `pt`, when
//! such a catalog exists) → the product default locale → `en-US` →
//! (debug builds only) the key itself, rendered as `missing:<key>`. The
//! chain is total: a missing translation can never panic rendering; every
//! miss records a diagnostic. See [`fallback`].
//!
//! ## The five catalog-audit rules (doc 07 §6.4)
//!
//! 1. **Missing-key detection** — every catalog ⊇ the reference key set
//!    ([`audit::audit_missing_keys`], with [`audit::catalog_coverage`]).
//! 2. **Duplicate-key detection** — duplicate keys inside a file are TOML
//!    parse errors (parse-level, surfaced by [`catalog::Catalog::from_toml_str`]);
//!    a catalog key absent from the reference is an *orphan* error
//!    ([`audit::audit_orphan_keys`]).
//! 3. **Placeholder validation** — each key's placeholder set (names +
//!    types) must equal the reference's exactly ([`audit::audit_placeholders`]).
//!    `{percent, number, percent}` in en-US and `{percent}` in `de` is a
//!    CI failure.
//! 4. **Reference-completeness** — every `label_key` / `description_key` /
//!    `title_idle_key` / `description` msgid referenced from
//!    `product.toml`, `schemas/settings.toml`, `layouts/`, `keymaps/`,
//!    `profiles/`, `themes/` resolves to a reference-catalog key
//!    ([`refs::extract_msgid_refs`] + [`audit::audit_references`]).
//! 5. **Plural-rule conformance** — each `{n, plural, …}` message carries
//!    exactly the locale's CLDR cardinal categories
//!    ([`audit::audit_plural_conformance`], data in [`plural`]).
//!
//! [`audit::audit_catalogs`] runs all of them over a `locales/` directory;
//! `audit::audit_catalogs_with_refs` adds rule 4. The CI job wiring
//! (`cargo xtask l10n-audit`, doc 07 §6.4) belongs to card 1.E2 — the
//! xtask dispatch table is frozen (worklog 1.A3) — so until then this
//! crate's own test suite enforces the rules on the shipped tree
//! (`tests/seed_integrity.rs`) and on violation fixtures
//! (`tests/audit_proof_matrix.rs`).
//!
//! ## No literal UI text (doc 07 §6.5)
//!
//! The translation API accepts only [`key::MessageKey`] values — a
//! validated newtype — so a raw string literal cannot reach a user through
//! this API. The Rust heuristic sweep and the TypeScript/TSX ESLint rule
//! are separate enforcement layers (xtask `lint-literals`, `ui/.eslintrc.json`
//! stub); their fixtures are owned by the cards that land them.
//!
//! ## Lint policy
//!
//! Workspace lints apply (doc 10 §3.1): no `unsafe`, no
//! `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`. All failure
//! paths are typed errors or recorded diagnostics — including message
//! rendering, which must never panic (doc 07 §6.3: "a missing translation
//! can never crash rendering").

pub mod audit;
pub mod catalog;
pub mod fallback;
pub mod key;
pub mod locale;
pub mod message;
pub mod plural;
pub mod refs;

pub use audit::{CatalogIssue, audit_catalogs, audit_catalogs_with_refs};
pub use catalog::{Catalog, CatalogError};
pub use fallback::{Localizer, Translation, select_locale};
pub use key::{KeyError, MAX_SEGMENTS, MIN_SEGMENTS, MessageKey};
pub use locale::{Locale, LocaleError};
pub use message::{Arg, Message, MessageError, PlaceholderKind, RenderDiagnostic, Rendered};
pub use plural::{PluralCategory, categories_for_language, select_plural_category};
pub use refs::{MsgidRef, extract_msgid_refs};

/// Translate a byte offset into a 1-based `(line, column)` pair.
///
/// Columns count Unicode scalar values, not bytes, so diagnostics stay
/// honest in catalogs containing non-ASCII text. `pos` beyond the end of
/// `src` reports the position after the last character.
pub(crate) fn line_col(src: &str, pos: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::line_col;

    #[test]
    fn line_col_counts_lines_and_scalar_columns() {
        let src = "ab\ncdé\nf";
        assert_eq!(line_col(src, 0), (1, 1));
        assert_eq!(line_col(src, 1), (1, 2));
        assert_eq!(line_col(src, 3), (2, 1));
        // 'é' is two UTF-8 bytes but ONE column.
        assert_eq!(line_col(src, 5), (2, 3));
        assert_eq!(line_col(src, 8), (3, 1));
        // Beyond the end clamps to the final position.
        assert_eq!(line_col(src, 100), (3, 2));
    }
}
