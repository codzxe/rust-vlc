//! The doc 07 §6.2 message-format **v1 subset**: parsing, placeholder
//! contracts, and panic-free rendering.
//!
//! Supported forms (nothing else — the exact feature set is deliberately
//! minimal until the Phase-4 engine decision `[PROPOSAL, doc 07 §6.2]`):
//!
//! ```text
//! {name}                       simple placeholder
//! {name, number, percent}      percentage; fraction input, 0.45 → "45%"
//! {name, select, a {…} other {…}}   string choice; `other` arm required
//! {name, plural, one {…} other {…}} count choice; `other` arm required
//! ```
//!
//! Placeholder names are `[A-Za-z_][A-Za-z0-9_]*` (snake_case by
//! convention) and are part of each key's contract (rule 3 compares them
//! across catalogs). `{{` / `}}` escape literal braces **at the top level
//! of a message**; inside `select`/`plural` arms a `}` always closes the
//! arm (ICU semantics — `}}` there is arm-close + placeholder-close, so
//! literal braces inside arms are a v1 limitation). Inside plural arms
//! only, `#` renders the count. Rendering never panics: every problem
//! (missing argument, type mismatch, missing plural arm) records a
//! [`RenderDiagnostic`] and degrades visibly.

use std::collections::BTreeMap;
use std::fmt;

use crate::locale::Locale;
use crate::plural::{PluralCategory, select_plural_category};

/// A render-time argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// A string value (for `{name, select, …}` and simple placeholders).
    Str(String),
    /// An integer count (for `{name, plural, …}` and simple placeholders).
    Int(i64),
    /// A float (for `{name, number, percent}` and simple placeholders).
    Float(f64),
}

impl Arg {
    /// Plain-text form: strings verbatim, integers as-is, floats in
    /// shortest round-trip form with integral values un-suffixed.
    fn to_display(&self) -> String {
        match self {
            Arg::Str(s) => s.clone(),
            Arg::Int(i) => i.to_string(),
            Arg::Float(f) => format_float(*f),
        }
    }
}

impl From<&str> for Arg {
    fn from(value: &str) -> Self {
        Arg::Str(value.to_string())
    }
}

impl From<String> for Arg {
    fn from(value: String) -> Self {
        Arg::Str(value)
    }
}

impl From<i64> for Arg {
    fn from(value: i64) -> Self {
        Arg::Int(value)
    }
}

impl From<f64> for Arg {
    fn from(value: f64) -> Self {
        Arg::Float(value)
    }
}

/// Shortest round-trip form; integral floats print without a fraction.
fn format_float(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// ICU-consistent percentage: multiply by 100, round to two decimals
/// (absorbing binary multiplication noise), trim trailing zeros.
fn format_percent(value: f64) -> String {
    let rounded = (value * 100.0 * 100.0).round() / 100.0;
    let mut text = format!("{rounded:.2}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text.push('%');
    text
}

/// One placeholder's type — the other half of the rule-3 contract
/// (name + type must match the reference catalog exactly).
#[derive(Debug, Clone, PartialEq)]
pub enum PlaceholderKind {
    /// `{name}`
    Simple,
    /// `{name, number, percent}`
    NumberPercent,
    /// `{name, select, …}` with its named arms.
    Select(Vec<(String, Message)>),
    /// `{name, plural, …}` with its category arms.
    Plural(Vec<(PluralCategory, Message)>),
}

impl PlaceholderKind {
    /// Stable tag used in placeholder-set comparison and diagnostics.
    pub fn tag(&self) -> &'static str {
        match self {
            PlaceholderKind::Simple => "simple",
            PlaceholderKind::NumberPercent => "number,percent",
            PlaceholderKind::Select(_) => "select",
            PlaceholderKind::Plural(_) => "plural",
        }
    }
}

impl fmt::Display for PlaceholderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Text(String),
    Placeholder {
        name: String,
        kind: PlaceholderKind,
    },
    /// `#` inside a plural arm — renders the plural count.
    CountSelf,
}

/// A parsed message (doc 07 §6.2 v1 subset).
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    parts: Vec<Part>,
}

