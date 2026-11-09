//! ASS override-tag styling engine (issue #17, G-2): native
//! interpretation of the tag **minimum** — `{\b} {\i} {\u} {\s} {\fs}`
//! `{\fn} {\c}/{\1c} {\alpha}/{\1a} {\pos} {\move}` — into styled
//! [`CueText`] runs plus an optional position override, clean-room from
//! the published ASS/SSA format description (R-09: no libass code).
//!
//! # Semantics (the honest ledger)
//!
//! * **State seeding** — the working style starts as the referenced
//!   style's projection (`Bold`/`Italic`/`Underline`/`PrimaryColour`,
//!   font name/size); every tag mutates that state from that point on,
//!   exactly like ASS's inline overrides. Runs are emitted with the
//!   *absolute* style in effect (style ⊕ overrides), so a consumer
//!   never re-derives inheritance.
//! * **Booleans** `\b \i \u \s` — `0` off, any nonzero on (libass
//!   renders any nonzero `\b` weight as bold; weights are not modeled
//!   — noted). Malformed arguments are counted into
//!   [`AssStats::dropped_tags`] and ignored, never a parse failure.
//! * **`\fs`** — exact fixed-point thousandths via the parser's
//!   `parse_decimal_milli` (no floats, MP §8); `<= 0` is malformed
//!   (counted, ignored).
//! * **`\fn`** — the argument up to the next `\` (parenthesized form
//!   accepted too); an empty argument resets to *inherit* (`None`),
//!   the documented libass reading of a bare `{\fn}`.
//! * **Colours** `\c`/`\1c` — `&H`-literal or decimal, BGR byte order,
//!   ASS inverted alpha (the parser's `parse_ass_colour`). A literal
//!   with MORE than 6 hex digits carries an explicit alpha byte and
//!   sets it; a 6-digit literal sets only RGB, leaving any `\alpha`
//!   override intact (libass keeps colour and alpha separately).
//! * **`\alpha`/`\1a`** — `&HXX` inverted-alpha (or decimal) applied to
//!   the primary colour's alpha channel.
//! * **`{\2c} {\3c} {\4c} {\2a} {\3a} {\4a}`** — parsed and validated,
//!   then honestly unmapped: the model's single-colour subset has no
//!   secondary/outline/shadow slots. Counted into
//!   [`AssStats::unmapped_colour_tags`], not `dropped_tags` (they were
//!   *understood*, just not representable).
//! * **`\pos(x,y)`** — script-resolution coordinates; the LAST `\pos`
//!   in an event wins (multiple `\pos` is undefined in the spec; last
//!   is the conservative choice, pinned by test).
//! * **`\move(x1,y1,x2,y2[,t1,t2])`** — the motion is NOT
//!   interpolated (the cue model is static): the anchor freezes at the
//!   **start point** `(x1,y1)` and the tag is counted into
//!   [`AssStats::move_tags_frozen`]. `[VERIFY]` against libass
//!   rendering once the real font stack lands.
//! * Everything else (`\k`, `\t`, `\frz`, `\r`, `\an`, `\fad`, …)
//!   stays in the Phase-5 honest-degrade bucket: counted into
//!   [`AssStats::dropped_tags`] by name.
//!
//! The text walk keeps the parser's exact VSFilter/libass quirks: the
//! literal `{}` pair is preserved; an unterminated `{` is literal text;
//! `\N`/`\n` fold to `'\n'`; `\h` to `' '`; UTF-8 scalars are copied
//! whole. A cue whose every run ends up equal to the seed style is
//! emitted as [`CueText::Plain`] (D-1: the tag-free majority pays no
//! run vector).

use crate::model::cue::{CueText, StyledSegment};
use crate::model::style::{Color, TextStyle};

use super::parser::{AssStats, parse_ass_colour, parse_decimal_milli};

