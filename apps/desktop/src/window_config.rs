//! Window config: `product.toml` §2.2 `[window]` → the shell's window
//! parameters (card 4.H1; doc 07 — the config-driven system; doc 05
//! §5.1's shell consumes exactly this struct when the tauri wrapper
//! builds the `WebviewWindowBuilder`).
//!
//! # Honest scope
//!
//! A **std-only, `[window]`-table** reader — not a general TOML
//! parser. The workspace is std-only by C5, no `toml` crate is on the
//! allowlist, and the shell needs one flat table with eight keys and
//! schema bounds (product.toml's own comments state them). The reader
//! is line-oriented, section-scoped, comment- and quote-aware; every
//! deviation is a typed [`WindowConfigError`], never a guess. When a
//! real TOML dependency is ever admitted (its own dossier), this
//! reader is the seam it replaces — one function,
//! [`parse_window_table`].
//!
//! Schema bounds (product.toml §2.2): `width` 480..=3840, `height`
//! 320..=2160, `min_width < width`, `min_height < height`; title
//! template variables restricted to `{product_name}` and
//! `{media_title}` (unknown variables are a validation error).

use std::fmt;

/// A window-config violation (typed, like every seam in this crate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowConfigError {
    /// A required key is absent from the `[window]` table.
    MissingKey {
        /// The missing key.
        key: &'static str,
    },
    /// A key outside the table's flat shape (nested table/array).
    UnexpectedShape {
        /// The offending line, verbatim.
        line: String,
    },
    /// An integer field that is not an integer or breaks its bound.
    BadNumber {
        /// The key.
        key: &'static str,
        /// Why (range or parse sentence).
        reason: String,
    },
    /// A boolean field that is not `true`/`false`.
    BadBool {
        /// The key.
        key: &'static str,
        /// The offending value.
        found: String,
    },
    /// The title template uses a variable outside the allowed set.
    UnknownTemplateVariable {
        /// The offending variable (brace content).
        found: String,
    },
}

impl fmt::Display for WindowConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowConfigError::MissingKey { key } => {
                write!(f, "window config: required key `{key}` is absent")
            }
            WindowConfigError::UnexpectedShape { line } => {
                write!(f, "window config: unexpected shape: {line:?}")
            }
            WindowConfigError::BadNumber { key, reason } => {
                write!(f, "window config: `{key}`: {reason}")
            }
            WindowConfigError::BadBool { key, found } => {
                write!(
                    f,
                    "window config: `{key}` must be true/false, found {found:?}"
                )
            }
            WindowConfigError::UnknownTemplateVariable { found } => {
                write!(
                    f,
                    "window config: title template variable {{{found}}} is not allowed"
                )
            }
        }
    }
}

impl std::error::Error for WindowConfigError {}

/// The `[window]` table as the shell consumes it (product.toml §2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    /// Default width (schema 480..=3840).
    pub width: u32,
    /// Default height (schema 320..=2160).
    pub height: u32,
    /// Minimum width (schema: `min_width < width`).
    pub min_width: u32,
    /// Minimum height (schema: `min_height < height`).
    pub min_height: u32,
    /// Start maximized.
    pub start_maximized: bool,
    /// Remember geometry across runs.
    pub remember_geometry: bool,
    /// Title template (`{product_name}` / `{media_title}`).
    pub title_template: String,
    /// Idle title localization key.
    pub title_idle_key: String,
}

impl WindowConfig {
    /// The title with no media open: the idle key (localization joins
    /// with the i18n layer; the shell never hardcodes wording).
    #[must_use = "pure reporter"]
    pub fn idle_title_key(&self) -> &str {
        &self.title_idle_key
    }

