//! Catalogs — `locales/<tag>.toml` files parsed into key→message maps
//! (doc 07 §6.3).
//!
//! A catalog file is **flat**: every top-level entry is a quoted dotted
//! key mapped to a TOML string. Duplicate keys are rejected by the TOML
//! parser itself (rule 2's in-file half, "parse-level for free" — doc 07
//! §6.2); nested tables and non-string values are rejected here with
//! actionable diagnostics so accidental `[section]` nesting cannot
//! silently change key identity.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::key::{KeyError, MessageKey};
use crate::line_col;
use crate::locale::Locale;
use crate::message::{Message, MessageError};

/// The reference locale (doc 07 §6.3): shipped first, complete, the
/// source of truth for the key set, and therefore always the last link of
/// the fallback chain.
pub const REFERENCE_LOCALE: &str = "en-US";

/// Ways a catalog file can fail to load.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// TOML syntax error — including duplicate keys (rule 2, in-file
    /// half). `key` is extracted from the error span when possible.
    #[error("parse error: {message} (line {line}, column {column})")]
    Parse {
        message: String,
        line: usize,
        column: usize,
        /// The duplicated key, when the parse error is a duplicate key.
        key: Option<String>,
    },
    /// The file name is not a valid BCP-47 tag.
    #[error("invalid locale tag {found:?}: {reason}")]
    InvalidLocaleTag { found: String, reason: String },
    /// I/O failure while reading the file.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    /// A key violates the doc 07 §6.1 grammar.
    #[error("key {key:?}: {reason}")]
    InvalidKey { key: String, reason: String },
    /// A value is not a TOML string.
    #[error("key {key:?}: value must be a TOML string, found {found}")]
    NonStringValue { key: String, found: &'static str },
    /// A nested table where a flat entry was expected.
    #[error(
        "table {key:?}: catalogs are flat — write quoted dotted keys \
         (e.g. \"a.b\" = \"text\"), not [a] sections with sub-keys"
    )]
    NestedTable { key: String },
    /// A value violates the message-format v1 subset.
    #[error("key {key:?}: {message} (line {line}, column {column})")]
    MessageFormat {
        key: String,
        message: String,
        line: usize,
        column: usize,
    },
}

/// One loaded catalog: a locale, its messages, and where it came from.
#[derive(Debug, Clone)]
pub struct Catalog {
    locale: Locale,
    source_name: String,
    messages: BTreeMap<MessageKey, Message>,
}

impl Catalog {
    /// Parse catalog contents. `source_name` names the origin in
    /// diagnostics (file name or a synthetic label).
    pub fn from_toml_str(
        locale: Locale,
        source_name: &str,
        src: &str,
    ) -> Result<Catalog, CatalogError> {
        let table: toml::Table = src
            .parse()
            .map_err(|error| catalog_parse_error(src, &error))?;
        let mut messages = BTreeMap::new();
        for (raw_key, value) in &table {
            // Value shape first: a nested `[section]` table must be
            // diagnosed as such even when the section name is not itself
            // a valid key (e.g. `[a]`), so the flat-catalog hint wins
            // over the key-grammar error.
            let text = match value {
                toml::Value::String(text) => text,
                toml::Value::Table(_) => {
                    return Err(CatalogError::NestedTable {
                        key: raw_key.clone(),
                    });
                }
                other => {
                    return Err(CatalogError::NonStringValue {
                        key: raw_key.clone(),
                        found: value_type_name(other),
                    });
                }
            };
            let key = MessageKey::new(raw_key).map_err(|error| match error {
                KeyError::Empty => CatalogError::InvalidKey {
                    key: String::new(),
                    reason: "empty key".to_string(),
                },
                KeyError::SegmentCount { found } => CatalogError::InvalidKey {
                    key: raw_key.clone(),
                    reason: format!(
                        "expected between {} and {} dotted segments, found {found}",
                        crate::key::MIN_SEGMENTS,
                        crate::key::MAX_SEGMENTS
                    ),
                },
                KeyError::InvalidSegment { segment } => CatalogError::InvalidKey {
                    key: raw_key.clone(),
                    reason: format!(
                        "segment {segment:?} must match [a-z0-9-]+ (kebab-case, no uppercase, no underscores)"
                    ),
                },
            })?;
            let message = Message::parse(text)
                .map_err(|error| catalog_message_error(src, raw_key, text, &error))?;
            messages.insert(key, message);
        }
        Ok(Catalog {
            locale,
            source_name: source_name.to_string(),
            messages,
        })
    }