/// An explicit position override (`\pos`, or `\move` frozen at its
/// start point), in script (PlayRes) units as exact thousandths of a
/// pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionOverride {
    /// Horizontal script coordinate, milli-px.
    pub x_milli: i64,
    /// Vertical script coordinate, milli-px.
    pub y_milli: i64,
    /// `true` when the source was `\move` (frozen at the start point).
    pub from_move: bool,
}

/// The working override state: the seeded style plus a tracked alpha
/// so `\alpha` survives a later 6-digit `\c` (libass keeps the two
/// channels independently).
#[derive(Debug, Clone, PartialEq)]
struct WorkState {
    style: TextStyle,
    alpha: Option<u8>,
}

impl WorkState {
    fn new(seed: TextStyle) -> Self {
        Self {
            style: seed,
            alpha: None,
        }
    }

    /// Materialize the emit-time style: the tracked alpha, if any,
    /// stamped onto the current colour.
    fn materialize(&self) -> TextStyle {
        match (self.style.clone(), self.alpha) {
            (mut s, Some(a)) => {
                s.color = Some(match s.color {
                    Some(c) => Color::new(c.r, c.g, c.b, a),
                    // Colour not set yet: an alpha alone still has to
                    // ride somewhere — the alpha channel of a black
                    // placeholder colour is the least-wrong carrier
                    // (\c will overwrite RGB later or the seed wins).
                    None => Color::new(0, 0, 0, a),
                });
                s
            }
            (s, None) => s,
        }
    }
}

/// The tag-name table, longest-first: the scanner matches the
/// LONGEST known name at each backslash (the libass table-lookup
/// reading) so `\fnComic Sans` is tag `fn` + argument `Comic Sans`,
/// not the tag `fnComic`. Beyond the G-2 applied minimum the extra
/// names exist purely for honest per-name attribution of the
/// dropped-tag counters (`\clip` counts as `clip`, not a malformed
/// `\c`).
const KNOWN_TAGS: [&str; 52] = [
    // 5
    "alpha", "iclip", "xbord", "ybord", "xshad", "yshad",
    // 4
    "clip", "fscx", "fscy", "bord", "shad", "blur", "move", "fade",
    // 3
    "pos", "fsp", "frz", "frx", "fry", "fax", "fay", "org", "fad", "rnd", "pbo",
    // 2
    "fs", "fn", "fe", "be", "an", "kf", "ko", "1c", "2c", "3c", "4c", "1a", "2a", "3a", "4a",
    // 1
    "b", "i", "u", "s", "c", "k", "K", "t", "r", "q", "a", "p",
];

/// Interpret one override block's interior, mutating `state` and
/// recording every tag into `stats` (applied, unmapped or dropped).
fn apply_block(block: &str, state: &mut WorkState, stats: &mut AssStats, pos: &mut Option<PositionOverride>) {
    let bytes = block.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &block[i + 1..];
        // Longest known tag name at this position wins.
        let known = KNOWN_TAGS.iter().find(|name| rest.starts_with(**name));
        let (name, arg): (&str, &str) = match known {
            Some(name) => {
                let arg_at = i + 1 + name.len();
                let name = &block[i + 1..arg_at];
                let arg = read_arg(block, arg_at);
                (name, arg.0)
            }
            // Unknown tag: consume the name run for the honest
            // per-name counter (letters; digit-led runs are
            // alphanumeric — the `\1c` family shape).
            None => {
                let mut j = i + 1;
                if bytes.get(i + 1).is_some_and(|b| b.is_ascii_digit()) {
                    while j < bytes.len() && bytes[j].is_ascii_alphanumeric() {
                        j += 1;
                    }
                } else {
                    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                }
                if j == i + 1 {
                    // A bare `\\` (e.g. `\\\\` or `\\{`): not a tag.
                    i = j;
                    continue;
                }
                let name = &block[i + 1..j];
                let (arg, next) = read_arg(block, j);
                let _ = next;
                stats.count_tag(name);
                // Skip the whole tag (argument included) — one count.
                i = skip_arg(block, j);
                let _ = arg;
                continue;
            }
        };
        // For known tags, advance past the argument.
        let arg_at = i + 1 + name.len();
        let next = skip_arg(block, arg_at);
        apply_tag(name, arg, state, stats, pos);
        i = next;
    }
}

