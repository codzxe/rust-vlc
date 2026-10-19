//! THE LUMEN THEME-TOKENS CRATE (card 1.B4 — doc 08 §3, doc 01 §2.3).
//!
//! Design-token pipeline: [`parse()`] reads the canonical token source
//! `themes/tokens.json` — a W3C Design Tokens draft subset (nested groups,
//! `$value`/`$type`/`$description`, `{dotted.path}` alias references,
//! group `$type` inheritance) plus one lumen extension (per-mode `$value`
//! objects) — validates the three-layer grammar
//! primitive → alias → semantic (doc 08 §3.2) and produces a [`TokenSet`].
//!
//! The emitters ([`emit::emit_css`], [`emit::emit_rust`]) render the
//! semantic layer deterministically (sorted keys, stable formatting) into:
//!
//! * `themes/generated/tokens.css` — CSS custom properties switched by
//!   `:root[data-theme="dark|light"]` and `:root[data-accent="…"]`
//!   (doc 08 §3.3), and
//! * `crates/theme-tokens/src/generated/tokens.rs` — Rust constants for
//!   engine-side needs (doc 01 §2.3), included below as [`generated`].
//!
//! Both outputs are COMMITTED and diff-tested (`tests/diff.rs`);
//! regeneration through `cargo xtask tokens` is idempotent. The
//! `tokens-gen` binary (`src/bin/tokens_gen.rs`) is the entry point the
//! (frozen) xtask `tokens` subcommand shells out to — the Lead-ratified
//! subprocess bridge, since xtask cannot take a path dependency without an
//! edit to its frozen manifest (doc 08 §3.1).
//!
//! Components consume SEMANTIC tokens only (doc 08 §3.2);
//! [`lint::lint_source`] checks a source text for primitive/alias name
//! references and raw color literals that bypass the semantic layer.
//!
//! Dependency policy (doc 11): `serde_json` only (pre-audit PASS,
//! `docs/dependencies/preaudit-foundation.md` §1).

pub mod color;
pub mod emit;
pub mod lint;
pub mod model;
pub mod parse;

/// The committed, generated Rust constants (doc 08 §3.1). Regenerate with
/// `cargo xtask tokens` — never edit by hand.
#[path = "generated/tokens.rs"]
pub mod generated;

pub use model::TokenSet;
pub use parse::parse;

use std::fmt;

/// Every failure mode of the token pipeline. All variants carry actionable
/// context (the offending path / reference / accent); `Display` renders a
/// single diagnostic line.
#[derive(Debug)]
pub enum TokenError {
    /// Malformed token source: bad JSON, a schema violation, or a broken
    /// layering rule. `path` is the dotted token path (or `$extensions` /
    /// the layer group) the diagnostic is about.
    Parse {
        /// Dotted path of the offending token or group ("" = whole file).
        path: String,
        /// What is wrong.
        message: String,
    },
    /// A `{dotted.path}` reference cycle (`a -> b -> a`).
    Cycle {
        /// The full cycle chain, arrow-separated.
        chain: String,
    },
    /// A `{dotted.path}` reference that does not resolve to any token.
    UnknownReference {
        /// Token containing the reference.
        from: String,
        /// The unresolved reference target.
        target: String,
    },
    /// An accent-ramp contrast failure (doc 08 §3.3: on-accent glyphs need
    /// ≥ 4.5:1 against the accent primary).
    Contrast {
        /// What failed.
        message: String,
    },
    /// An emission-time rule violation (duplicate CSS/Rust names, an
    /// unsupported mode set for the accent-ramp model, ...).
    Emit {
        /// What is wrong.
        message: String,
    },
    /// An I/O failure while reading the source or writing an output.
    Io {
        /// What we were doing when the error occurred.
        context: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// An internal invariant was violated — a bug in this crate, not in the
    /// token source.
    Internal(String),
}

impl TokenError {
    /// Build a [`TokenError::Parse`].
    pub fn parse(path: impl Into<String>, message: impl Into<String>) -> Self {
        TokenError::Parse {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Build a [`TokenError::Cycle`].
    pub fn cycle(chain: impl Into<String>) -> Self {
        TokenError::Cycle {
            chain: chain.into(),
        }
    }

    /// Build a [`TokenError::UnknownReference`].
    pub fn unknown_reference(from: &str, target: &str) -> Self {
        TokenError::UnknownReference {
            from: from.to_owned(),
            target: target.to_owned(),
        }
    }

    /// Build a [`TokenError::Contrast`].
    pub fn contrast(message: impl Into<String>) -> Self {
        TokenError::Contrast {
            message: message.into(),
        }
    }

    /// Build an [`TokenError::Emit`].
    pub fn emit(message: impl Into<String>) -> Self {
        TokenError::Emit {
            message: message.into(),
        }
    }

    /// Attach a what-was-happening context to an I/O error.
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        TokenError::Io {
            context: context.into(),
            source,
        }
    }

    /// Build a [`TokenError::Internal`].
    pub fn internal(message: impl Into<String>) -> Self {
        TokenError::Internal(message.into())
    }
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::Parse { path, message } => {
                if path.is_empty() {
                    write!(f, "tokens.json: {message}")
                } else {
                    write!(f, "tokens.json `{path}`: {message}")
                }
            }
            TokenError::Cycle { chain } => write!(f, "reference cycle: {chain}"),
            TokenError::UnknownReference { from, target } => {
                write!(f, "`{from}` references unknown token `{target}`")
            }
            TokenError::Contrast { message } => write!(f, "accent contrast: {message}"),
            TokenError::Emit { message } => write!(f, "emission: {message}"),
            TokenError::Io { context, source } => write!(f, "{context}: {source}"),
            TokenError::Internal(message) => write!(
                f,
                "internal error: {message} (a bug in theme-tokens — please report it)"
            ),
        }
    }
}

impl std::error::Error for TokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TokenError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn parse_error_names_the_offending_path() {
        let err = TokenError::parse("primitive.color.gray.900", "bad color `#0909`");
        assert_eq!(
            err.to_string(),
            "tokens.json `primitive.color.gray.900`: bad color `#0909`"
        );
    }

    #[test]
    fn parse_error_without_path_addresses_the_whole_file() {
        let err = TokenError::parse("", "invalid JSON: EOF");
        assert_eq!(err.to_string(), "tokens.json: invalid JSON: EOF");
    }

    #[test]
    fn unknown_reference_renders_from_and_target() {
        let err = TokenError::unknown_reference("alias.color.record", "primitive.color.rec");
        assert_eq!(
            err.to_string(),
            "`alias.color.record` references unknown token `primitive.color.rec`"
        );
    }

    #[test]
    fn io_error_carries_context_and_source() {
        let err = TokenError::io(
            "reading themes/tokens.json",
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        );
        assert_eq!(err.to_string(), "reading themes/tokens.json: no such file");
        assert!(err.source().is_some());
    }
}