    /// The catalog's locale.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Where this catalog came from (diagnostics label).
    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    /// Look up a message by key.
    pub fn get(&self, key: &MessageKey) -> Option<&Message> {
        self.messages.get(key)
    }

    /// Whether the key is present.
    pub fn contains_key(&self, key: &MessageKey) -> bool {
        self.messages.contains_key(key)
    }

    /// All keys, in lexicographic order (deterministic diagnostics).
    pub fn keys(&self) -> impl Iterator<Item = &MessageKey> {
        self.messages.keys()
    }

    /// Number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the catalog has no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl fmt::Display for Catalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "catalog {} ({} messages, {})",
            self.locale,
            self.messages.len(),
            self.source_name
        )
    }
}

fn value_type_name(value: &toml::Value) -> &'static str {
    match value {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "integer",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "boolean",
        toml::Value::Datetime(_) => "datetime",
        toml::Value::Array(_) => "array",
        toml::Value::Table(_) => "table",
    }
}

/// Map a TOML parse error to line/column, extracting the duplicated key
/// when the error is a duplicate-key error (rule 2's parse-level half).
fn catalog_parse_error(src: &str, error: &toml::de::Error) -> CatalogError {
    let message = error.message().to_string();
    let span = error.span();
    let (line, column) = span
        .as_ref()
        .map(|span| line_col(src, span.start))
        .unwrap_or((1, 1));
    let key = if message.contains("duplicate key") {
        span.map(|span| extract_key_at(src, span))
    } else {
        None
    };
    CatalogError::Parse {
        message,
        line,
        column,
        key,
    }
}

/// Read the (possibly quoted) key token at `span` by scanning forward to
/// the `=`.
fn extract_key_at(src: &str, span: std::ops::Range<usize>) -> String {
    let start = span.start.min(src.len());
    let end = src[start..]
        .find('=')
        .map(|offset| start + offset)
        .unwrap_or(span.end.min(src.len()));
    src[start..end].trim().trim_matches('"').to_string()
}

/// Map a message-format error to a file line/column by locating the
/// quoted key in the source, then adding the message-relative position.
fn catalog_message_error(
    src: &str,
    raw_key: &str,
    value: &str,
    error: &MessageError,
) -> CatalogError {
    let (value_line, value_column) = line_col(value, error.offset());
    let needle = format!("\"{raw_key}\"");
    let file_line = src.find(&needle).map(|byte| line_col(src, byte).0);
    let (line, column) = match file_line {
        Some(key_line) => (key_line + value_line - 1, value_column),
        None => (value_line, value_column),
    };
    CatalogError::MessageFormat {
        key: raw_key.to_string(),
        message: error.to_string(),
        line,
        column,
    }
}

/// Load a single catalog file; the file stem must be the BCP-47 tag
/// (doc 07 §6.3: "file name = BCP-47 tag").
pub fn load_catalog_file(path: &Path) -> Result<Catalog, CatalogError> {
    let source_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let tag = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let locale = Locale::parse(&tag).map_err(|error| CatalogError::InvalidLocaleTag {
        found: tag.clone(),
        reason: error.to_string(),
    })?;
    let src = std::fs::read_to_string(path).map_err(|source| CatalogError::Io {
        context: format!("reading {source_name}"),
        source,
    })?;
    Catalog::from_toml_str(locale, &source_name, &src)
}

/// The outcome of scanning a `locales/` directory.
#[derive(Debug, Default)]
pub struct LoadedCatalogs {
    /// Successfully loaded catalogs, sorted by locale tag.
    pub catalogs: Vec<Catalog>,
    /// Files that failed to load, with their errors.
    pub failures: Vec<(String, CatalogError)>,
}

/// Load every `*.toml` file directly inside `dir`. Non-TOML files are
/// skipped (documented: the directory holds catalogs only).
pub fn load_locales_dir(dir: &Path) -> LoadedCatalogs {
    let mut loaded = LoadedCatalogs::default();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) => {
            loaded.failures.push((
                dir.to_string_lossy().into_owned(),
                CatalogError::Io {
                    context: format!("listing locales directory {}", dir.display()),
                    source,
                },
            ));
            return loaded;
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(_) => None,
        })
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.to_string_lossy() == "toml")
        })
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        match load_catalog_file(&path) {
            Ok(catalog) => loaded.catalogs.push(catalog),
            Err(error) => loaded.failures.push((name, error)),
        }
    }
    loaded.catalogs.sort_by(|a, b| a.locale().cmp(b.locale()));
    loaded
}