/// The argument span after a tag name: parenthesized tags read to
/// the first `)` (commas live inside); everything else reads to the
/// next `\\` or block end — NEVER past a following tag.
fn read_arg<'a>(block: &'a str, at: usize) -> (&'a str, usize) {
    let bytes = block.as_bytes();
    if bytes.get(at) == Some(&b'(') {
        match block[at + 1..].find(')') {
            Some(close) => (&block[at + 1..at + 1 + close], at + close + 2),
            // Unterminated `(`: the rest of the block, lenient.
            None => (&block[at + 1..], bytes.len()),
        }
    } else {
        let end = block[at..]
            .find('\\')
            .map_or(bytes.len(), |p| at + p);
        (&block[at..end], end)
    }
}

/// The position just past a tag's argument (same span rule as
/// [`read_arg`], without materializing the slice).
fn skip_arg(block: &str, at: usize) -> usize {
    read_arg(block, at).1
}

/// Dispatch one parsed `\name(arg)` tag.
fn apply_tag(
    name: &str,
    arg: &str,
    state: &mut WorkState,
    stats: &mut AssStats,
    pos: &mut Option<PositionOverride>,
) {
    let applied = match name {
        "b" | "i" | "u" | "s" => {
            match arg.trim().parse::<i64>() {
                Ok(v) => {
                    let on = v != 0;
                    match name {
                        "b" => state.style.bold = on,
                        "i" => state.style.italic = on,
                        "u" => state.style.underline = on,
                        _ => state.style.strikeout = on,
                    }
                    true
                }
                // A bare `{\b}` toggles nothing we can express
                // truthfully — count it dropped rather than guess.
                Err(_) => {
                    stats.count_tag(name);
                    return;
                }
            }
        }
        "fs" => {
            match parse_decimal_milli(arg) {
                Some(m) if m > 0 => {
                    state.style.font_size_milli = Some(m);
                    true
                }
                _ => {
                    stats.count_tag("fs");
                    return;
                }
            }
        }
        "fn" => {
            // Empty argument: reset to inherit (the style's font).
            if arg.is_empty() {
                state.style.font_name = None;
            } else {
                state.style.font_name = Some(arg.to_owned().into_boxed_str());
            }
            true
        }
        "c" | "1c" => {
            match parse_tag_colour(arg) {
                Some((c, alpha_explicit)) => {
                    state.style.color = Some(match (state.style.color, alpha_explicit) {
                        (Some(cur), false) => Color::new(c.r, c.g, c.b, cur.a),
                        _ => c,
                    });
                    true
                }
                None => {
                    stats.count_tag(name);
                    return;
                }
            }
        }
        "alpha" | "1a" => {
            match parse_tag_alpha(arg) {
                Some(a) => {
                    state.alpha = Some(a);
                    true
                }
                None => {
                    stats.count_tag(name);
                    return;
                }
            }
        }
        // Understood but not representable in the single-colour
        // subset — counted separately from the dropped zoo.
        "2c" | "3c" | "4c" | "2a" | "3a" | "4a" => {
            let ok = matches!(name.as_bytes().last(), Some(b'c'))
                && parse_ass_colour(arg).is_some()
                || matches!(name.as_bytes().last(), Some(b'a')) && parse_tag_alpha(arg).is_some();
            if !ok {
                stats.count_tag(name);
            } else {
                stats.unmapped_colour_tags += 1;
            }
            return;
        }
        "pos" => {
            match parse_coords(arg) {
                Some((x, y)) => {
                    *pos = Some(PositionOverride {
                        x_milli: x,
                        y_milli: y,
                        from_move: false,
                    });
                    true
                }
                None => {
                    stats.count_tag("pos");
                    return;
                }
            }
        }
        "move" => {
            match parse_move_args(arg) {
                Some((x1, y1)) => {
                    *pos = Some(PositionOverride {
                        x_milli: x1,
                        y_milli: y1,
                        from_move: true,
                    });
                    stats.move_tags_frozen += 1;
                    true
                }
                None => {
                    stats.count_tag("move");
                    return;
                }
            }
        }
        _ => {
            // Not in the G-2 minimum: the Phase-5 honest-degrade
            // bucket, counted by name.
            stats.count_tag(name);
            return;
        }
    };
    if applied {
        stats.applied_override_tags += 1;
    }
}

