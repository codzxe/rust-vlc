//! SRT inline markup (card 5.M1): the HTML-ish tag subset real SRT
//! files carry, mapped onto the model's style-token subset (5.L2).
//!
//! Supported (mapped, loss-free):
//! `<b>`, `<i>`, `<u>` + their closing tags (case-insensitive), and
//! `<font color="#rrggbb" | "#rgb" | "#rrggbbaa">` / `</font>`.
//!
//! Degraded honestly (doc 06 §7.8 — counted, never fatal):
//! * unknown tags (`<sub>`, `<ruby>`, …) → stripped +
//!   [`SrtDamageKind::UnknownTag`];
//! * a `<` with no closing `>` → kept literally +
//!   [`SrtDamageKind::UnterminatedTag`];
//! * the named entities (`&amp; &lt; &gt; &quot; &apos; &nbsp;`) and
//!   numeric forms (`&#65;`, `&#x41;`) decode; unknown entities stay
//!   literal + [`SrtDamageKind::UnknownEntity`];
//! * a `<font>` color that is not a supported hex literal → style
//!   dropped + [`SrtDamageKind::BadStyleValue`];
//! * other `<font>` attributes (`face`, `size`) are ignored silently —
//!   they carry no model meaning (the font stack is the renderer's).
//!
//! Output: [`CueText::Plain`] when the text has no effective styling
//! left (the SRT majority — one allocation, D-1),
//! [`CueText::Styled`] otherwise, with runs split at style
//! boundaries; empty runs are not emitted.

use crate::model::cue::{CueText, StyledSegment};
use crate::model::style::{Color, TextStyle};
use crate::srt::damage::{DamageSink, SrtDamageKind};

/// Parse markup in `text` (the block's joined text lines) into a
/// [`CueText`], recording degradations at `line_no`.
#[must_use]
pub fn parse_markup(text: &str, damage: &mut DamageSink, line_no: u32) -> CueText {
    if !text.contains('<') && !text.contains('&') {
        return CueText::Plain(text.to_owned());
    }

    let mut segments: Vec<StyledSegment> = Vec::new();
    let mut current = String::with_capacity(text.len());
    let mut style = TextStyle::PLAIN;

    let mut rest = text;
    while let Some(at) = rest.find(['<', '&']) {
        current.push_str(&rest[..at]);
        let tail = &rest[at..];
        if tail.as_bytes().first() == Some(&b'<') {
            match tail.find('>') {
                Some(gt) => {
                    let before = style.clone();
                    apply_tag(&tail[1..gt], &mut style, damage, line_no);
                    if style != before && !current.is_empty() {
                        segments.push(StyledSegment {
                            text: std::mem::take(&mut current),
                            style: before,
                        });
                    }
                    rest = &tail[gt + 1..];
                }
                None => {
                    // Unterminated: keep the '<' literally and continue
                    // after it (the rest may still hold valid markup).
                    damage.record(SrtDamageKind::UnterminatedTag, line_no);
                    current.push('<');
                    rest = &tail[1..];
                }
            }
        } else {
            match decode_entity(tail, damage, line_no) {
                EntityDecode::Char(c, consumed) => {
                    current.push(c);
                    rest = &tail[consumed..];
                }
                EntityDecode::Literal(consumed) => {
                    current.push_str(&tail[..consumed]);
                    rest = &tail[consumed..];
                }
            }
        }
    }
    current.push_str(rest);

    if segments.is_empty() && style == TextStyle::PLAIN {
        return CueText::Plain(current);
    }
    if !current.is_empty() || segments.is_empty() {
        segments.push(StyledSegment {
            text: current,
            style,
        });
    }
    // A style that was set and unset with nothing in between leaves
    // only plain runs — collapse back to Plain.
    if segments.iter().all(|s| s.style == TextStyle::PLAIN) {
        let joined = segments.iter().map(|s| s.text.as_str()).collect::<String>();
        return CueText::Plain(joined);
    }
    CueText::Styled(segments)
}

/// Apply one tag's body (between `<` and `>`) to the running style.
fn apply_tag(tag: &str, style: &mut TextStyle, damage: &mut DamageSink, line_no: u32) {
    // Tag name = up to the first whitespace; compared
    // case-insensitively without allocating.
    let name_end = tag.find(char::is_whitespace).unwrap_or(tag.len());
    let (name, attrs) = tag.split_at(name_end);

    if name.eq_ignore_ascii_case("b") {
        style.bold = true;
    } else if name.eq_ignore_ascii_case("i") {
        style.italic = true;
    } else if name.eq_ignore_ascii_case("u") {
        style.underline = true;
    } else if name.eq_ignore_ascii_case("/b") {
        style.bold = false;
    } else if name.eq_ignore_ascii_case("/i") {
        style.italic = false;
    } else if name.eq_ignore_ascii_case("/u") {
        style.underline = false;
    } else if name.eq_ignore_ascii_case("/font") {
        style.color = None;
    } else if name.eq_ignore_ascii_case("font") {
        match find_color(attrs) {
            Some(Some(color)) => style.color = Some(color),
            // No color attribute at all (e.g. `<font face="Arial">`):
            // a legal no-op for the model.
            Some(None) => {}
            None => damage.record(SrtDamageKind::BadStyleValue, line_no),
        }
    } else if !name.is_empty() {
        damage.record(SrtDamageKind::UnknownTag, line_no);
    }
    // An empty tag body `<>` is silently dropped (no damage worth a
    // ledger slot; it styles nothing).
}