/// Ways a message can violate the v1 format. Variants carry the byte
/// `offset` of the offending token within the message source so callers
/// can map them to file positions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    /// A `{` was never closed.
    #[error("unterminated placeholder (missing `}}`)")]
    Unterminated { offset: usize },
    /// A `}` appeared without a matching `{`.
    #[error("unmatched `}}`")]
    UnmatchedClose { offset: usize },
    /// `{` not followed by a placeholder name.
    #[error("missing placeholder name")]
    MissingName { offset: usize },
    /// The placeholder name contains characters outside
    /// `[A-Za-z_][A-Za-z0-9_]*`.
    #[error("invalid placeholder name character {found:?} (names are [A-Za-z_][A-Za-z0-9_]*)")]
    InvalidName { offset: usize, found: char },
    /// Expected `,` or `}` after the placeholder name.
    #[error("expected `,` or `}}` after placeholder name, found {found:?}")]
    ExpectedCommaOrClose { offset: usize, found: char },
    /// The placeholder type keyword is not one of `number`, `select`,
    /// `plural`.
    #[error("unknown placeholder type {found:?} (v1: number/select/plural)")]
    UnknownType { offset: usize, found: String },
    /// `{name, number, …}` with a style other than `percent` (including a
    /// missing style — the v1 subset has no bare `number` form).
    #[error("unsupported number style {found:?} (v1 supports only `percent`)")]
    UnsupportedNumberStyle { offset: usize, found: String },
    /// A plural arm name is not a CLDR cardinal category.
    #[error("invalid plural category {found:?} (zero/one/two/few/many/other)")]
    InvalidCategory { offset: usize, found: String },
    /// A `select`/`plural` without the mandatory `other` arm.
    #[error("select/plural placeholder is missing the mandatory `other` arm")]
    MissingOtherArm { offset: usize },
    /// The same option/category name appears twice.
    #[error("duplicate {kind} arm {found:?}")]
    DuplicateArm {
        offset: usize,
        kind: &'static str,
        found: String,
    },
    /// The same placeholder name is used with two different types.
    #[error("placeholder {name:?} is used with conflicting types ({first} vs {second})")]
    ConflictingPlaceholderType {
        name: String,
        first: &'static str,
        second: &'static str,
    },
}

impl MessageError {
    /// Byte offset of the offending token within the message source.
    pub fn offset(&self) -> usize {
        match self {
            MessageError::Unterminated { offset }
            | MessageError::UnmatchedClose { offset }
            | MessageError::MissingName { offset }
            | MessageError::InvalidName { offset, .. }
            | MessageError::ExpectedCommaOrClose { offset, .. }
            | MessageError::UnknownType { offset, .. }
            | MessageError::UnsupportedNumberStyle { offset, .. }
            | MessageError::InvalidCategory { offset, .. }
            | MessageError::MissingOtherArm { offset }
            | MessageError::DuplicateArm { offset, .. } => *offset,
            MessageError::ConflictingPlaceholderType { .. } => 0,
        }
    }
}

/// A non-fatal problem encountered while rendering (never a panic —
/// doc 07 §6.3: a missing translation can never crash rendering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDiagnostic {
    /// The placeholder's argument was not supplied.
    ArgMissing { name: String },
    /// The argument's type does not fit the placeholder (e.g. a string
    /// passed to `{n, number, percent}`).
    ArgTypeMismatch {
        name: String,
        expected: &'static str,
    },
    /// The selected plural category has no arm in this message; the
    /// `other` arm was used instead (a non-conformant catalog at runtime —
    /// audit rule 5 should have caught it in CI).
    PluralCategoryMissing {
        name: String,
        category: PluralCategory,
    },
}

impl fmt::Display for RenderDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderDiagnostic::ArgMissing { name } => {
                write!(f, "missing argument {name:?}")
            }
            RenderDiagnostic::ArgTypeMismatch { name, expected } => {
                write!(
                    f,
                    "argument {name:?} has the wrong type (expected {expected})"
                )
            }
            RenderDiagnostic::PluralCategoryMissing { name, category } => {
                write!(
                    f,
                    "plural placeholder {name:?} has no {category} arm; rendered `other`"
                )
            }
        }
    }
}