/// A colour literal for `\c`: the parser's ASS decoder, plus whether
/// the literal carried an EXPLICIT alpha byte (>6 hex digits or a
/// high decimal) — a 6-digit literal leaves the current alpha alone.
fn parse_tag_colour(arg: &str) -> Option<(Color, bool)> {
    let trimmed = arg.trim();
    let color = parse_ass_colour(trimmed)?;
    let alpha_explicit = match trimmed.strip_prefix("&H").or_else(|| trimmed.strip_prefix("&h")) {
        Some(hex) => hex.trim_end_matches('&').len() > 6,
        None => {
            // Decimal form: alpha explicit when it does not fit 24
            // bits (the RGB-only domain).
            trimmed.parse::<i64>().ok().map_or(true, |v| !(0..=0xFF_FFFF).contains(&v))
        }
    };
    Some((color, alpha_explicit))
}

/// An alpha literal for `\alpha`: `&HXX` (inverted, `00` = opaque) or
/// a decimal 0..=255, converted to straight alpha.
fn parse_tag_alpha(arg: &str) -> Option<u8> {
    let trimmed = arg.trim();
    let ass_alpha: u8 = if let Some(hex) = trimmed
        .strip_prefix("&H")
        .or_else(|| trimmed.strip_prefix("&h"))
    {
        let hex = hex.trim_end_matches('&');
        if hex.is_empty() || hex.len() > 2 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u8::from_str_radix(hex, 16).ok()?
    } else {
        trimmed.parse::<i64>().ok().and_then(|v| u8::try_from(v).ok())?
    };
    Some(255 - ass_alpha)
}