#[cfg(test)]
mod tests {
    use super::{Catalog, CatalogError, REFERENCE_LOCALE, load_locales_dir};
    use crate::key::MessageKey;
    use crate::locale::Locale;
    use std::collections::BTreeMap;

    fn en_us() -> Result<Locale, Box<dyn std::error::Error>> {
        Ok(Locale::parse(REFERENCE_LOCALE)?)
    }

    #[test]
    fn parses_flat_catalog() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\
\"player.controls.play\" = \"Play\"
\"player.status.buffering\" = \"Buffering… {percent, number, percent}\"
";
        let catalog = Catalog::from_toml_str(en_us()?, "en-US.toml", src)?;
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.source_name(), "en-US.toml");
        let keys: Vec<&str> = catalog.keys().map(|key| key.as_str()).collect();
        assert_eq!(keys, ["player.controls.play", "player.status.buffering"]);
        let key = MessageKey::new("player.controls.play")?;
        assert!(catalog.contains_key(&key));
        Ok(())
    }

    #[test]
    fn duplicate_keys_are_parse_errors_with_position() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\
\"a.b\" = \"one\"
\"a.c\" = \"two\"
\"a.b\" = \"three\"
";
        let error = Catalog::from_toml_str(en_us()?, "dup.toml", src).err();
        assert!(
            matches!(
                &error,
                Some(CatalogError::Parse { message, line: 3, column: 1, key })
                    if message.contains("duplicate key") && key.as_deref() == Some("a.b"),
            ),
            "expected duplicate-key parse error, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn non_string_values_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\"a.b\" = 3\n";
        let error = Catalog::from_toml_str(en_us()?, "bad.toml", src).err();
        assert!(
            matches!(&error, Some(CatalogError::NonStringValue { key, found: "integer" }) if key == "a.b"),
            "expected non-string-value error, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn nested_tables_are_rejected_with_hint() -> Result<(), Box<dyn std::error::Error>> {
        let src = "[a]\nb = \"x\"\n";
        let error = Catalog::from_toml_str(en_us()?, "bad.toml", src).err();
        assert!(
            matches!(&error, Some(CatalogError::NestedTable { key }) if key == "a"),
            "expected nested-table error, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn invalid_keys_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\"Player.Play\" = \"x\"\n";
        let error = Catalog::from_toml_str(en_us()?, "bad.toml", src).err();
        assert!(
            matches!(&error, Some(CatalogError::InvalidKey { key, reason })
                if key == "Player.Play" && reason.contains("kebab-case")),
            "expected invalid-key error, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn message_errors_carry_file_position() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\"a.b\" = \"x {oops\"\n";
        let error = Catalog::from_toml_str(en_us()?, "bad.toml", src).err();
        assert!(
            matches!(&error, Some(CatalogError::MessageFormat { key, line: 1, .. }) if key == "a.b"),
            "expected message-format error, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn empty_catalog_is_valid() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = Catalog::from_toml_str(en_us()?, "empty.toml", "# nothing\n")?;
        assert!(catalog.is_empty());
        Ok(())
    }

    #[test]
    fn loads_a_locales_directory() -> Result<(), Box<dyn std::error::Error>> {
        // Uses the repo's real seed catalog — also asserted in the
        // integration tests; here it proves the directory loader.
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../locales");
        let loaded = load_locales_dir(std::path::Path::new(dir));
        assert!(
            loaded.failures.is_empty(),
            "failures: {:?}",
            loaded.failures
        );
        assert_eq!(loaded.catalogs.len(), 1);
        assert_eq!(loaded.catalogs[0].locale().as_str(), "en-US");
        Ok(())
    }

    #[test]
    fn placeholder_set_shape() -> Result<(), Box<dyn std::error::Error>> {
        let src = "\"a.b\" = \"Hello {name}! {p, number, percent}\"\n";
        let catalog = Catalog::from_toml_str(en_us()?, "x.toml", src)?;
        let key = MessageKey::new("a.b")?;
        let message = catalog.get(&key);
        assert!(message.is_some());
        let expected: BTreeMap<String, &str> = [
            ("name".to_string(), "simple"),
            ("p".to_string(), "number,percent"),
        ]
        .into_iter()
        .collect();
        assert_eq!(message.map(|m| m.placeholder_set()), Some(expected));
        Ok(())
    }
}