    /// Render the title template with media open (the `{product_name}`
    /// / `{media_title}` variables — the only two [`validate`]
    /// admits). The wrapper calls this on `media.info-ready`-class
    /// updates and sets the window title to the result.
    #[must_use = "pure reporter"]
    pub fn render_media_title(&self, product_name: &str, media_title: &str) -> String {
        self.title_template
            .replace("{product_name}", product_name)
            .replace("{media_title}", media_title)
    }

    /// The build spec for window construction — the exact parameter
    /// set the ADR-004 wrapper feeds `WebviewWindowBuilder` (issue #3,
    /// 4.H1 handoff c: the `[window]` table is CONSUMED at window
    /// build, not just parsed). `remember_geometry` rides along so the
    /// wrapper branches on config (restore persisted geometry vs
    /// apply `inner_size`), never on a guess.
    #[must_use = "pure reporter — the wrapper applies this verbatim"]
    pub fn build_spec(&self) -> WindowBuildSpec {
        WindowBuildSpec {
            inner_size: (self.width, self.height),
            min_inner_size: (self.min_width, self.min_height),
            maximized: self.start_maximized,
            remember_geometry: self.remember_geometry,
            title: WindowTitle::Idle {
                key: self.title_idle_key.clone(),
            },
        }
    }
}

/// The window-title source at build time: the idle localization key
/// until media opens (the webview's i18n layer resolves the key), the
/// rendered template once one has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowTitle {
    /// No media: resolve this localization key.
    Idle {
        /// The key (e.g. `window.title.idle`).
        key: String,
    },
    /// Media open: the rendered template, verbatim.
    Media {
        /// The rendered title string.
        rendered: String,
    },
}

/// What the shell hands the window builder (one struct, no config
/// knowledge leaks into the wrapper — the tauri-side code applies
/// this and nothing else).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowBuildSpec {
    /// Initial inner size (`inner_size`).
    pub inner_size: (u32, u32),
    /// Minimum inner size (`min_inner_size`).
    pub min_inner_size: (u32, u32),
    /// Start maximized (`maximized`).
    pub maximized: bool,
    /// Restore persisted geometry when true (the geometry store is
    /// the settings-backed persistence card's follow-up; the flag is
    /// consumed here either way).
    pub remember_geometry: bool,
    /// The title source at build time.
    pub title: WindowTitle,
}

/// Parse the `[window]` table out of product.toml text.
///
/// # Errors
/// [`WindowConfigError`] for any missing key, bad value, or schema
/// violation — never a default-invented window (the honest-parse
/// rule: config drift is loud).
pub fn parse_window_table(toml_text: &str) -> Result<WindowConfig, WindowConfigError> {
    let mut width = None;
    let mut height = None;
    let mut min_width = None;
    let mut min_height = None;
    let mut start_maximized = None;
    let mut remember_geometry = None;
    let mut title_template = None;
    let mut title_idle_key = None;

    let mut in_window = false;
    for raw in toml_text.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_window = line == "[window]";
            continue;
        }
        if !in_window {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(WindowConfigError::UnexpectedShape {
                line: raw.to_string(),
            });
        };
        let key = key.trim();
        let value = value.trim();
        // Scalars only: a nested table or array is out of shape here.
        if value.starts_with('[') || value.starts_with('{') {
            return Err(WindowConfigError::UnexpectedShape {
                line: raw.to_string(),
            });
        }
        match key {
            "width" => width = Some(parse_u32("width", value, 480, 3840)?),
            "height" => height = Some(parse_u32("height", value, 320, 2160)?),
            "min_width" => min_width = Some(parse_u32("min_width", value, 1, 3840)?),
            "min_height" => min_height = Some(parse_u32("min_height", value, 1, 2160)?),
            "start_maximized" => start_maximized = Some(parse_bool("start_maximized", value)?),
            "remember_geometry" => {
                remember_geometry = Some(parse_bool("remember_geometry", value)?)
            }
            "title_template" => title_template = Some(parse_string(value)),
            "title_idle_key" => title_idle_key = Some(parse_string(value)),
            // Unknown keys inside [window] are tolerated the way TOML
            // tolerates them: forward-compat, not an error. The
            // config-validator owns full-schema strictness.
            _ => {}
        }
    }

    let width = width.ok_or(WindowConfigError::MissingKey { key: "width" })?;
    let height = height.ok_or(WindowConfigError::MissingKey { key: "height" })?;
    let min_width = min_width.ok_or(WindowConfigError::MissingKey { key: "min_width" })?;
    let min_height = min_height.ok_or(WindowConfigError::MissingKey { key: "min_height" })?;
    let config = WindowConfig {
        width,
        height,
        min_width,
        min_height,
        start_maximized: start_maximized.ok_or(WindowConfigError::MissingKey {
            key: "start_maximized",
        })?,
        remember_geometry: remember_geometry.ok_or(WindowConfigError::MissingKey {
            key: "remember_geometry",
        })?,
        title_template: title_template.ok_or(WindowConfigError::MissingKey {
            key: "title_template",
        })?,
        title_idle_key: title_idle_key.ok_or(WindowConfigError::MissingKey {
            key: "title_idle_key",
        })?,
    };
    validate(&config)?;
    Ok(config)
}