/// The result of rendering a message.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    /// The rendered text (always present, possibly degraded).
    pub text: String,
    /// Non-fatal problems recorded during rendering.
    pub diagnostics: Vec<RenderDiagnostic>,
}

impl Message {
    /// Parse a message source string (the value side of a catalog entry).
    pub fn parse(src: &str) -> Result<Message, MessageError> {
        let mut parser = Parser { src, pos: 0 };
        let message = parser.parse_message(ParseContext::Top)?;
        if parser.pos < src.len() {
            // `parse_message` only stops before end-of-input at a `}`,
            // which therefore has no matching `{`.
            return Err(MessageError::UnmatchedClose { offset: parser.pos });
        }
        message.validate_placeholder_types()?;
        Ok(message)
    }

    /// The placeholder contract of this message: name → type tag, sorted.
    /// This is what audit rule 3 compares across catalogs (names + types
    /// must match the reference exactly).
    pub fn placeholder_set(&self) -> BTreeMap<String, &'static str> {
        let mut set = BTreeMap::new();
        self.collect_placeholders(&mut set);
        set
    }

    /// Every plural placeholder with its declared arm categories — the
    /// input to audit rule 5 (plural-rule conformance per locale).
    pub fn plural_arms_by_name(&self) -> Vec<(String, Vec<PluralCategory>)> {
        let mut out = Vec::new();
        self.collect_plural_arms(&mut out);
        out
    }

    fn collect_plural_arms(&self, out: &mut Vec<(String, Vec<PluralCategory>)>) {
        for part in &self.parts {
            if let Part::Placeholder { name, kind } = part {
                match kind {
                    PlaceholderKind::Plural(arms) => {
                        out.push((
                            name.clone(),
                            arms.iter().map(|(category, _)| *category).collect(),
                        ));
                        for (_, sub) in arms {
                            sub.collect_plural_arms(out);
                        }
                    }
                    PlaceholderKind::Select(arms) => {
                        for (_, sub) in arms {
                            sub.collect_plural_arms(out);
                        }
                    }
                    PlaceholderKind::Simple | PlaceholderKind::NumberPercent => {}
                }
            }
        }
    }

    fn collect_placeholders(&self, set: &mut BTreeMap<String, &'static str>) {
        for part in &self.parts {
            if let Part::Placeholder { name, kind } = part {
                set.insert(name.clone(), kind.tag());
                match kind {
                    PlaceholderKind::Select(arms) => {
                        for (_, sub) in arms {
                            sub.collect_placeholders(set);
                        }
                    }
                    PlaceholderKind::Plural(arms) => {
                        for (_, sub) in arms {
                            sub.collect_placeholders(set);
                        }
                    }
                    PlaceholderKind::Simple | PlaceholderKind::NumberPercent => {}
                }
            }
        }
    }

    /// One placeholder name must carry exactly one type per message.
    fn validate_placeholder_types(&self) -> Result<(), MessageError> {
        let mut seen: BTreeMap<&str, &'static str> = BTreeMap::new();
        let mut conflicts: Vec<(&str, &'static str, &'static str)> = Vec::new();
        self.collect_conflicts(&mut seen, &mut conflicts);
        if let Some((name, first, second)) = conflicts.into_iter().next() {
            return Err(MessageError::ConflictingPlaceholderType {
                name: name.to_string(),
                first,
                second,
            });
        }
        Ok(())
    }

    fn collect_conflicts<'a>(
        &'a self,
        seen: &mut BTreeMap<&'a str, &'static str>,
        conflicts: &mut Vec<(&'a str, &'static str, &'static str)>,
    ) {
        for part in &self.parts {
            if let Part::Placeholder { name, kind } = part {
                match seen.get(name.as_str()) {
                    Some(&existing) if existing != kind.tag() => {
                        conflicts.push((name.as_str(), existing, kind.tag()));
                    }
                    _ => {
                        seen.insert(name.as_str(), kind.tag());
                    }
                }
                match kind {
                    PlaceholderKind::Select(arms) => {
                        for (_, sub) in arms {
                            sub.collect_conflicts(seen, conflicts);
                        }
                    }
                    PlaceholderKind::Plural(arms) => {
                        for (_, sub) in arms {
                            sub.collect_conflicts(seen, conflicts);
                        }
                    }
                    PlaceholderKind::Simple | PlaceholderKind::NumberPercent => {}
                }
            }
        }
    }

    /// Render with `args` under `locale`. Never panics: problems are
    /// recorded as diagnostics and degrade visibly (a missing argument
    /// renders the literal `{name}` so the miss is obvious in output).
    pub fn render(&self, locale: &Locale, args: &[(&str, Arg)]) -> Rendered {
        let mut out = String::new();
        let mut diagnostics = Vec::new();
        self.render_into(&mut out, &mut diagnostics, locale, args, None);
        Rendered {
            text: out,
            diagnostics,
        }
    }

    fn render_into(
        &self,
        out: &mut String,
        diagnostics: &mut Vec<RenderDiagnostic>,
        locale: &Locale,
        args: &[(&str, Arg)],
        count: Option<&Arg>,
    ) {
        for part in &self.parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::CountSelf => match count {
                    Some(arg) => out.push_str(&arg.to_display()),
                    // Unreachable by construction: CountSelf only parses
                    // inside plural arms, which always pass a count.
                    None => out.push('#'),
                },
                Part::Placeholder { name, kind } => {
                    render_placeholder(name, kind, out, diagnostics, locale, args);
                }
            }
        }
    }
}

