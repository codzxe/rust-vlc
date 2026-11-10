//! The ASS/SSA parser core (card 5.M2a): sections → styles →
//! dialogue events → [`Cue`]s, plain-text subset.
//!
//! # What is parsed vs. deferred (honest ledger)
//!
//! Parsed now: `[Script Info]` key/value pairs; `[V4+ Styles]` /
//! `[V4 Styles]` `Format`+`Style` lines projected onto the model's
//! style-token subset ([`TextStyle`]: bold/italic/underline +
//! `PrimaryColour` as RGBA8) plus the raw placement fields the
//! Phase-5 styling card will consume (font name/size, alignment,
//! margins — both alignment conventions: ASS numpad 1–9 and the
//! legacy SSA base+flags 1–11); `[Events]` `Format`+`Dialogue`
//! lines in ANY field order (the `Text` field absorbs commas).
//!
//! Deferred (recorded, not interpreted): the override tags OUTSIDE
//! the G-2 minimum (issue #17) — karaoke, rotation, animation,
//! fades, drawing — stripped, counted per tag name into
//! [`AssStats::dropped_tags`]; the secondary/outline/shadow colour
//! slots are parsed and counted into [`AssStats::unmapped_colour_tags`]
//! (the model's single-colour subset). The G-2 minimum — `\b \i \u
//! \s \fs \fn \c \1c \alpha \1a \pos \move` — is interpreted
//! natively by [`super::tags`] into styled runs plus a position
//! override. Layer z-order, Name/Effect metadata and margin overrides
//! are counted into [`AssStats`] as unmapped. The cue's
//! [`Positioning`] carries the referenced style's alignment (and the
//! `\pos`/`\move` override, as integer percents of the script
//! resolution); margins still need PlayRes-aware math beyond this
//! parser's scope.
//!
//! # Times
//!
//! ASS `H:MM:SS.CC` is centisecond-precise; the landed cue model
//! speaks **microseconds** (the card sketch said ns, but the model
//! is the authority — 1 cs = 10 000 µs exactly, so the mapping is
//! lossless). Fractions of 1–6 digits are accepted exactly (ffmpeg
//! writes 2); finer or malformed tokens are typed damage. Minutes
//! and seconds outside `0..=59` are accepted verbatim (libass's
//! lenient read: they contribute linearly); the only hard failures
//! are an unparseable token or arithmetic past `i64::MAX` µs.
//!
//! # Text rules (the plain-text subset)
//!
//! * `{\...}` override block with a closing `}` → stripped, tags
//!   counted. `{}` — the *exact* empty pair — is preserved as
//!   literal text (the VSFilter quirk the format inherited).
//! * An unterminated `{` (no `}` after it) → literal text from
//!   there on (libass compatibility).
//! * `\N` (hard) and `\n` (soft) → `'\n'` (the distinction is a
//!   WrapStyle renderer concern — deferred with styling); `\h` →
//!   `' '` (a hard space flattens to a space in plain text).
//! * Any other `\x` outside braces → literal backslash + `x`.

use std::collections::BTreeMap;

use crate::ass::tags::{self, PositionOverride};
#[cfg(test)]
use crate::model::cue::CueText;
use crate::model::cue::{Cue, CueId};
use crate::model::position::{
    Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign,
};
use crate::model::style::{Color, TextStyle};
use crate::model::time::CueInterval;

/// Which style-section flavour a [`AssStyle`] came from — this
/// decides the alignment convention (numpad vs. legacy base+flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleFlavour {
    /// `[V4+ Styles]` (ASS): `Alignment` is the numpad 1–9.
    V4Plus,
    /// `[V4 Styles]` (SSA): `Alignment` is base (1=left, 2=center,
    /// 3=right) + 4 for top + 8 for mid — the 1–11 legacy encoding.
    V4Legacy,
}

/// One `[V4+ Styles]`/`[V4 Styles]` `Style:` line, parsed to the
/// model's style-token subset plus the raw placement fields kept
/// for the Phase-5 styling card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssStyle {
    /// The style's name (`Default` is the conventional fallback).
    pub name: String,
    /// `Fontname` verbatim (e.g. `Arial`).
    pub font_name: String,
    /// `Fontsize` as an exact fixed-point value in thousandths
    /// (`16` → 16 000; `20.5` → 20 500) — no float anywhere near
    /// layout (MP §8); malformed tokens parse to 0 + damage.
    pub font_size_milli: i64,
    /// `PrimaryColour` as RGBA8, ASS `&HAABBGGRR`/decimal decoded
    /// (BGR order, ASS alpha 00=opaque inverted to straight
    /// 255=opaque). `None` when malformed (damage recorded).
    pub primary_colour: Option<Color>,
    /// The §2.2 style-token subset: `Bold`/`Italic`/`Underline`
    /// (ASS `-1`/`0`) + [`AssStyle::primary_colour`].
    pub text_style: TextStyle,
    /// The raw `Alignment` token, for the styling card.
    pub raw_alignment: i64,
    /// `Alignment` mapped through the section's convention into
    /// the model's 3×3 matrix.
    pub alignment: Alignment,
    /// `MarginL` (px, ASS convention).
    pub margin_l: i32,
    /// `MarginR` (px, ASS convention).
    pub margin_r: i32,
    /// `MarginV` (px, ASS convention).
    pub margin_v: i32,
    /// Which style-section flavour (alignment convention) applied.
    pub flavour: StyleFlavour,
}

/// A typed tolerance event. Every variant documents whether the
/// owning record was skipped or kept-with-defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssDamage {
    /// A `Start`/`End` token did not parse — **the cue was
    /// skipped**.
    BadTime {
        /// 1-based line number.
        line_no: usize,
        /// `"Start"` or `"End"`.
        which: &'static str,
        /// The offending token.
        raw: String,
    },
    /// `End` parsed but precedes `Start` (the model's typed-invalid
    /// interval) — **the cue was skipped**.
    EndBeforeStart {
        /// 1-based line number.
        line_no: usize,
        /// The rejected start (µs).
        start_us: i64,
        /// The rejected end (µs).
        end_us: i64,
    },
    /// An integer/decimal field (Layer, margins, Fontsize,
    /// Bold/…) failed to parse — **the record was kept**, the field
    /// fell back to its default (0 / flag-off).
    BadField {
        /// 1-based line number.
        line_no: usize,
        /// Which field.
        field: &'static str,
        /// The offending token.
        raw: String,
    },
    /// A colour literal was malformed — **the style was kept**,
    /// colour `None`.
    BadColour {
        /// 1-based line number.
        line_no: usize,
        /// The offending token.
        raw: String,
    },
    /// An `Alignment` token was outside the flavour's domain —
    /// **the style was kept**, bottom-center alignment.
    BadAlignment {
        /// 1-based line number.
        line_no: usize,
        /// The offending token.
        raw: String,
    },
    /// A `Style:`/`Dialogue:` line had too few fields for the
    /// declared `Format:` — **the record was skipped** (a skipped
    /// style means its cues fall back to the default positioning).
    BadFieldCount {
        /// 1-based line number.
        line_no: usize,
        /// Fields the `Format:` line declared.
        expected: usize,
        /// Fields the line actually carried.
        seen: usize,
    },
    /// A `Dialogue:` line referenced an unknown style — **the cue
    /// was kept**, default bottom-center positioning.
    UnknownStyle {
        /// 1-based line number.
        line_no: usize,
        /// The style name that was not defined.
        name: String,
    },
    /// An `[Events]` `Format:` line was unusable (no `Text`
    /// field) — **the standard field order was assumed**.
    BadEventFormat {
        /// 1-based line number.
        line_no: usize,
    },
}