/// Find the `color` attribute value in a tag's attribute string.
/// `Some(None)` = no color attribute; `None` = color present but not
/// a supported hex literal.
fn find_color(attrs: &str) -> Option<Option<Color>> {
    // Locate "color" (ASCII case-insensitive) as an attribute name.
    // `to_ascii_lowercase` is byte-position-preserving, so indices
    // into `lower_attrs` are valid for `attrs`.
    let lower_attrs = attrs.to_ascii_lowercase();
    let mut from = 0usize;
    let anchor = loop {
        match lower_attrs[from..].find("color") {
            Some(rel) => {
                let at = from + rel;
                let after = at + "color".len();
                let before_ok = at == 0
                    || lower_attrs[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_whitespace() || c == '=');
                let after_ok = lower_attrs[after..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_whitespace() || c == '=');
                if before_ok && after_ok {
                    break at;
                }
                from = after;
            }
            None => return Some(None),
        }
    };
    // Skip ws, '=', ws, then an optional quote and the value.
    let mut v = &attrs[anchor + "color".len()..];
    v = v.trim_start();
    if !v.starts_with('=') {
        return Some(None);
    }
    v = v[1..].trim_start();
    let value = match v.chars().next() {
        Some(q @ ('"' | '\'')) => {
            let inner = &v[1..];
            let end = inner.find(q).unwrap_or(inner.len());
            &inner[..end]
        }
        _ => {
            let end = v.find(char::is_whitespace).unwrap_or(v.len());
            &v[..end]
        }
    };
    parse_hex_color(value).map(Some)
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` (case-insensitive) → [`Color`].
fn parse_hex_color(value: &str) -> Option<Color> {
    let digits = value.strip_prefix('#')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let nib = |b: u8| -> u8 { (b as char).to_digit(16).unwrap_or(0) as u8 };
    match digits.len() {
        3 => {
            let b = digits.as_bytes();
            Some(Color::new(
                nib(b[0]) << 4 | nib(b[0]),
                nib(b[1]) << 4 | nib(b[1]),
                nib(b[2]) << 4 | nib(b[2]),
                255,
            ))
        }
        6 => {
            let b = digits.as_bytes();
            Some(Color::new(
                nib(b[0]) << 4 | nib(b[1]),
                nib(b[2]) << 4 | nib(b[3]),
                nib(b[4]) << 4 | nib(b[5]),
                255,
            ))
        }
        8 => {
            let b = digits.as_bytes();
            Some(Color::new(
                nib(b[0]) << 4 | nib(b[1]),
                nib(b[2]) << 4 | nib(b[3]),
                nib(b[4]) << 4 | nib(b[5]),
                nib(b[6]) << 4 | nib(b[7]),
            ))
        }
        _ => None,
    }
}

/// Entity decode outcome: a decoded char, or literal bytes kept.
enum EntityDecode {
    /// The decoded character and the byte length consumed.
    Char(char, usize),
    /// Keep this many bytes literally (bare `&`, or unknown entity).
    Literal(usize),
}

/// Decode one `&…;` entity at the head of `tail`.
fn decode_entity(tail: &str, damage: &mut DamageSink, line_no: u32) -> EntityDecode {
    // An entity name starts with an ASCII alphanumeric or '#' —
    // anything else ("fish & chips") is a bare `&`: literal, no
    // damage (extremely common in SRT).
    match tail.as_bytes().get(1).copied() {
        Some(b) if b.is_ascii_alphanumeric() || b == b'#' => {}
        _ => return EntityDecode::Literal(1),
    }
    // Only look for a `;` within a small window — a bare `&` followed
    // later by an unrelated `;` must not be eaten as one entity. `;`
    // is ASCII, so byte-position search cannot split a multibyte char.
    let semi_rel = tail
        .bytes()
        .enumerate()
        .take(12)
        .find(|&(_, b)| b == b';')
        .map(|(i, _)| i);
    let Some(semi_rel) = semi_rel else {
        // Bare `&`: literal, no damage (extremely common in SRT).
        return EntityDecode::Literal(1);
    };
    let body = &tail[1..semi_rel];
    // An entity name never contains whitespace — "AT&T &bogus;"
    // must not read the `&…;` span as one entity.
    if body.bytes().any(|b| b.is_ascii_whitespace()) {
        return EntityDecode::Literal(1);
    }
    let consumed = semi_rel + 1;
    if body.is_empty() {
        return EntityDecode::Literal(1);
    }
    let decoded = match body {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        _ => {
            if let Some(num) = body.strip_prefix('#') {
                if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                }
                .and_then(char::from_u32)
            } else {
                None
            }
        }
    };
    match decoded {
        Some(c) => EntityDecode::Char(c, consumed),
        None => {
            damage.record(SrtDamageKind::UnknownEntity, line_no);
            EntityDecode::Literal(consumed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_markup;
    use crate::model::cue::{CueText, StyledSegment};
    use crate::model::style::{Color, TextStyle};
    use crate::srt::damage::{DamageSink, SrtDamageKind};

    fn run(text: &str) -> (CueText, Vec<SrtDamageKind>) {
        let mut sink = DamageSink::new();
        let out = parse_markup(text, &mut sink, 1);
        let (events, _) = sink.into_parts();
        (out, events.into_iter().map(|e| e.kind).collect())
    }

    fn seg(text: &str, style: TextStyle) -> StyledSegment {
        StyledSegment {
            text: text.into(),
            style,
        }
    }

    #[test]
    fn plain_text_stays_plain() {
        let (out, dmg) = run("Hello world\nsecond line");
        assert_eq!(out, CueText::Plain("Hello world\nsecond line".into()));
        assert!(dmg.is_empty());
    }

    #[test]
    fn bold_italic_underline_map_to_segments() {
        let (out, dmg) = run("<b>Bold</b> and <i>italic</i> mix");
        assert!(dmg.is_empty());
        assert_eq!(
            out,
            CueText::Styled(vec![
                seg("Bold", TextStyle::new(true, false, false, None)),
                seg(" and ", TextStyle::PLAIN),
                seg("italic", TextStyle::new(false, true, false, None)),
                seg(" mix", TextStyle::PLAIN),
            ])
        );
        let (out, dmg) = run("<u>Underlined</u> green");
        assert!(dmg.is_empty());
        assert_eq!(
            out,
            CueText::Styled(vec![
                seg("Underlined", TextStyle::new(false, false, true, None)),
                seg(" green", TextStyle::PLAIN),
            ])
        );
    }

    #[test]
    fn font_color_maps_and_bad_values_degrade() {
        let (out, dmg) = run("<font color=\"#00ff00\">green</font> tail");
        assert!(dmg.is_empty());
        assert_eq!(
            out,
            CueText::Styled(vec![
                seg(
                    "green",
                    TextStyle::new(false, false, false, Some(Color::new(0, 255, 0, 255)))
                ),
                seg(" tail", TextStyle::PLAIN),
            ])
        );
        let (_, dmg) = run("<font color=red>named</font>");
        assert_eq!(dmg, vec![SrtDamageKind::BadStyleValue]);
        let (_, dmg) = run("<font color='#0f0'>short</font>");
        assert!(dmg.is_empty());
        let (_, dmg) = run("<font face=\"Arial\" size=\"20\">attrs</font>");
        assert!(dmg.is_empty());
        // Set then unset with nothing between collapses to Plain.
        let (out, dmg) = run("a<font color=\"#ff0000\"></font>b");
        assert!(dmg.is_empty());
        assert_eq!(out, CueText::Plain("ab".into()));
    }

    #[test]
    fn unknown_and_unterminated_tags() {
        let (out, dmg) = run("a <sub>junk</sub> b");
        assert_eq!(out, CueText::Plain("a junk b".into()));
        assert_eq!(
            dmg,
            vec![SrtDamageKind::UnknownTag, SrtDamageKind::UnknownTag]
        );
        let (out, dmg) = run("1 < 2");
        assert_eq!(out, CueText::Plain("1 < 2".into()));
        assert_eq!(dmg, vec![SrtDamageKind::UnterminatedTag]);
    }

    #[test]
    fn entities_decode_or_stay_literal() {
        let (out, dmg) = run("a &amp; b &lt;c&gt; &quot;q&quot; &#65;&#x42;");
        assert_eq!(out, CueText::Plain("a & b <c> \"q\" AB".into()));
        assert!(dmg.is_empty());
        let (out, dmg) = run("AT&T &bogus; end");
        assert_eq!(out, CueText::Plain("AT&T &bogus; end".into()));
        assert_eq!(dmg, vec![SrtDamageKind::UnknownEntity]);
        let (out, dmg) = run("&nbsp;x");
        assert_eq!(out, CueText::Plain("\u{00A0}x".into()));
        assert!(dmg.is_empty());
        // A bare & far from any ; stays literal without damage.
        let (out, dmg) = run("fish & chips; later");
        assert_eq!(out, CueText::Plain("fish & chips; later".into()));
        assert!(dmg.is_empty());
    }

    #[test]
    fn style_returning_to_plain_still_splits() {
        let (out, _) = run("<i>a</i> plain tail");
        assert_eq!(
            out,
            CueText::Styled(vec![
                seg("a", TextStyle::new(false, true, false, None)),
                seg(" plain tail", TextStyle::PLAIN),
            ])
        );
    }
}