fn find_arg<'a>(args: &'a [(&str, Arg)], name: &str) -> Option<&'a Arg> {
    args.iter()
        .find(|(arg_name, _)| *arg_name == name)
        .map(|(_, value)| value)
}

fn render_placeholder(
    name: &str,
    kind: &PlaceholderKind,
    out: &mut String,
    diagnostics: &mut Vec<RenderDiagnostic>,
    locale: &Locale,
    args: &[(&str, Arg)],
) {
    let literal = format!("{{{name}}}");
    let report_missing = |out: &mut String, diagnostics: &mut Vec<RenderDiagnostic>| {
        diagnostics.push(RenderDiagnostic::ArgMissing {
            name: name.to_string(),
        });
        out.push_str(&literal);
    };
    let language = locale.as_str().split('-').next().unwrap_or_default();
    match kind {
        PlaceholderKind::Simple => match find_arg(args, name) {
            Some(arg) => out.push_str(&arg.to_display()),
            None => report_missing(out, diagnostics),
        },
        PlaceholderKind::NumberPercent => match find_arg(args, name) {
            // ICU semantics: the argument is a fraction; both integer and
            // float arguments are multiplied by 100.
            Some(Arg::Int(value)) => out.push_str(&format_percent(*value as f64)),
            Some(Arg::Float(value)) => out.push_str(&format_percent(*value)),
            Some(Arg::Str(_)) => {
                diagnostics.push(RenderDiagnostic::ArgTypeMismatch {
                    name: name.to_string(),
                    expected: "number",
                });
                out.push_str(&literal);
            }
            None => report_missing(out, diagnostics),
        },
        PlaceholderKind::Select(arms) => {
            let Some(Arg::Str(value)) = find_arg(args, name) else {
                report_missing(out, diagnostics);
                return;
            };
            // ICU semantics: unmatched values fall back to `other`
            // (always present — enforced at parse time).
            let chosen = arms
                .iter()
                .find(|(arm, _)| arm == value)
                .or_else(|| arms.iter().find(|(arm, _)| arm == "other"));
            if let Some((_, sub)) = chosen {
                sub.render_into(out, diagnostics, locale, args, None);
            } else {
                // Unreachable: parse enforces the `other` arm.
                out.push_str(&literal);
            }
        }
        PlaceholderKind::Plural(arms) => {
            let Some(arg) = find_arg(args, name) else {
                report_missing(out, diagnostics);
                return;
            };
            let category = match arg {
                Arg::Int(value) => select_plural_category(language, *value),
                // Integral floats behave as integer counts; fractional
                // counts select `other` (v1 simplification, documented in
                // `plural`).
                Arg::Float(value) if value.fract() == 0.0 => {
                    select_plural_category(language, *value as i64)
                }
                Arg::Float(_) => PluralCategory::Other,
                Arg::Str(_) => {
                    diagnostics.push(RenderDiagnostic::ArgTypeMismatch {
                        name: name.to_string(),
                        expected: "integer count",
                    });
                    out.push_str(&literal);
                    return;
                }
            };
            let chosen = arms
                .iter()
                .find(|(arm, _)| *arm == category)
                .or_else(|| arms.iter().find(|(arm, _)| *arm == PluralCategory::Other));
            match chosen {
                Some((_, sub)) => sub.render_into(out, diagnostics, locale, args, Some(arg)),
                None => {
                    // Unreachable: parse enforces the `other` arm.
                    diagnostics.push(RenderDiagnostic::PluralCategoryMissing {
                        name: name.to_string(),
                        category,
                    });
                    out.push_str(&literal);
                }
            }
        }
    }
}