/// Per-document honest-degradation counters (feasibility §5 §2.2's
/// `drop (counted)` dispositions — the R-09 conformance artifact).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssStats {
    /// `Comment:` events seen (legitimate, not damage).
    pub comment_events: u64,
    /// Non-text SSA events (`Picture`/`Sound`/`Movie`/`Command`/
    /// `Unknown`) — v1 has no rendering for them.
    pub non_text_events: u64,
    /// Sections encountered but not parsed, in order (`[Fonts]`,
    /// `[Aegisub Project Garbage]`, … anything unrecognized).
    pub skipped_sections: Vec<String>,
    /// Override blocks `{\...}` stripped from cue text.
    pub stripped_override_blocks: u64,
    /// Override tags APPLIED by the G-2 minimum (`\b \i \u \s \fs
    /// `\fn \c \1c \alpha \1a \pos \move`) — issue #17.
    pub applied_override_tags: u64,
    /// Secondary/outline/shadow colour tags (`\2c \3c \4c \2a
    /// `\3a \4a`) — parsed and validated, but the model's
    /// single-colour subset cannot hold them (unmapped, not dropped).
    pub unmapped_colour_tags: u64,
    /// Dropped override tags by name (`"k"`, `"frz"`, `"t"`, … —
    /// everything outside the G-2 minimum, plus malformed arguments).
    pub dropped_tags: BTreeMap<String, u64>,
    /// `\move` tags: the motion is not interpolated (static cue
    /// model) — the anchor froze at the start point.
    pub move_tags_frozen: u64,
    /// `\pos`/`\move` overrides successfully applied to a cue's
    /// [`Positioning`] as integer percents of the script resolution.
    pub position_overrides_applied: u64,
    /// `\pos`/`\move` overrides the model could not hold (coordinates
    /// outside `0..=100 %` of the script resolution) — counted, not
    /// clamped.
    pub position_overrides_unmappable: u64,
    /// `\N` hard newlines folded into `'\n'`.
    pub hard_newlines: u64,
    /// `\n` soft newlines folded into `'\n'`.
    pub soft_newlines: u64,
    /// `\h` hard spaces flattened to `' '`.
    pub hard_spaces: u64,
    /// Dialogue lines with a nonzero `Layer` — z-order is not in
    /// the v1 cue model.
    pub layer_values_ignored: u64,
    /// Dialogue lines with a nonzero margin override — v1
    /// positioning is alignment-only.
    pub margin_overrides_ignored: u64,
    /// Dialogue lines with a nonempty `Name` (actor) — unmapped.
    pub name_fields_ignored: u64,
    /// Dialogue lines with a nonempty `Effect` — unmapped.
    pub effect_fields_ignored: u64,
}

impl AssStats {
    /// Count a tag into [`AssStats::dropped_tags`] (the honest-degrade
    /// bucket). `pub(super)` — the G-2 engine in `ass::tags` shares it.
    pub(super) fn count_tag(&mut self, name: &str) {
        *self.dropped_tags.entry(name.to_owned()).or_insert(0) += 1;
    }
}

/// The parse result: everything found plus everything tolerated.
#[derive(Debug, Clone, Default)]
pub struct AssDocument {
    /// `[Script Info]` pairs in file order (duplicates kept — the
    /// later key usually wins in consumers; we stay honest).
    pub script_info: Vec<(String, String)>,
    /// `[V4+ Styles]`/`[V4 Styles]` styles, in file order.
    pub styles: Vec<AssStyle>,
    /// The dialogue events, mapped to the cue model in file order.
    pub cues: Vec<Cue>,
    /// Typed tolerance events, in encounter order.
    pub damage: Vec<AssDamage>,
    /// Degradation counters.
    pub stats: AssStats,
}

impl AssDocument {
    /// First value for a `[Script Info]` key (case-sensitive, as
    /// written in the file).
    #[must_use]
    pub fn info(&self, key: &str) -> Option<&str> {
        self.script_info
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The `Title` script-info value, if present.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.info("Title")
    }

    /// A style by name.
    #[must_use]
    pub fn style(&self, name: &str) -> Option<&AssStyle> {
        self.styles.iter().find(|s| s.name == name)
    }
}

/// A `Format:` field-name list: either the file's own or the
/// spec default — one accessor, no per-line allocation.
#[derive(Debug, Clone, Copy)]
enum Names<'a> {
    Default(&'a [&'static str]),
    Custom(&'a [String]),
}

impl Names<'_> {
    fn position(&self, name: &str) -> Option<usize> {
        match *self {
            Self::Default(names) => names.iter().position(|f| *f == name),
            Self::Custom(names) => names.iter().position(|f| f == name),
        }
    }

    fn len(&self) -> usize {
        match *self {
            Self::Default(names) => names.len(),
            Self::Custom(names) => names.len(),
        }
    }
}

/// Default `[V4+ Styles]` field order (the spec's).
const DEFAULT_STYLE_FORMAT: &[&str] = &[
    "Name",
    "Fontname",
    "Fontsize",
    "PrimaryColour",
    "SecondaryColour",
    "OutlineColour",
    "BackColour",
    "Bold",
    "Italic",
    "Underline",
    "StrikeOut",
    "ScaleX",
    "ScaleY",
    "Spacing",
    "Angle",
    "BorderStyle",
    "Outline",
    "Shadow",
    "Alignment",
    "MarginL",
    "MarginR",
    "MarginV",
    "Encoding",
];

/// Default `[Events]` field order (the spec's).
const DEFAULT_EVENT_FORMAT: &[&str] = &[
    "Layer", "Start", "End", "Style", "Name", "MarginL", "MarginR", "MarginV", "Effect", "Text",
];