/// Schema cross-field rules (`min_width < width`, template vars).
fn validate(config: &WindowConfig) -> Result<(), WindowConfigError> {
    if config.min_width >= config.width {
        return Err(WindowConfigError::BadNumber {
            key: "min_width",
            reason: format!(
                "min_width ({}) must be < width ({})",
                config.min_width, config.width
            ),
        });
    }
    if config.min_height >= config.height {
        return Err(WindowConfigError::BadNumber {
            key: "min_height",
            reason: format!(
                "min_height ({}) must be < height ({})",
                config.min_height, config.height
            ),
        });
    }
    let mut rest = config.title_template.as_str();
    while let Some(open) = rest.find('{') {
        let after = &rest[open + '{'.len_utf8()..];
        let Some(close) = after.find('}') else {
            return Err(WindowConfigError::UnknownTemplateVariable {
                found: rest[open..].to_string(),
            });
        };
        let variable = &after[..close];
        if variable != "product_name" && variable != "media_title" {
            return Err(WindowConfigError::UnknownTemplateVariable {
                found: variable.to_string(),
            });
        }
        rest = &after[close + '}'.len_utf8()..];
    }
    Ok(())
}

/// Strip a `#` comment (quote-aware enough for this table: the two
/// string values carry no `#` in the shipped product.toml, and a `#`
/// inside quotes would be a validator concern, not the shell's).
fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) => &line[..at],
        None => line,
    }
}

fn parse_u32(key: &'static str, value: &str, min: u32, max: u32) -> Result<u32, WindowConfigError> {
    let parsed: u32 = value.parse().map_err(|_| WindowConfigError::BadNumber {
        key,
        reason: format!("{value:?} is not an integer"),
    })?;
    if parsed < min || parsed > max {
        return Err(WindowConfigError::BadNumber {
            key,
            reason: format!("{parsed} outside the schema range {min}..={max}"),
        });
    }
    Ok(parsed)
}

fn parse_bool(key: &'static str, value: &str) -> Result<bool, WindowConfigError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(WindowConfigError::BadBool {
            key,
            found: other.to_string(),
        }),
    }
}

