//! Resolved token model (card 1.B4) — the shape [`crate::parse`] produces and
//! the emitters consume. Doc 01 §2.3 interface sketch: `TokenSet` carries the
//! semantic + primitive tokens (plus the alias layer, needed for the
//! semantic-only consumption lint, doc 08 §3.2).

use crate::color::{Rgb8, Shadow};

/// A fully resolved token value.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedValue {
    /// A color (`$type: color`).
    Color(Rgb8),
    /// A px dimension (`$type: dimension`).
    Dimension(f64),
    /// A bare number (`$type: number` / `fontWeight`).
    Number(f64),
    /// A drop shadow (`$type: shadow`).
    Shadow(Shadow),
    /// A font family stack (`$type: fontFamily`).
    Family(Vec<String>),
}

impl ResolvedValue {
    /// The color value, if this token is a color.
    pub fn as_color(&self) -> Option<Rgb8> {
        match self {
            ResolvedValue::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// The numeric value, if this token is a dimension/number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ResolvedValue::Dimension(v) | ResolvedValue::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// The shadow value, if this token is a shadow.
    pub fn as_shadow(&self) -> Option<Shadow> {
        match self {
            ResolvedValue::Shadow(s) => Some(*s),
            _ => None,
        }
    }

    /// The family stack, if this token is a fontFamily.
    pub fn as_family(&self) -> Option<&[String]> {
        match self {
            ResolvedValue::Family(f) => Some(f),
            _ => None,
        }
    }
}

/// Values of a token across modes: one value, or one per registered mode
/// (doc 08 §3.1 per-mode values).
#[derive(Clone, Debug, PartialEq)]
pub enum ModeValues {
    /// Same value in every mode.
    Single(ResolvedValue),
    /// One value per mode; coverage of all registered modes is validated at
    /// parse time (doc 08 §3.4).
    PerMode(Vec<(String, ResolvedValue)>),
}

impl ModeValues {
    /// The value for `mode` (`None` = any/modeless context). `None` if the
    /// token has no entry for that mode (prevented by validation).
    pub fn value_for(&self, mode: Option<&str>) -> Option<&ResolvedValue> {
        match self {
            ModeValues::Single(v) => Some(v),
            ModeValues::PerMode(pairs) => {
                let mode = mode?;
                pairs.iter().find(|(m, _)| m == mode).map(|(_, v)| v)
            }
        }
    }

    /// Whether this token carries per-mode values.
    pub fn is_per_mode(&self) -> bool {
        matches!(self, ModeValues::PerMode(_))
    }
}

/// One semantic-layer token (the only layer components may consume,
/// doc 08 §3.2).
#[derive(Clone, Debug)]
pub struct SemanticToken {
    /// Dotted path inside the `semantic` group, e.g. `color.surface.deep`.
    pub path: String,
    /// CSS custom-property name, e.g. `--color-surface-deep`.
    pub css_name: String,
    /// Rust constant name, e.g. `COLOR_SURFACE_DEEP`.
    pub rust_name: String,
    /// Top category segment (`color`, `font`, `radius`, …).
    pub category: String,
    /// Resolved values (single or per-mode).
    pub values: ModeValues,
}

/// Name-form metadata for one non-semantic token (lint universe input).
#[derive(Clone, Debug)]
pub struct LayerToken {
    /// Dotted path with the layer root stripped, e.g. `color.gray.900`
    /// (primitive) or `color.surface.deep` (alias).
    pub name: String,
    /// `primitive` or `alias`.
    pub layer: &'static str,
    /// The resolved value(s) of this token — the raw-value half of the
    /// semantic-only lint (doc 08 §3.2) needs them to recognize raw `#hex`
    /// literals that bypass the semantic layer.
    pub values: ModeValues,
}

/// Layer counts for reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenCounts {
    /// Primitive-layer token count.
    pub primitives: usize,
    /// Alias-layer token count.
    pub aliases: usize,
    /// Semantic-layer token count.
    pub semantic: usize,
}

/// The parsed, resolved, validated token set (doc 01 §2.3 `TokenSet`).
#[derive(Clone, Debug)]
pub struct TokenSet {
    pub(crate) modes: Vec<String>,
    pub(crate) default_mode: String,
    pub(crate) accents: Vec<String>,
    pub(crate) default_accent: String,
    pub(crate) non_semantic: Vec<LayerToken>,
    pub(crate) semantic: Vec<SemanticToken>,
    pub(crate) ramps: Vec<crate::color::AccentRamp>,
    /// Semantic CSS names that switch per accent (doc 08 §3.3).
    pub(crate) accent_family_css: Vec<String>,
}

impl TokenSet {
    /// Registered modes (`dark`, `light`).
    pub fn modes(&self) -> &[String] {
        &self.modes
    }

    /// The default mode (`dark` per doc 08 §3.3).
    pub fn default_mode(&self) -> &str {
        &self.default_mode
    }

    /// Registered accent ids in `data-accent` order (doc 08 §3.3).
    pub fn accents(&self) -> &[String] {
        &self.accents
    }

    /// The default accent (`orange`).
    pub fn default_accent(&self) -> &str {
        &self.default_accent
    }

    /// All semantic tokens, in token-file order (emission order).
    pub fn semantic(&self) -> &[SemanticToken] {
        &self.semantic
    }

    /// Look up a semantic token by CSS name (`--color-surface`).
    pub fn semantic_by_css_name(&self, css_name: &str) -> Option<&SemanticToken> {
        self.semantic.iter().find(|t| t.css_name == css_name)
    }

    /// Primitive + alias name forms (layer root stripped) — the lint universe.
    pub fn non_semantic(&self) -> &[LayerToken] {
        &self.non_semantic
    }

    /// All accent ramps, in declared `data-accent` order.
    pub fn accent_ramps(&self) -> &[crate::color::AccentRamp] {
        &self.ramps
    }

    /// Semantic CSS names that are overridden per accent.
    pub fn accent_family_css(&self) -> &[String] {
        &self.accent_family_css
    }

    /// Per-layer token counts.
    pub fn counts(&self) -> TokenCounts {
        TokenCounts {
            primitives: self
                .non_semantic
                .iter()
                .filter(|t| t.layer == "primitive")
                .count(),
            aliases: self
                .non_semantic
                .iter()
                .filter(|t| t.layer == "alias")
                .count(),
            semantic: self.semantic.len(),
        }
    }

    /// The ramp for one accent id.
    pub fn ramp_for(&self, accent: &str) -> Option<&crate::color::AccentRamp> {
        self.ramps.iter().find(|r| r.id == accent)
    }
}