/// Parse a whole ASS/SSA document. Infallible — malformed pieces
/// become [`AssDamage`] / [`AssStats`] entries.
#[must_use]
pub fn parse(input: &str) -> AssDocument {
    let mut doc = AssDocument::default();
    // Tolerate a UTF-8 BOM and both line endings (lines()' split is
    // normalized below with a \r strip).
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut section = Section::None;
    let mut style_format: Option<Vec<String>> = None;
    let mut event_format: Option<Vec<String>> = None;
    let mut flavour = StyleFlavour::V4Plus;

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with(';') {
            continue; // blank / SSA `;` comment line
        }
        if let Some(name) = section_name(line) {
            match name {
                "Script Info" => section = Section::ScriptInfo,
                "V4+ Styles" => {
                    flavour = StyleFlavour::V4Plus;
                    section = Section::Styles;
                }
                "V4 Styles" => {
                    flavour = StyleFlavour::V4Legacy;
                    section = Section::Styles;
                }
                "Events" => section = Section::Events,
                other => {
                    doc.stats.skipped_sections.push(other.to_owned());
                    section = Section::None;
                }
            }
            // Formats are section-scoped; reset on every entry.
            style_format = None;
            event_format = None;
            continue;
        }
        match section {
            Section::None => {}
            Section::ScriptInfo => parse_info_line(line, &mut doc),
            Section::Styles => {
                if let Some(rest) = line.strip_prefix("Format:") {
                    style_format = Some(parse_format_line(rest));
                } else {
                    let names = match style_format.as_deref() {
                        Some(custom) => Names::Custom(custom),
                        None => Names::Default(DEFAULT_STYLE_FORMAT),
                    };
                    parse_style_line(line, line_no, flavour, names, &mut doc);
                }
            }
            Section::Events => {
                if let Some(rest) = line.strip_prefix("Format:") {
                    let fields = parse_format_line(rest);
                    if fields.iter().any(|f| f == "Text") {
                        event_format = Some(fields);
                    } else {
                        doc.damage.push(AssDamage::BadEventFormat { line_no });
                    }
                } else {
                    let names = match event_format.as_deref() {
                        Some(custom) => Names::Custom(custom),
                        None => Names::Default(DEFAULT_EVENT_FORMAT),
                    };
                    parse_event_line(line, line_no, names, &mut doc);
                }
            }
        }
    }
    doc
}

/// A `Format:` payload → trimmed, nonempty field names.
fn parse_format_line(rest: &str) -> Vec<String> {
    rest.split(',')
        .map(|f| f.trim().to_owned())
        .filter(|f| !f.is_empty())
        .collect()
}

/// Which section the parser is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    ScriptInfo,
    Styles,
    Events,
}

/// `[Name]` section headers only — nothing else starts with `[`.
fn section_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
}

/// `Key: Value` inside `[Script Info]`.
fn parse_info_line(line: &str, doc: &mut AssDocument) {
    if let Some((key, value)) = line.split_once(':') {
        doc.script_info
            .push((key.trim().to_owned(), value.trim().to_owned()));
    }
}

/// A `Style:` line inside a styles section (`Format:` lines are
/// handled by the caller).
fn parse_style_line(
    line: &str,
    line_no: usize,
    flavour: StyleFlavour,
    format: Names<'_>,
    doc: &mut AssDocument,
) {
    let Some(rest) = line.strip_prefix("Style:") else {
        return; // unknown line type in a styles section: skipped
    };
    let fields = split_fields(rest);
    if fields.len() < format.len() {
        doc.damage.push(AssDamage::BadFieldCount {
            line_no,
            expected: format.len(),
            seen: fields.len(),
        });
        return;
    }
    let mut style = AssStyle {
        name: String::new(),
        font_name: String::new(),
        font_size_milli: 0,
        primary_colour: None,
        text_style: TextStyle::PLAIN,
        raw_alignment: 2,
        alignment: Positioning::default().alignment,
        margin_l: 0,
        margin_r: 0,
        margin_v: 0,
        flavour,
    };
    for idx in 0..format.len() {
        let raw = fields[idx].trim();
        let field: &str = match format {
            Names::Default(names) => names[idx],
            Names::Custom(names) => names[idx].as_str(),
        };
        match field {
            "Name" => style.name = raw.to_owned(),
            "Fontname" => style.font_name = raw.to_owned(),
            "Fontsize" => match parse_decimal_milli(raw) {
                Some(m) => style.font_size_milli = m,
                None => doc.damage.push(AssDamage::BadField {
                    line_no,
                    field: "Fontsize",
                    raw: raw.to_owned(),
                }),
            },
            "PrimaryColour" => match parse_ass_colour(raw) {
                Some(c) => {
                    style.primary_colour = Some(c);
                    style.text_style.color = Some(c);
                }
                None => doc.damage.push(AssDamage::BadColour {
                    line_no,
                    raw: raw.to_owned(),
                }),
            },
            "Bold" => style.text_style.bold = parse_bool_field(raw, line_no, "Bold", doc),
            "Italic" => style.text_style.italic = parse_bool_field(raw, line_no, "Italic", doc),
            "Underline" => {
                style.text_style.underline = parse_bool_field(raw, line_no, "Underline", doc);
            }
            "StrikeOut" => {
                style.text_style.strikeout = parse_bool_field(raw, line_no, "StrikeOut", doc);
            }
            "Alignment" => match raw.parse::<i64>() {
                Ok(v) => {
                    style.raw_alignment = v;
                    style.alignment = map_alignment(v, flavour).unwrap_or_else(|| {
                        doc.damage.push(AssDamage::BadAlignment {
                            line_no,
                            raw: raw.to_owned(),
                        });
                        Positioning::default().alignment
                    });
                }
                Err(_) => doc.damage.push(AssDamage::BadAlignment {
                    line_no,
                    raw: raw.to_owned(),
                }),
            },
            "MarginL" => style.margin_l = parse_int_field(raw, line_no, "MarginL", doc),
            "MarginR" => style.margin_r = parse_int_field(raw, line_no, "MarginR", doc),
            "MarginV" => style.margin_v = parse_int_field(raw, line_no, "MarginV", doc),
            // SecondaryColour/OutlineColour/BackColour, ScaleX/Y,
            // Spacing, Angle, BorderStyle, Outline, Shadow, Encoding:
            // full styling — deferred (§2.2).
            _ => {}
        }
    }
    doc.styles.push(style);
}