/// `\pos(x,y)` — two exact decimal coordinates (milli-px).
fn parse_coords(arg: &str) -> Option<(i64, i64)> {
    let mut parts = arg.split(',');
    let x = parse_decimal_milli(parts.next()?)?;
    let y = parse_decimal_milli(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

/// `\move(x1,y1,x2,y2[,t1,t2])` — validate the full argument list,
/// return the frozen start point.
fn parse_move_args(arg: &str) -> Option<(i64, i64)> {
    let parts: Vec<&str> = arg.split(',').collect();
    if parts.len() != 4 && parts.len() != 6 {
        return None;
    }
    let x1 = parse_decimal_milli(parts[0])?;
    let y1 = parse_decimal_milli(parts[1])?;
    // The rest must parse (honest validation) even though the motion
    // is frozen at the start point.
    for p in &parts[2..] {
        parse_decimal_milli(p)?;
    }
    Some((x1, y1))
}

/// Walk an event's text: strip/interpret override blocks, fold the
/// escape family, and emit styled runs. Returns the cue text (Plain
/// when nothing diverged from the seed) and the position override if
/// a `\pos`/`\move` was applied.
pub(super) fn styled_event_text(
    text: &str,
    seed: TextStyle,
    stats: &mut AssStats,
) -> (CueText, Option<PositionOverride>) {
    let mut state = WorkState::new(seed.clone());
    let mut pos: Option<PositionOverride> = None;
    let mut segments: Vec<StyledSegment> = Vec::new();
    let mut current = String::new();
    let mut seg_style = state.materialize();

    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // The exact empty pair `{}` is preserved (VSFilter
                // quirk); any other `{...}` with a closing brace is an
                // override block; an unterminated `{` is literal.
                if bytes.get(i + 1) == Some(&b'}') {
                    current.push_str("{}");
                    i += 2;
                    continue;
                }
                let Some(close) = bytes[i + 1..].iter().position(|&b| b == b'}') else {
                    current.push('{');
                    i += 1;
                    continue;
                };
                apply_block(&text[i + 1..i + 1 + close], &mut state, stats, &mut pos);
                stats.stripped_override_blocks += 1;
                i += close + 2;
                // Close the open run if the effective style changed,
                // so segments stay style-homogeneous.
                let now = state.materialize();
                if now != seg_style {
                    if !current.is_empty() {
                        segments.push(StyledSegment {
                            text: std::mem::take(&mut current),
                            style: seg_style,
                        });
                    }
                    seg_style = now;
                }
            }
            b'\\' => match bytes.get(i + 1) {
                Some(b'N') => {
                    current.push('\n');
                    stats.hard_newlines += 1;
                    i += 2;
                }
                Some(b'n') => {
                    current.push('\n');
                    stats.soft_newlines += 1;
                    i += 2;
                }
                Some(b'h') => {
                    current.push(' ');
                    stats.hard_spaces += 1;
                    i += 2;
                }
                _ => {
                    current.push('\\');
                    i += 1;
                }
            },
            _ => {
                // Copy one UTF-8 scalar — never split a code point.
                let len = utf8_len(bytes[i]);
                let end = (i + len).min(bytes.len());
                current.push_str(&text[i..end]);
                i = end;
            }
        }
    }
    if !current.is_empty() {
        segments.push(StyledSegment {
            text: current,
            style: seg_style,
        });
    }

    // Nothing diverged from the seed → the plain payload (D-1).
    if segments.iter().all(|s| s.style == seed) {
        let joined: String = segments.iter().map(|s| s.text.as_str()).collect();
        return (CueText::Plain(joined), pos);
    }
    (CueText::Styled(segments), pos)
}