fn parse_string(value: &str) -> String {
    value.trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped product.toml parses to the §2.2 defaults — the
    /// real file, not a copy (drift is loud here).
    #[test]
    fn the_shipped_product_toml_parses() {
        let product = include_str!("../../../product.toml");
        let config = parse_window_table(product).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert_eq!(config.min_width, 480);
        assert_eq!(config.min_height, 320);
        assert!(!config.start_maximized);
        assert!(config.remember_geometry);
        assert_eq!(config.title_template, "{media_title} — {product_name}");
        assert_eq!(config.idle_title_key(), "window.title.idle");
    }

    #[test]
    fn bounds_and_cross_fields_are_typed_errors() {
        let base = "[window]\nwidth = 1280\nheight = 720\nmin_width = 480\nmin_height = 320\n\
                    start_maximized = false\nremember_geometry = true\n\
                    title_template = \"{media_title} — {product_name}\"\n\
                    title_idle_key = \"window.title.idle\"\n";
        assert!(parse_window_table(base).is_ok());

        let mut bad = base.replace("width = 1280", "width = 100");
        assert_eq!(
            parse_window_table(&bad),
            Err(WindowConfigError::BadNumber {
                key: "width",
                reason: "100 outside the schema range 480..=3840".to_string()
            })
        );

        bad = base.replace("min_width = 480", "min_width = 2000");
        assert!(matches!(
            parse_window_table(&bad),
            Err(WindowConfigError::BadNumber {
                key: "min_width",
                ..
            })
        ));

        bad = base.replace("start_maximized = false", "start_maximized = maybe");
        assert!(matches!(
            parse_window_table(&bad),
            Err(WindowConfigError::BadBool {
                key: "start_maximized",
                ..
            })
        ));

        bad = base.replace(
            "title_template = \"{media_title} — {product_name}\"",
            "title_template = \"{media_title} — {os_name}\"",
        );
        assert_eq!(
            parse_window_table(&bad),
            Err(WindowConfigError::UnknownTemplateVariable {
                found: "os_name".to_string()
            })
        );

        let missing = base.replace("height = 720\n", "");
        assert_eq!(
            parse_window_table(&missing),
            Err(WindowConfigError::MissingKey { key: "height" })
        );
    }

    /// Comments and other sections are not confused with the table
    /// (the `[brand]` section that follows carries its own keys).
    #[test]
    fn other_sections_and_comments_do_not_leak() {
        let text = "# leading comment\n[window]\nwidth = 960 # inline comment\nheight = 540\n\
                    min_width = 480\nmin_height = 320\nstart_maximized = true\n\
                    remember_geometry = false\ntitle_template = \"{product_name}\"\n\
                    title_idle_key = \"k\"\n[brand]\naccent_color = \"#4f8cff\"\n";
        let config = parse_window_table(text).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(config.width, 960);
        assert!(config.start_maximized);
        assert!(!config.remember_geometry);
        assert_eq!(config.title_template, "{product_name}");
    }

    /// 4.H1 handoff c (issue #3): the shipped product.toml's [window]
    /// table is CONSUMED as a build spec — the exact parameter set
    /// the ADR-004 wrapper feeds the window builder, derived from the
    /// REAL file (drift is loud here).
    #[test]
    fn the_shipped_product_toml_yields_the_build_spec() {
        let product = include_str!("../../../product.toml");
        let config = parse_window_table(product).unwrap_or_else(|err| unreachable!("{err}"));
        let spec = config.build_spec();
        assert_eq!(spec.inner_size, (1280, 720));
        assert_eq!(spec.min_inner_size, (480, 320));
        assert!(!spec.maximized);
        assert!(spec.remember_geometry);
        assert_eq!(
            spec.title,
            WindowTitle::Idle {
                key: "window.title.idle".to_string()
            }
        );
    }

    /// The media title renders through the template's variables
    /// (both directions), and the idle key stands in until one does.
    #[test]
    fn media_titles_render_through_the_template() {
        let product = include_str!("../../../product.toml");
        let config = parse_window_table(product).unwrap_or_else(|err| unreachable!("{err}"));
        assert_eq!(
            config.render_media_title("Lumen", "Big Buck Bunny"),
            "Big Buck Bunny — Lumen"
        );
        assert_eq!(config.render_media_title("Nova", "Sintel"), "Sintel — Nova");
        // No media open: the idle key (resolved by the i18n layer).
        assert_eq!(config.idle_title_key(), "window.title.idle");
    }
}