/// A `Dialogue:`/`Comment:`/other event line inside `[Events]`
/// (`Format:` lines are handled by the caller).
fn parse_event_line(line: &str, line_no: usize, format: Names<'_>, doc: &mut AssDocument) {
    // Non-dialogue SSA event types: legitimate, counted, not
    // rendered in v1.
    for prefix in [
        "Comment:", "Picture:", "Sound:", "Movie:", "Command:", "Unknown:",
    ] {
        if line.starts_with(prefix) {
            if prefix == "Comment:" {
                doc.stats.comment_events += 1;
            } else {
                doc.stats.non_text_events += 1;
            }
            return;
        }
    }
    let Some(rest) = line.strip_prefix("Dialogue:") else {
        return; // unrecognized line type in Events: skipped
    };
    let Some(text_idx) = format.position("Text") else {
        return; // guarded by BadEventFormat; unreachable via parse()
    };
    // Untrimmed split: only the pre-Text fields are trimmed at
    // lookup — Text itself must keep its internal spacing.
    let fields: Vec<&str> = rest.split(',').collect();
    if fields.len() < format.len() {
        doc.damage.push(AssDamage::BadFieldCount {
            line_no,
            expected: format.len(),
            seen: fields.len(),
        });
        return;
    }
    // The `Text` field absorbs commas ONLY when it is the last
    // declared field (the spec's position — and the only one where
    // absorption is sound); a non-final Text is one field.
    let joined = if text_idx + 1 == format.len() {
        fields[text_idx..].join(",")
    } else {
        fields[text_idx].to_owned()
    };
    // The text field's EDGES are trimmed (the space after
    // `Dialogue:` lands there), its interior is preserved.
    let text = joined.trim().to_owned();
    let get = |name: &str| -> Option<&str> {
        format
            .position(name)
            .and_then(|i| fields.get(i).map(|s| s.trim()))
    };

    // --- times: the two fields that can skip the cue ---
    let start_raw = get("Start").unwrap_or("");
    let end_raw = get("End").unwrap_or("");
    let Some(start_us) = parse_time(start_raw) else {
        doc.damage.push(AssDamage::BadTime {
            line_no,
            which: "Start",
            raw: start_raw.to_owned(),
        });
        return;
    };
    let Some(end_us) = parse_time(end_raw) else {
        doc.damage.push(AssDamage::BadTime {
            line_no,
            which: "End",
            raw: end_raw.to_owned(),
        });
        return;
    };
    let Ok(interval) = CueInterval::new(start_us, end_us) else {
        doc.damage.push(AssDamage::EndBeforeStart {
            line_no,
            start_us,
            end_us,
        });
        return;
    };

    // --- style → positioning + the G-2 style seed ---
    let style_name = get("Style")
        .filter(|s| !s.is_empty())
        .unwrap_or("Default")
        .to_owned();
    let (found_style, positioning) = match doc.style(&style_name).cloned() {
        Some(s) => (Some(s.clone()), Positioning::new(s.alignment, None, None)),
        None => {
            doc.damage.push(AssDamage::UnknownStyle {
                line_no,
                name: style_name,
            });
            (None, Positioning::default())
        }
    };
    // The override-tag engine's seed: the referenced style's token
    // projection (bold/italic/underline/strikeout/colour) plus its
    // font name/size — tags mutate this state mid-text (G-2).
    let seed = match &found_style {
        Some(s) => TextStyle {
            bold: s.text_style.bold,
            italic: s.text_style.italic,
            underline: s.text_style.underline,
            strikeout: s.text_style.strikeout,
            color: s.primary_colour,
            font_size_milli: (s.font_size_milli > 0).then_some(s.font_size_milli),
            font_name: (!s.font_name.is_empty()).then(|| s.font_name.as_str().into()),
        },
        None => TextStyle::PLAIN,
    };

    // --- tolerated / counted-then-dropped metadata ---
    if parse_int_field(get("Layer").unwrap_or("0"), line_no, "Layer", doc) != 0 {
        doc.stats.layer_values_ignored += 1;
    }
    let margin_nonzero = ["MarginL", "MarginR", "MarginV"]
        .iter()
        .any(|m| parse_int_field(get(m).unwrap_or("0"), line_no, m, doc) != 0);
    if margin_nonzero {
        doc.stats.margin_overrides_ignored += 1;
    }
    if !get("Name").unwrap_or("").is_empty() {
        doc.stats.name_fields_ignored += 1;
    }
    if !get("Effect").unwrap_or("").is_empty() {
        doc.stats.effect_fields_ignored += 1;
    }

    // --- text: the G-2 override-tag engine (styled runs + pos) ---
    let (cue_text, pos_override) = tags::styled_event_text(&text, seed, &mut doc.stats);
    let positioning = match pos_override {
        Some(p) => apply_position_override(positioning, p, play_res(doc), &mut doc.stats),
        None => positioning,
    };

    let id = CueId::new(doc.cues.len() as u64 + 1);
    doc.cues.push(Cue::new(id, interval, cue_text, positioning));
}

/// The effective script resolution: `PlayResX`/`PlayResY` when
/// present and nonzero, else the 384×288 VSFilter-heritage default
/// (`layout::DEFAULT_PLAY_RES`, the libass behaviour).
fn play_res(doc: &AssDocument) -> (i64, i64) {
    let read = |key: &str, default: i64| {
        doc.info(key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(default)
    };
    (read("PlayResX", 384), read("PlayResY", 288))
}

/// Apply a `\pos`/`\move` override to a cue's [`Positioning`]: the
/// script coordinates become integer percents of the script
/// resolution (round-half-up — the model's lossy percent vocabulary,
/// `model::position` docs). Coordinates outside `0..=100 %` are
/// unmappable in that vocabulary: counted into
/// [`AssStats::position_overrides_unmappable`], never clamped (a
/// clamp would move the cue silently; MP §8 prefers honest data).
fn apply_position_override(
    mut positioning: Positioning,
    p: PositionOverride,
    play_res: (i64, i64),
    stats: &mut AssStats,
) -> Positioning {
    // percent = milli / (extent × 1000) × 100, round-half-up.
    let to_percent = |milli: i64, extent: i64| -> Option<i32> {
        let num = milli.checked_mul(100)?;
        let den = extent.checked_mul(1000)?;
        if den == 0 {
            return None;
        }
        let q = (num + den / 2) / den;
        i32::try_from(q).ok().filter(|v| (0..=100).contains(v))
    };
    match (
        to_percent(p.x_milli, play_res.0),
        to_percent(p.y_milli, play_res.1),
    ) {
        (Some(px), Some(py)) => match (Percent::new(px), Percent::new(py)) {
            (Ok(x), Ok(y)) => {
                positioning.position = Some(x);
                positioning.line = Some(LinePlacement::Percent(y));
                stats.position_overrides_applied += 1;
            }
            // Unreachable (pre-validated 0..=100); kept total (MP §8).
            _ => stats.position_overrides_unmappable += 1,
        },
        _ => stats.position_overrides_unmappable += 1,
    }
    positioning
}

/// Split a post-`Style:` field list on commas, trimming each field
/// (style fields carry no significant spacing).
fn split_fields(rest: &str) -> Vec<&str> {
    rest.split(',').map(str::trim).collect()
}

/// ASS `-1`/`0`-style boolean; `1` is accepted, anything else is
/// damage + false.
fn parse_bool_field(raw: &str, line_no: usize, field: &'static str, doc: &mut AssDocument) -> bool {
    match raw.trim() {
        "-1" | "1" => true,
        "0" => false,
        other => {
            doc.damage.push(AssDamage::BadField {
                line_no,
                field,
                raw: other.to_owned(),
            });
            false
        }
    }
}

/// A tolerated integer field: parse or default 0 + damage.
fn parse_int_field(raw: &str, line_no: usize, field: &'static str, doc: &mut AssDocument) -> i32 {
    match raw.trim().parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            doc.damage.push(AssDamage::BadField {
                line_no,
                field,
                raw: raw.trim().to_owned(),
            });
            0
        }
    }
}