/// Byte length of the UTF-8 scalar starting at `first` (callers only
/// land on scalar boundaries).
const fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::{PositionOverride, styled_event_text};
    use crate::ass::parser::AssStats;
    use crate::model::cue::CueText;
    use crate::model::style::{Color, TextStyle};

    fn run(text: &str, seed: TextStyle) -> (CueText, Option<PositionOverride>, AssStats) {
        let mut stats = AssStats::default();
        let (cue, pos) = styled_event_text(text, seed, &mut stats);
        (cue, pos, stats)
    }

    fn seg(text: &str, style: TextStyle) -> crate::model::cue::StyledSegment {
        crate::model::cue::StyledSegment { text: text.into(), style }
    }

    #[test]
    fn tag_free_text_stays_plain() {
        let (cue, pos, stats) = run("plain text\\Ntwo lines", TextStyle::PLAIN);
        assert_eq!(cue, CueText::Plain("plain text\ntwo lines".into()));
        assert_eq!(pos, None);
        assert_eq!(stats.applied_override_tags, 0);
        assert_eq!(stats.dropped_tags.len(), 0);
    }

    #[test]
    fn boolean_tags_flip_runs() {
        let (cue, _, stats) = run("{\\b1}Bold{\\b0} plain{\\i1}it", TextStyle::PLAIN);
        assert_eq!(
            cue,
            CueText::Styled(vec![
                seg("Bold", TextStyle::new(true, false, false, None)),
                seg(" plain", TextStyle::PLAIN),
                seg("it", TextStyle::new(false, true, false, None)),
            ])
        );
        assert_eq!(stats.applied_override_tags, 3);
        // \b and \i are no longer "dropped" — they were applied.
        assert_eq!(stats.dropped_tags.get("b"), None);
    }

    #[test]
    fn strikeout_underline_and_multi_tag_blocks() {
        // Several tags in ONE block, applied left to right.
        let (cue, _, _) = run("{\\u1\\s1}both", TextStyle::PLAIN);
        assert_eq!(
            cue,
            CueText::Styled(vec![seg(
                "both",
                TextStyle::new(false, false, true, None).with_strikeout(true)
            )])
        );
    }

    #[test]
    fn boolean_weights_nonzero_is_on() {
        let (cue, _, _) = run("{\\b600}w", TextStyle::PLAIN);
        assert_eq!(
            cue,
            CueText::Styled(vec![seg("w", TextStyle::new(true, false, false, None))])
        );
        // Malformed boolean argument: counted dropped, ignored.
        let (cue2, _, stats) = run("{\\bq}x", TextStyle::PLAIN);
        assert_eq!(cue2, CueText::Plain("x".into()));
        assert_eq!(stats.dropped_tags.get("b"), Some(&1));
    }

    #[test]
    fn fs_and_fn_override_font_fields() {
        let seed = TextStyle::PLAIN
            .with_font_size_milli(Some(16_000))
            .with_font_name(Some("Arial"));
        let (cue, _, _) = run("{\\fs32.5}big{\\fnComic Sans}fonty{\\fn}reset", seed);
        let CueText::Styled(segs) = cue else {
            panic!("styled");
        };
        assert_eq!(segs[0].style.font_size_milli, Some(32_500));
        assert_eq!(segs[0].style.font_name.as_deref(), Some("Arial"));
        assert_eq!(segs[1].style.font_name.as_deref(), Some("Comic Sans"));
        // Empty \fn resets to inherit.
        assert_eq!(segs[2].style.font_name, None);
        // Malformed \fs (negative, garbage): dropped + ignored.
        let (_, _, stats) = run("{\\fs-4}x", TextStyle::PLAIN);
        assert_eq!(stats.dropped_tags.get("fs"), Some(&1));
        let (_, _, stats) = run("{\\fsq}x", TextStyle::PLAIN);
        assert_eq!(stats.dropped_tags.get("fs"), Some(&1));
    }

    #[test]
    fn colour_and_alpha_tags() {
        let seed = TextStyle::new(false, false, false, Some(Color::WHITE));
        let (cue, _, _) = run("{\\c&H0000FF&}red", seed.clone());
        // &H0000FF → BGR: B=00 G=00 R=FF → opaque red.
        assert_eq!(
            cue,
            CueText::Styled(vec![seg(
                "red",
                TextStyle::new(false, false, false, Some(Color::new(255, 0, 0, 255)))
            )])
        );
        // 6-digit \c keeps a previously set alpha.
        let (cue, _, _) = run("{\\alpha&H80&}{\\c&H0000FF&}red", seed.clone());
        let CueText::Styled(s) = cue else { panic!() };
        // &H80 = 128 inverted → straight 127.
        assert_eq!(s[0].style.color, Some(Color::new(255, 0, 0, 127)));
        // An 8-digit \c carries its own (inverted) alpha: &H80 alpha
        // byte → straight 127.
        let (cue, _, _) = run("{\\c&H800000FF&}red", seed);
        let CueText::Styled(s) = cue else { panic!() };
        assert_eq!(s[0].style.color, Some(Color::new(255, 0, 0, 127)));
        // Malformed colour: dropped + ignored.
        let (_, _, stats) = run("{\\c&Hzz&}x", TextStyle::PLAIN);
        assert_eq!(stats.dropped_tags.get("c"), Some(&1));
    }

    #[test]
    fn secondary_colour_slots_are_unmapped_not_dropped() {
        let (_, _, stats) = run("{\\3c&HFFFFFF&}{\\2a&H40&}x", TextStyle::PLAIN);
        assert_eq!(stats.unmapped_colour_tags, 2);
        assert_eq!(stats.dropped_tags.get("3c"), None);
        // Malformed secondary colour IS dropped (it was not even a
        // valid literal).
        let (_, _, stats) = run("{\\4c&Hzz&}x", TextStyle::PLAIN);
        assert_eq!(stats.dropped_tags.get("4c"), Some(&1));
    }

    #[test]
    fn pos_and_move_overrides() {
        let (_, pos, _) = run("{\\pos(192,144)}x", TextStyle::PLAIN);
        assert_eq!(
            pos,
            Some(PositionOverride {
                x_milli: 192_000,
                y_milli: 144_000,
                from_move: false
            })
        );
        // Decimal coordinates parse exactly.
        let (_, pos, _) = run("{\\pos(10.5,20.25)}x", TextStyle::PLAIN);
        assert_eq!(
            pos,
            Some(PositionOverride {
                x_milli: 10_500,
                y_milli: 20_250,
                from_move: false
            })
        );
        // \move freezes at the start point, counted.
        let (_, pos, stats) = run("{\\move(10,20,30,40)}x", TextStyle::PLAIN);
        assert_eq!(
            pos,
            Some(PositionOverride {
                x_milli: 10_000,
                y_milli: 20_000,
                from_move: true
            })
        );
        assert_eq!(stats.move_tags_frozen, 1);
        // 6-arg \move validates all coordinates.
        let (_, pos, _) = run("{\\move(1,2,3,4,0,1000)}x", TextStyle::PLAIN);
        assert_eq!(pos.map(|p| (p.x_milli, p.y_milli)), Some((1_000, 2_000)));
        // The LAST \pos wins (pinned policy).
        let (_, pos, _) = run("{\\pos(1,2)}a{\\pos(3,4)}b", TextStyle::PLAIN);
        assert_eq!(pos.map(|p| (p.x_milli, p.y_milli)), Some((3_000, 4_000)));
        // Malformed: wrong arity, garbage.
        let (_, pos, stats) = run("{\\pos(1)}x", TextStyle::PLAIN);
        assert_eq!(pos, None);
        assert_eq!(stats.dropped_tags.get("pos"), Some(&1));
        let (_, _, stats) = run("{\\move(1,2,3)}x", TextStyle::PLAIN);
        assert_eq!(stats.dropped_tags.get("move"), Some(&1));
    }

    #[test]
    fn unsupported_tags_stay_dropped_and_counted() {
        let (cue, _, stats) = run("{\\k20}ka{\\frz30}ra{\\t(0,100,\\fs20)}oke", TextStyle::PLAIN);
        assert_eq!(cue, CueText::Plain("karaoke".into()));
        assert_eq!(stats.dropped_tags.get("k"), Some(&1));
        assert_eq!(stats.dropped_tags.get("frz"), Some(&1));
        assert_eq!(stats.dropped_tags.get("t"), Some(&1));
        assert_eq!(stats.applied_override_tags, 0);
    }

    #[test]
    fn text_quirks_survive_the_styling_walk() {
        // {} preserved, unterminated { literal, \h space, UTF-8 whole.
        let (cue, _, _) = run("a{}b{ oops\\N日本語\\h", TextStyle::PLAIN);
        assert_eq!(cue, CueText::Plain("a{}b{ oops\n日本語 ".into()));
        // Tags reset per event: the seed is the referenced style.
        let seed = TextStyle::new(true, false, false, Some(Color::WHITE))
            .with_font_name(Some("Arial"))
            .with_font_size_milli(Some(16_000));
        let (cue, _, _) = run("styled by seed", seed.clone());
        assert_eq!(cue, CueText::Plain("styled by seed".into()));
        // \b0 diverging from a bold seed DOES split runs.
        let (cue, _, _) = run("on{\\b0}off", seed.clone());
        assert_eq!(
            cue,
            CueText::Styled(vec![
                seg("on", seed),
                seg("off", TextStyle::new(false, false, false, Some(Color::WHITE))
                    .with_font_name(Some("Arial"))
                    .with_font_size_milli(Some(16_000))),
            ])
        );
    }
}