/// Grammar of `select`/`plural` arm names.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ArmGrammar {
    /// `select`: free-form option identifiers (`known`, `unknown`, …).
    Option,
    /// `plural`: CLDR cardinal categories (`one`, `few`, …).
    Category,
}

/// Where [`Parser::parse_message`] is running. Brace escapes (`{{`, `}}`)
/// are honored only at the top level: inside an arm, `}` always closes the
/// arm (so `…}}` at an arm boundary is arm-close + placeholder-close, not
/// an escape — ICU semantics). `#` is special only inside plural arms.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseContext {
    /// The top level of a message (catalog value).
    Top,
    /// Inside a `select` arm.
    SelectArm,
    /// Inside a `plural` arm.
    PluralArm,
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn starts_with(&self, token: &str) -> bool {
        self.src[self.pos..].starts_with(token)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }

    /// Parse an identifier `[A-Za-z_][A-Za-z0-9_]*`. Returns `None` at
    /// end of input or a non-identifier character (position preserved).
    fn parse_ident(&mut self) -> Option<String> {
        let start = self.pos;
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => self.pos += c.len_utf8(),
            _ => return None,
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        Some(self.src[start..self.pos].to_string())
    }

    /// Parse a run of parts until end of input or an unconsumed `}`.
    /// `ctx` selects the level: brace escapes exist only at the top level
    /// (inside arms a `}` always closes the arm), and `#` (count
    /// references) only inside plural arms.
    fn parse_message(&mut self, ctx: ParseContext) -> Result<Message, MessageError> {
        let mut parts = Vec::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if ctx == ParseContext::Top && (self.starts_with("{{") || self.starts_with("}}")) {
                text.push(c);
                self.pos += 2;
                continue;
            }
            match c {
                '{' => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    self.pos += 1;
                    parts.push(self.parse_placeholder()?);
                }
                '}' => break,
                '#' if ctx == ParseContext::PluralArm => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    self.pos += 1;
                    parts.push(Part::CountSelf);
                }
                _ => {
                    text.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(Message { parts })
    }

    fn parse_placeholder(&mut self) -> Result<Part, MessageError> {
        self.skip_ws();
        let name_offset = self.pos;
        let Some(name) = self.parse_ident() else {
            return match self.peek() {
                None => Err(MessageError::Unterminated {
                    offset: name_offset,
                }),
                Some(c) if c == '}' || c == ',' => Err(MessageError::MissingName {
                    offset: name_offset,
                }),
                Some(c) => Err(MessageError::InvalidName {
                    offset: name_offset,
                    found: c,
                }),
            };
        };
        self.skip_ws();
        match self.peek() {
            None => return Err(MessageError::Unterminated { offset: self.pos }),
            Some('}') => {
                self.pos += 1;
                return Ok(Part::Placeholder {
                    name,
                    kind: PlaceholderKind::Simple,
                });
            }
            Some(',') => {
                self.pos += 1;
            }
            Some(c) => {
                return Err(MessageError::ExpectedCommaOrClose {
                    offset: self.pos,
                    found: c,
                });
            }
        }
        self.skip_ws();
        let type_offset = self.pos;
        let Some(type_name) = self.parse_ident() else {
            return Err(MessageError::UnknownType {
                offset: type_offset,
                found: String::new(),
            });
        };
        let kind = match type_name.as_str() {
            "number" => self.parse_number_style(type_offset)?,
            "select" => self.parse_arms(type_offset, ArmGrammar::Option)?,
            "plural" => self.parse_arms(type_offset, ArmGrammar::Category)?,
            found => {
                return Err(MessageError::UnknownType {
                    offset: type_offset,
                    found: found.to_string(),
                });
            }
        };
        Ok(Part::Placeholder { name, kind })
    }

    /// The v1 subset supports only `{name, number, percent}`.
    fn parse_number_style(&mut self, type_offset: usize) -> Result<PlaceholderKind, MessageError> {
        self.skip_ws();
        let style_offset = self.pos;
        if self.peek() != Some(',') {
            // Missing style: `{n, number}` is not in the v1 subset.
            return Err(MessageError::UnsupportedNumberStyle {
                offset: style_offset,
                found: String::new(),
            });
        }
        self.pos += 1;
        self.skip_ws();
        let style_offset = self.pos;
        let Some(style) = self.parse_ident() else {
            return Err(MessageError::UnsupportedNumberStyle {
                offset: style_offset,
                found: String::new(),
            });
        };
        if style != "percent" {
            return Err(MessageError::UnsupportedNumberStyle {
                offset: style_offset,
                found: style,
            });
        }
        self.skip_ws();
        match self.peek() {
            Some('}') => {
                self.pos += 1;
                Ok(PlaceholderKind::NumberPercent)
            }
            None => Err(MessageError::Unterminated {
                offset: type_offset,
            }),
            Some(c) => Err(MessageError::ExpectedCommaOrClose {
                offset: self.pos,
                found: c,
            }),
        }
    }

    fn parse_arms(
        &mut self,
        type_offset: usize,
        grammar: ArmGrammar,
    ) -> Result<PlaceholderKind, MessageError> {
        // Consume the `,` separating the type keyword from the first arm
        // (mirrors `parse_number_style`, which consumes its own comma).
        self.skip_ws();
        match self.peek() {
            Some(',') => self.pos += 1,
            None => {
                return Err(MessageError::Unterminated {
                    offset: type_offset,
                });
            }
            Some(c) => {
                return Err(MessageError::ExpectedCommaOrClose {
                    offset: self.pos,
                    found: c,
                });
            }
        }
        let mut select_arms: Vec<(String, Message)> = Vec::new();
        let mut plural_arms: Vec<(PluralCategory, Message)> = Vec::new();
        let mut has_other = false;
        loop {
            // Arm-error offsets point just past the previous token (the
            // `,` after the type keyword, or the `}` closing the previous
            // arm): the start of the region where the arm name belongs.
            let arm_offset = self.pos;
            self.skip_ws();
            match self.peek() {
                None => {
                    return Err(MessageError::Unterminated {
                        offset: type_offset,
                    });
                }
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {}
            }
            let Some(arm_name) = self.parse_ident() else {
                return Err(MessageError::UnknownType {
                    offset: arm_offset,
                    found: String::new(),
                });
            };
            match grammar {
                ArmGrammar::Option => {
                    if select_arms
                        .iter()
                        .any(|(existing, _)| *existing == arm_name)
                    {
                        return Err(MessageError::DuplicateArm {
                            offset: arm_offset,
                            kind: "select",
                            found: arm_name,
                        });
                    }
                    has_other |= arm_name == "other";
                }
                ArmGrammar::Category => {
                    let Some(category) = PluralCategory::parse(&arm_name) else {
                        return Err(MessageError::InvalidCategory {
                            offset: arm_offset,
                            found: arm_name,
                        });
                    };
                    if plural_arms
                        .iter()
                        .any(|(existing, _)| *existing == category)
                    {
                        return Err(MessageError::DuplicateArm {
                            offset: arm_offset,
                            kind: "plural",
                            found: arm_name,
                        });
                    }
                    has_other |= category == PluralCategory::Other;
                }
            }
            self.skip_ws();
            match self.peek() {
                Some('{') => self.pos += 1,
                None => return Err(MessageError::Unterminated { offset: arm_offset }),
                Some(c) => {
                    return Err(MessageError::InvalidName {
                        offset: self.pos,
                        found: c,
                    });
                }
            }
            // Sub-message: recursion; `#` only inside plural arms.
            let sub = self.parse_message(match grammar {
                ArmGrammar::Option => ParseContext::SelectArm,
                ArmGrammar::Category => ParseContext::PluralArm,
            })?;
            match self.peek() {
                None => return Err(MessageError::Unterminated { offset: arm_offset }),
                // `parse_message` only stops before a `}`.
                Some(_) => self.pos += 1,
            }
            match grammar {
                ArmGrammar::Option => select_arms.push((arm_name, sub)),
                ArmGrammar::Category => {
                    // Checked above; the parse is total over valid names.
                    if let Some(category) = PluralCategory::parse(&arm_name) {
                        plural_arms.push((category, sub));
                    }
                }
            }
        }
        if !has_other {
            return Err(MessageError::MissingOtherArm {
                offset: type_offset,
            });
        }
        Ok(match grammar {
            ArmGrammar::Option => PlaceholderKind::Select(select_arms),
            ArmGrammar::Category => PlaceholderKind::Plural(plural_arms),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Arg, Message, MessageError, PlaceholderKind, format_float, format_percent};
    use crate::Locale;

    #[test]
    fn parses_plain_text_and_escapes() -> Result<(), MessageError> {
        let message = Message::parse("Plain text")?;
        assert_eq!(message.placeholder_set().len(), 0);
        let message = Message::parse("literal {{ brace }} here")?;
        assert_eq!(message.placeholder_set().len(), 0);
        Ok(())
    }

    #[test]
    fn parses_all_v1_forms() -> Result<(), MessageError> {
        let message = Message::parse("Hello, {name}!")?;
        assert_eq!(
            message.placeholder_set(),
            [("name".to_string(), "simple")].into_iter().collect()
        );

        let message = Message::parse("Buffering… {percent, number, percent}")?;
        assert_eq!(
            message.placeholder_set(),
            [("percent".to_string(), "number,percent")]
                .into_iter()
                .collect()
        );

        // Doc 07 §6.2 example, adjusted: the crate's v1 subset requires the
        // mandatory `other` arm (`MissingOtherArm`) while the doc's example
        // omits it — reconciliation is flagged for the Lead
        // [VERIFY — doc 07 §6.2 vs. crate rule].
        let message = Message::parse(
            "This media can't be played: {format, select, \
             known {no {format_name} decoder is available} \
             unknown {the format isn't recognized} \
             other {the format isn't supported}}",
        )?;
        let set = message.placeholder_set();
        assert_eq!(set.len(), 2);
        assert_eq!(set.get("format").copied(), Some("select"));
        assert_eq!(set.get("format_name").copied(), Some("simple"));

        let message = Message::parse("{count, plural, one {# item} other {# items}}")?;
        assert_eq!(
            message.placeholder_set(),
            [("count".to_string(), "plural")].into_iter().collect()
        );
        Ok(())
    }

    #[test]
    fn rejects_malformed_messages() {
        let cases: &[(&str, MessageError)] = &[
            ("oops {", MessageError::Unterminated { offset: 6 }),
            ("oops }", MessageError::UnmatchedClose { offset: 5 }),
            ("{} bad", MessageError::MissingName { offset: 1 }),
            (
                "{9bad}",
                MessageError::InvalidName {
                    offset: 1,
                    found: '9',
                },
            ),
            (
                "{name 3}",
                MessageError::ExpectedCommaOrClose {
                    offset: 6,
                    found: '3',
                },
            ),
            (
                "{name, uppercase}",
                MessageError::UnknownType {
                    offset: 7,
                    found: "uppercase".to_string(),
                },
            ),
            (
                "{n, number}",
                MessageError::UnsupportedNumberStyle {
                    offset: 10,
                    found: String::new(),
                },
            ),
            (
                "{n, number, bytes}",
                MessageError::UnsupportedNumberStyle {
                    offset: 12,
                    found: "bytes".to_string(),
                },
            ),
            (
                "{n, plural, one {x} two {y}}",
                MessageError::MissingOtherArm { offset: 4 },
            ),
            (
                "{n, plural, manys {x} other {y}}",
                MessageError::InvalidCategory {
                    offset: 11,
                    found: "manys".to_string(),
                },
            ),
            (
                "{n, select, a {x} a {y} other {z}}",
                MessageError::DuplicateArm {
                    offset: 17,
                    kind: "select",
                    found: "a".to_string(),
                },
            ),
            (
                "{n, plural, one {x} one {y} other {z}}",
                MessageError::DuplicateArm {
                    offset: 19,
                    kind: "plural",
                    found: "one".to_string(),
                },
            ),
            (
                "{a} {a, select, other {x}}",
                MessageError::ConflictingPlaceholderType {
                    name: "a".to_string(),
                    first: "simple",
                    second: "select",
                },
            ),
        ];
        for (src, expected) in cases {
            let got = Message::parse(src).err();
            assert_eq!(got.as_ref(), Some(expected), "case {src:?}");
        }
    }

    #[test]
    fn renders_simple_and_typed_placeholders() -> Result<(), Box<dyn std::error::Error>> {
        let locale = Locale::parse("en-US")?;
        let message = Message::parse("Hello, {name}! {percent, number, percent}")?;
        let rendered = message.render(
            &locale,
            &[("name", Arg::from("Lumen")), ("percent", Arg::from(0.45))],
        );
        assert_eq!(rendered.text, "Hello, Lumen! 45%");
        assert!(rendered.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn renders_select_and_plural() -> Result<(), Box<dyn std::error::Error>> {
        let locale = Locale::parse("en-US")?;
        let message = Message::parse("{count, plural, one {# item} other {# items}}")?;
        let one = message.render(&locale, &[("count", Arg::from(1i64))]);
        assert_eq!(one.text, "1 item");
        let many = message.render(&locale, &[("count", Arg::from(5i64))]);
        assert_eq!(many.text, "5 items");

        let message =
            Message::parse("{format, select, known {known format} other {unknown format}}")?;
        let known = message.render(&locale, &[("format", Arg::from("known"))]);
        assert_eq!(known.text, "known format");
        // Unmatched values fall back to `other` (ICU semantics, no diagnostic).
        let unknown = message.render(&locale, &[("format", Arg::from("mkv"))]);
        assert_eq!(unknown.text, "unknown format");
        assert!(unknown.diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn rendering_never_panics_on_bad_args() -> Result<(), Box<dyn std::error::Error>> {
        let locale = Locale::parse("en-US")?;
        let message = Message::parse("Hello, {name}! {percent, number, percent}")?;

        // Missing argument → visible literal + diagnostic.
        let rendered = message.render(&locale, &[]);
        assert_eq!(rendered.text, "Hello, {name}! {percent}");
        assert_eq!(rendered.diagnostics.len(), 2);

        // Type mismatch → visible literal + diagnostic.
        let rendered = message.render(
            &locale,
            &[("name", Arg::from("x")), ("percent", Arg::from("45"))],
        );
        assert_eq!(rendered.text, "Hello, x! {percent}");
        assert_eq!(rendered.diagnostics.len(), 1);
        Ok(())
    }

    #[test]
    fn number_formatting_helpers() {
        assert_eq!(format_float(45.0), "45");
        assert_eq!(format_float(45.5), "45.5");
        assert_eq!(format_percent(0.45), "45%");
        assert_eq!(format_percent(0.4567), "45.67%");
        assert_eq!(format_percent(1.0), "100%");
        assert_eq!(format_percent(0.0), "0%");
        assert_eq!(format_percent(0.455), "45.5%");
    }

    #[test]
    fn placeholder_kind_tags() {
        assert_eq!(PlaceholderKind::Simple.tag(), "simple");
        assert_eq!(PlaceholderKind::NumberPercent.tag(), "number,percent");
    }
}