/// `H:MM:SS.CC` → µs. Lenient: any digit counts for H/M/S (they
/// contribute linearly), a fraction of 1–6 digits is accepted
/// exactly, a missing fraction is allowed. Exact `u64` math; past
/// `i64::MAX` µs the time is typed-bad (`None`).
fn parse_time(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let (hms, frac) = match raw.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (raw, None),
    };
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: u64 = parts[0].parse().ok()?;
    let m: u64 = parts[1].parse().ok()?;
    let s: u64 = parts[2].parse().ok()?;
    let frac_us = match frac {
        None => 0,
        Some(digits)
            if (1..=6).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit()) =>
        {
            let value: u64 = digits.parse().ok()?;
            value.checked_mul(10_u64.pow(u32::try_from(6 - digits.len()).ok()?))?
        }
        Some(_) => return None,
    };
    let total_s = s
        .checked_add(m.checked_mul(60)?)?
        .checked_add(h.checked_mul(3600)?)?;
    let total_us = total_s.checked_mul(1_000_000)?.checked_add(frac_us)?;
    if total_us > i64::MAX as u64 {
        return None;
    }
    Some(total_us as i64)
}

/// ASS alignment → the model's 3×3 matrix. `flavour` selects the
/// convention:
///
/// * [`StyleFlavour::V4Plus`] — numpad 1–9 (`2` = bottom-center,
///   `5` = middle-center, …);
/// * [`StyleFlavour::V4Legacy`] — base 1/2/3 = left/center/right,
///   +4 top, +8 mid (the 1–11 encoding; bare 4 and 8 are invalid).
fn map_alignment(v: i64, flavour: StyleFlavour) -> Option<Alignment> {
    match flavour {
        StyleFlavour::V4Plus => {
            if !(1..=9).contains(&v) {
                return None;
            }
            let vertical = match v {
                1..=3 => VerticalAlign::Bottom,
                4..=6 => VerticalAlign::Middle,
                _ => VerticalAlign::Top,
            };
            let horizontal = match v {
                1 | 4 | 7 => HorizontalAlign::Left,
                2 | 5 | 8 => HorizontalAlign::Center,
                _ => HorizontalAlign::Right,
            };
            Some(Alignment::new(vertical, horizontal))
        }
        StyleFlavour::V4Legacy => {
            if !(1..=11).contains(&v) {
                return None;
            }
            let horizontal = match v & 3 {
                1 => HorizontalAlign::Left,
                2 => HorizontalAlign::Center,
                3 => HorizontalAlign::Right,
                _ => return None, // bare 4/8 — no horizontal base
            };
            let vertical = if v >= 9 {
                VerticalAlign::Middle
            } else if v >= 5 {
                VerticalAlign::Top
            } else {
                VerticalAlign::Bottom
            };
            Some(Alignment::new(vertical, horizontal))
        }
    }
}

/// An ASS colour literal → RGBA8 [`Color`].
///
/// Accepted forms: `&HAABBGGRR`, `&HBBGGRR` (and shorter hex,
/// zero-padded), an optional trailing `&`, and plain decimal
/// (signed; negative wraps as the two's-complement 32-bit value —
/// the field is 32-bit in both conventions). ASS alpha is inverted
/// (`00` opaque) and lands as straight alpha (`255` opaque).
/// `&H` with no digits is malformed (`None`).
pub(super) fn parse_ass_colour(raw: &str) -> Option<Color> {
    let raw = raw.trim();
    let hex_tail = raw
        .strip_prefix("&H")
        .or_else(|| raw.strip_prefix("&h"))
        .map(|t| t.strip_suffix('&').unwrap_or(t));
    let value: u32 = if let Some(hex) = hex_tail {
        if hex.is_empty() || hex.len() > 8 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok()?
    } else if let Ok(dec) = raw.parse::<i64>() {
        u32::try_from(dec).unwrap_or(dec as u32)
    } else {
        return None;
    };
    let r = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let b = ((value >> 16) & 0xFF) as u8;
    let ass_alpha = ((value >> 24) & 0xFF) as u8;
    Some(Color::new(r, g, b, 255 - ass_alpha))
}

/// An exact decimal → thousandths (`16` → 16 000, `20.5` →
/// 20 500). No floats (MP §8). Optional sign, 0–3 fraction digits.
pub(super) fn parse_decimal_milli(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let (neg, body) = match raw.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, raw.strip_prefix('+').unwrap_or(raw)),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
        || frac_part.len() > 3
    {
        return None;
    }
    let whole: i64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };
    let mut frac = 0i64;
    for (place, ch) in frac_part.chars().enumerate() {
        frac += i64::from(ch as u8 - b'0') * 10_i64.pow(2 - place as u32);
    }
    let milli = whole.checked_mul(1000)?.checked_add(frac)?;
    Some(if neg { -milli } else { milli })
}

/// Reduce ASS event text to plain string form — a thin test-only
/// wrapper over the G-2 engine (`ass::tags`) asserting the text-level
/// quirks independently of run styling.
#[cfg(test)]
fn plain_text(text: &str, stats: &mut AssStats) -> String {
    match tags::styled_event_text(text, TextStyle::PLAIN, stats).0 {
        CueText::Plain(s) => s,
        CueText::Styled(segs) => segs.iter().map(|s| s.text.as_str()).collect(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::cue::{CueText, StyledSegment};
    use crate::model::position::{HorizontalAlign, VerticalAlign};

    // lumen-allow-literal: test fixture value — round-trip data, not display identity
    const FFMPEG_LIKE: &str = "\
[Script Info]
ScriptType: v4.00+
PlayResX: 384
PlayResY: 288

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,16,&Hffffff,&Hffffff,&H0,&H0,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:03.50,Default,,0,0,0,,Hello Lumen
Dialogue: 0,0:00:04.25,0:00:06.00,Default,,0,0,0,,Second line here\\Nwith two lines
";

    /// A minimal style block + [Events] Format so event-only tests
    /// exercise the happy path (a defined `Default` style). Lines:
    /// 1–3 styles, 4 blank, 5 [Events], 6 Format — first Dialogue is
    /// line 7.
    const DEFAULT_STYLE_AND_EVENTS: &str = "\
[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,16,&Hffffff,&Hffffff,&H0,&H0,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
";

    #[test]
    fn parses_script_info_and_style() -> Result<(), Box<dyn std::error::Error>> {
        let doc = parse(FFMPEG_LIKE);
        assert_eq!(doc.info("ScriptType"), Some("v4.00+"));
        assert_eq!(doc.info("PlayResX"), Some("384"));
        assert_eq!(doc.title(), None);
        let style = doc.style("Default").ok_or("Default style")?;
        assert_eq!(style.font_name, "Arial");
        assert_eq!(style.font_size_milli, 16_000);
        // &Hffffff → BGR white, no alpha byte → opaque white.
        assert_eq!(style.primary_colour, Some(Color::WHITE));
        assert_eq!(style.text_style.color, Some(Color::WHITE));
        assert!(!style.text_style.bold);
        assert_eq!(style.margin_l, 10);
        assert_eq!(style.margin_v, 10);
        // numpad 2 = bottom-center
        assert_eq!(style.alignment.vertical, VerticalAlign::Bottom);
        assert_eq!(style.alignment.horizontal, HorizontalAlign::Center);
        assert_eq!(style.flavour, StyleFlavour::V4Plus);
        assert!(doc.damage.is_empty(), "{:?}", doc.damage);
        Ok(())
    }

    #[test]
    fn parses_dialogue_times_and_text() {
        let doc = parse(FFMPEG_LIKE);
        assert_eq!(doc.cues.len(), 2);
        let c0 = &doc.cues[0];
        assert_eq!(c0.id.get(), 1);
        assert_eq!(c0.interval.start_us(), 1_000_000);
        assert_eq!(c0.interval.end_us(), 3_500_000);
        assert_eq!(c0.text, CueText::Plain("Hello Lumen".into())); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        // Style alignment respected; no explicit line/position.
        assert_eq!(c0.positioning.alignment, doc.styles[0].alignment);
        assert!(c0.positioning.line.is_none());
        assert!(c0.positioning.position.is_none());
        let c1 = &doc.cues[1];
        assert_eq!(c1.interval.start_us(), 4_250_000);
        assert_eq!(c1.interval.end_us(), 6_000_000);
        assert_eq!(
            c1.text,
            CueText::Plain("Second line here\nwith two lines".into())
        );
        assert_eq!(doc.stats.hard_newlines, 1);
    }

    #[test]
    fn time_parsing_edges() {
        assert_eq!(parse_time("0:00:01.00"), Some(1_000_000));
        assert_eq!(parse_time("0:01:02.75"), Some(62_750_000));
        assert_eq!(parse_time("0:10:00.00"), Some(600_000_000));
        assert_eq!(parse_time("12:34:56.78"), Some(45_296_780_000));
        assert_eq!(parse_time("0:00:05"), Some(5_000_000)); // no fraction
        assert_eq!(parse_time("0:00:05.123456"), Some(5_123_456)); // µs-exact
        assert_eq!(parse_time("0:00:05.1234567"), None); // finer than µs
        assert_eq!(parse_time("0:00:05."), None); // bare dot
        assert_eq!(parse_time("x:00:05.00"), None);
        assert_eq!(parse_time("0:o:5.0"), None); // non-digit minute token
        assert_eq!(parse_time("00:00:05.00"), Some(5_000_000)); // 2-digit hour
        assert_eq!(parse_time("-0:00:01.00"), None); // negative
        // Overflow past i64::MAX µs: ~292k years.
        assert_eq!(parse_time("9999999999999:00:00.00"), None);
        // Lenient minutes/seconds (libass reads them linearly).
        assert_eq!(parse_time("0:99:00.00"), Some(99 * 60 * 1_000_000));
    }

    #[test]
    fn colour_forms() {
        assert_eq!(parse_ass_colour("&Hffffff"), Some(Color::WHITE));
        assert_eq!(parse_ass_colour("&HFFFFFF"), Some(Color::WHITE));
        assert_eq!(parse_ass_colour("&H00FFFFFF"), Some(Color::WHITE));
        // ASS alpha 00 opaque, FF transparent → straight alpha 0.
        assert_eq!(parse_ass_colour("&HFF000000"), Some(Color::new(0, 0, 0, 0)));
        // BGR: &H0000FF = red.
        assert_eq!(
            parse_ass_colour("&H0000FF"),
            Some(Color::new(255, 0, 0, 255))
        );
        // Trailing & (SSA V4 style) + short hex zero-pad.
        assert_eq!(parse_ass_colour("&HFFFFFF&"), Some(Color::WHITE));
        assert_eq!(parse_ass_colour("&H0"), Some(Color::BLACK));
        // Decimal forms: 16777215 = 0x00FFFFFF white; -1 wraps to
        // 0xFFFFFFFF (white, fully transparent).
        assert_eq!(parse_ass_colour("16777215"), Some(Color::WHITE));
        assert_eq!(parse_ass_colour("-1"), Some(Color::new(255, 255, 255, 0)));
        assert_eq!(parse_ass_colour("&H"), None);
        assert_eq!(parse_ass_colour("&HGGGGGG"), None);
        assert_eq!(parse_ass_colour("&H123456789"), None); // 9 digits
        assert_eq!(parse_ass_colour(""), None);
    }

    #[test]
    fn alignment_conventions() {
        use crate::model::position::{HorizontalAlign as H, VerticalAlign as V};
        let numpad = |v: i64| map_alignment(v, StyleFlavour::V4Plus);
        let legacy = |v: i64| map_alignment(v, StyleFlavour::V4Legacy);
        // ASS numpad: 1 bl 2 bc 3 br 4 ml 5 mc 6 mr 7 tl 8 tc 9 tr.
        assert_eq!(
            numpad(2).map(|a| (a.vertical, a.horizontal)),
            Some((V::Bottom, H::Center))
        );
        assert_eq!(
            numpad(5).map(|a| (a.vertical, a.horizontal)),
            Some((V::Middle, H::Center))
        );
        assert_eq!(
            numpad(9).map(|a| (a.vertical, a.horizontal)),
            Some((V::Top, H::Right))
        );
        assert_eq!(
            numpad(1).map(|a| (a.vertical, a.horizontal)),
            Some((V::Bottom, H::Left))
        );
        assert_eq!(numpad(0), None);
        assert_eq!(numpad(10), None);
        // SSA legacy: 1..3 bottom, +4 top, +8 mid; 4/8 bare invalid.
        assert_eq!(
            legacy(1).map(|a| (a.vertical, a.horizontal)),
            Some((V::Bottom, H::Left))
        );
        assert_eq!(
            legacy(2).map(|a| (a.vertical, a.horizontal)),
            Some((V::Bottom, H::Center))
        );
        assert_eq!(
            legacy(5).map(|a| (a.vertical, a.horizontal)),
            Some((V::Top, H::Left))
        );
        assert_eq!(
            legacy(7).map(|a| (a.vertical, a.horizontal)),
            Some((V::Top, H::Right))
        );
        assert_eq!(
            legacy(9).map(|a| (a.vertical, a.horizontal)),
            Some((V::Middle, H::Left))
        );
        assert_eq!(
            legacy(11).map(|a| (a.vertical, a.horizontal)),
            Some((V::Middle, H::Right))
        );
        assert_eq!(legacy(4), None);
        assert_eq!(legacy(8), None);
        assert_eq!(legacy(12), None);
    }

    #[test]
    fn v4_legacy_section_uses_legacy_alignment() -> Result<(), Box<dyn std::error::Error>> {
        let ssa = "\
[V4 Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, Bold, Italic, Alignment, MarginL, MarginR, MarginV
Style: Default,Courier,20,&HFFFFFF,-1,0,11,5,5,5

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,legacy
";
        let doc = parse(ssa);
        let style = doc.style("Default").ok_or("style")?;
        assert_eq!(style.flavour, StyleFlavour::V4Legacy);
        assert_eq!(style.raw_alignment, 11);
        assert_eq!(style.alignment.vertical, VerticalAlign::Middle);
        assert_eq!(style.alignment.horizontal, HorizontalAlign::Right);
        assert!(style.text_style.bold); // ASS -1 convention
        assert_eq!(style.font_size_milli, 20_000);
        // The cue inherits the legacy-mapped alignment.
        assert_eq!(doc.cues[0].positioning.alignment, style.alignment);
        assert!(doc.damage.is_empty(), "{:?}", doc.damage);
        Ok(())
    }

    #[test]
    fn custom_event_format_orders_fields() {
        let doc = parse(
            "[Events]\n\
             Format: Text, Start, End, Style\n\
             Dialogue: Hello, 0:00:01.00, 0:00:02.00, Default\n",
        );
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].text, CueText::Plain("Hello".into()));
        assert_eq!(doc.cues[0].interval.start_us(), 1_000_000);
        assert_eq!(doc.cues[0].interval.end_us(), 2_000_000);
        // Missing Style field falls back to Default → unknown here.
        assert!(matches!(
            doc.damage.first(),
            Some(AssDamage::UnknownStyle { .. })
        ));
    }

    #[test]
    fn text_commas_absorbed_into_text_field() {
        let doc = parse(
            "[Events]\n\
             Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
             Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,One, two, three\n",
        );
        assert_eq!(doc.cues[0].text, CueText::Plain("One, two, three".into()));
    }

    #[test]
    fn text_rules_override_blocks_and_escapes() {
        let mut stats = AssStats::default();
        assert_eq!(
            plain_text("{\\b1}Bold{\\b0} plain", &mut stats),
            "Bold plain"
        );
        assert_eq!(stats.stripped_override_blocks, 2);
        // G-2: \b is applied now, not dropped.
        assert_eq!(stats.applied_override_tags, 2);
        assert_eq!(stats.dropped_tags.get("b"), None);
        assert_eq!(plain_text("{}", &mut stats), "{}"); // preserved quirk
        assert_eq!(plain_text("a { b", &mut stats), "a { b"); // unterminated
        assert_eq!(plain_text("{ not a tag", &mut stats), "{ not a tag");
        assert_eq!(
            plain_text("hard\\Nsoft\\nspace\\h", &mut stats),
            "hard\nsoft\nspace "
        );
        assert_eq!(stats.hard_newlines, 1);
        assert_eq!(stats.soft_newlines, 1);
        assert_eq!(stats.hard_spaces, 1);
        assert_eq!(plain_text("slash\\d keeps", &mut stats), "slash\\d keeps");
        assert_eq!(plain_text("trailing\\", &mut stats), "trailing\\");
        // Karaoke stays dropped; \pos is applied by G-2 (and needs
        // PlayRes context to land in a Positioning — the plain-text
        // wrapper only checks the text here).
        let mut s2 = AssStats::default();
        assert_eq!(
            plain_text("{\\k20}ka{\\k40}ra{\\pos(10,20)}oke", &mut s2),
            "karaoke"
        );
        assert_eq!(s2.dropped_tags.get("k"), Some(&2));
        assert_eq!(s2.applied_override_tags, 1); // the \pos
        // UTF-8 walks scalars, never splits a code point.
        let mut s3 = AssStats::default();
        assert_eq!(plain_text("日本語\\Nテスト", &mut s3), "日本語\nテスト");
        assert_eq!(plain_text("aé{}", &mut s3), "aé{}");
        // Empty override-ish block with whitespace inside is NOT the
        // `{}` quirk: it is a block and is stripped.
        assert_eq!(plain_text("a{ }b", &mut s3), "ab");
    }

    #[test]
    fn malformed_times_skip_the_cue_with_typed_damage() {
        let doc = parse(&format!(
            "{DEFAULT_STYLE_AND_EVENTS}Dialogue: 0,bad,0:00:02.00,Default,,0,0,0,,skipped\n\
             Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,kept\n\
             Dialogue: 0,0:00:05.00,worse,Default,,0,0,0,,skipped\n\
             Dialogue: 0,0:00:09.00,0:00:08.00,Default,,0,0,0,,reversed\n"
        ));
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].text, CueText::Plain("kept".into()));
        assert_eq!(
            doc.damage,
            vec![
                AssDamage::BadTime {
                    line_no: 7,
                    which: "Start",
                    raw: "bad".into()
                },
                AssDamage::BadTime {
                    line_no: 9,
                    which: "End",
                    raw: "worse".into()
                },
                AssDamage::EndBeforeStart {
                    line_no: 10,
                    start_us: 9_000_000,
                    end_us: 8_000_000
                },
            ]
        );
    }

    #[test]
    fn zero_duration_cue_is_legal() {
        let doc = parse(&format!(
            "{DEFAULT_STYLE_AND_EVENTS}Dialogue: 0,0:00:05.00,0:00:05.00,Default,,0,0,0,,tick\n"
        ));
        assert_eq!(doc.cues.len(), 1);
        assert!(doc.cues[0].interval.is_empty());
        assert!(doc.damage.is_empty());
    }

    #[test]
    fn tolerated_field_damage_and_counters() {
        let doc = parse(&format!(
            "{DEFAULT_STYLE_AND_EVENTS}Dialogue: x,0:00:01.00,0:00:02.00,Default,Actor,y,0,5,FX!,{{\\i1}}styled{{\\i0}}\n\
             Comment: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,hidden comment\n"
        ));
        assert_eq!(doc.cues.len(), 1);
        // Bad Layer → 0 + damage; still counted as a layer value? No:
        // it parsed to 0 via damage, so not counted as ignored.
        assert!(
            doc.damage
                .iter()
                .any(|d| matches!(d, AssDamage::BadField { field: "Layer", .. }))
        );
        assert!(doc.damage.iter().any(|d| matches!(
            d,
            AssDamage::BadField {
                field: "MarginL",
                ..
            }
        )));
        assert_eq!(doc.stats.margin_overrides_ignored, 1); // MarginV=5
        assert_eq!(doc.stats.name_fields_ignored, 1);
        assert_eq!(doc.stats.effect_fields_ignored, 1);
        assert_eq!(doc.stats.comment_events, 1);
        // G-2: {\\i1} styled the run instead of being dropped.
        assert_eq!(
            doc.cues[0].text,
            CueText::Styled(vec![StyledSegment {
                text: "styled".into(),
                style: TextStyle::new(false, true, false, Some(Color::WHITE))
                    .with_font_name(Some("Arial"))
                    .with_font_size_milli(Some(16_000)),
            }])
        );
        assert_eq!(doc.stats.applied_override_tags, 2);
        assert_eq!(doc.stats.dropped_tags.get("i"), None);
    }

    #[test]
    fn unknown_sections_and_bom_and_crlf() {
        let raw = "\u{feff}[Script Info]\r\nTitle: Test\r\n; a comment\r\n\
                   [Aegisub Project Garbage]\r\nLast Style Storage: Default\r\n\
                   [Unknown Future Section]\r\nwhatever: 1\r\n\
                   [Events]\r\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\r\n\
                   Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,crlf\r\n";
        let doc = parse(raw);
        assert_eq!(doc.title(), Some("Test"));
        assert_eq!(doc.stats.skipped_sections.len(), 2);
        assert!(
            doc.stats
                .skipped_sections
                .contains(&"Aegisub Project Garbage".to_owned())
        );
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].text, CueText::Plain("crlf".into()));
    }

    #[test]
    fn style_line_damage_paths() {
        let doc = parse(
            "[V4+ Styles]\n\
             Format: Name, Fontname, Fontsize, PrimaryColour, Bold, Alignment\n\
             Style: Broken\n\
             Style: Odd,Arial,xx,notacolour,maybe,42\n",
        );
        // First style skipped for field count; second kept with damage.
        assert_eq!(doc.styles.len(), 1);
        assert!(matches!(
            doc.damage.first(),
            Some(AssDamage::BadFieldCount {
                expected: 6,
                seen: 1,
                ..
            })
        ));
        let s = &doc.styles[0];
        assert_eq!(s.name, "Odd");
        assert_eq!(s.font_size_milli, 0); // xx → damage + 0
        assert_eq!(s.primary_colour, None); // notacolour → damage
        assert!(!s.text_style.bold); // maybe → damage + false
        assert_eq!(s.alignment, Positioning::default().alignment); // 42 → damage
        assert_eq!(
            doc.damage
                .iter()
                .filter(|d| matches!(d, AssDamage::BadField { .. }))
                .count(),
            2
        );
        assert!(
            doc.damage
                .iter()
                .any(|d| matches!(d, AssDamage::BadColour { .. }))
        );
        assert!(
            doc.damage
                .iter()
                .any(|d| matches!(d, AssDamage::BadAlignment { .. }))
        );
    }

    #[test]
    fn decimal_milli_edges() {
        assert_eq!(parse_decimal_milli("16"), Some(16_000));
        assert_eq!(parse_decimal_milli("20.5"), Some(20_500));
        assert_eq!(parse_decimal_milli("0.001"), Some(1));
        assert_eq!(parse_decimal_milli(".5"), Some(500));
        assert_eq!(parse_decimal_milli("-3.25"), Some(-3_250));
        assert_eq!(parse_decimal_milli("+7"), Some(7_000));
        assert_eq!(parse_decimal_milli("1.2345"), None); // 4 fraction digits
        assert_eq!(parse_decimal_milli(""), None);
        assert_eq!(parse_decimal_milli("x"), None);
    }

    #[test]
    fn non_text_events_counted_not_damaged() {
        let doc = parse(
            "[Events]\n\
             Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
             Command: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,!some command\n\
             Picture: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,image.bmp\n",
        );
        assert!(doc.cues.is_empty());
        assert_eq!(doc.stats.non_text_events, 2);
        assert!(doc.damage.is_empty());
    }

    /// Deterministic fuzz-lite (card 5.M2 AC): LCG-seeded junk,
    /// truncations and byte mutations of a valid document, all
    /// under the 200 KB budget — zero panics, and the invariants
    /// hold on every output (intervals valid by construction,
    /// ids dense, damage line numbers in range).
    #[test]
    fn fuzz_lite_survives_junk_and_mutations() {
        const RUNS: u64 = 2000;
        let mut seed: u64 = 0x5eed_1234_abcd_ef01;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        const ALPHABET: &[u8] =
            b"[]{}\\:,\n\rabcXYZ019.;-+&HTitleStyleDialogueFormatCommentEvents \xb0";
        let check = |input: &str| {
            let bytes = input.as_bytes();
            assert!(bytes.len() < 200_000);
            let doc = parse(input);
            let n_lines = input.lines().count().max(1);
            for (i, cue) in doc.cues.iter().enumerate() {
                assert_eq!(cue.id.get(), i as u64 + 1);
                assert!(cue.interval.start_us() <= cue.interval.end_us());
                // G-2: mutated junk can legitimately style a run now
                // ({\b1} in the alphabet) — the invariant is a valid
                // payload, not plainness.
                assert!(cue.text.total_len() < 200_000);
            }
            for d in &doc.damage {
                let line = match d {
                    AssDamage::BadTime { line_no, .. }
                    | AssDamage::EndBeforeStart { line_no, .. }
                    | AssDamage::BadField { line_no, .. }
                    | AssDamage::BadColour { line_no, .. }
                    | AssDamage::BadAlignment { line_no, .. }
                    | AssDamage::BadFieldCount { line_no, .. }
                    | AssDamage::UnknownStyle { line_no, .. }
                    | AssDamage::BadEventFormat { line_no } => *line_no,
                };
                assert!(line >= 1 && line <= n_lines, "line {line} of {n_lines}");
            }
        };
        for run in 0..RUNS {
            let mode = run % 4;
            let input: String = if mode == 0 {
                // Pure junk from the structural alphabet.
                let len = (next() % 4096) as usize;
                (0..len)
                    .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize] as char)
                    .collect()
            } else {
                // Mutations of the valid corpus sample.
                let mut s = FFMPEG_LIKE.to_owned();
                s.push_str("Dialogue: 1,0:12:34.56,0:12:35.00,Other,N,1,2,3,F,{\\pos(1,2)\\k10}日本\\N語{}\n");
                let mutations = (next() % 24) as usize;
                let mut buf = s.into_bytes();
                for _ in 0..mutations {
                    if buf.is_empty() {
                        break;
                    }
                    let pos = (next() % buf.len() as u64) as usize;
                    match next() % 3 {
                        0 => buf[pos] = ALPHABET[(next() % ALPHABET.len() as u64) as usize],
                        1 => {
                            buf.remove(pos);
                        }
                        _ => buf.insert(pos, ALPHABET[(next() % ALPHABET.len() as u64) as usize]),
                    }
                }
                // Truncation half the time.
                if next() % 2 == 0 && !buf.is_empty() {
                    buf.truncate((next() as usize) % buf.len());
                }
                // The mutation may split a code point: lossy-decode
                // is exactly the dirty-world contract (parse must
                // survive any &str, and String::from_utf8_lossy is
                // how byte soup reaches a parser in practice).
                String::from_utf8_lossy(&buf).into_owned()
            };
            check(&input);
        }
    }
}
