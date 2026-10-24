//! Cue-text decoding (card 5.M3): WebVTT inline markup is stripped to
//! plain text — every tag (`<v Speaker>`, `<i>`, `</b>`, `<c.class>`,
//! `<00:00:01.000>` karaoke stamps, …) disappears, and the five named
//! escapes plus numeric character references are decoded.
//!
//! Order matters and is fixed: **strip tags first, decode escapes
//! second** — `&lt;i&gt;` must survive as the three characters
//! `<i>`, and `&amp;lt;` must decode exactly once to `&lt;` (a single
//! non-recursive pass, HTML semantics). A `<` with no closing `>` on
//! the line is malformed; the tolerant choice is to drop the rest of
//! the payload from there (per spec it cannot be cue text).
//!
//! Styling is DEFERED, like the ASS route (doc 06 §7.8 honesty): the
//! model's [`CueText::Styled`](crate::model::CueText::Styled) exists,
//! but WebVTT inline styling only arrives when a later card maps the
//! tag subset onto [`TextStyle`](crate::model::TextStyle) — the same
//! staged plan as ASS's `{\...}` overrides. Speaker names in `<v>`
//! tags are dropped with the tag.

/// Decode a cue's text lines (joined with `\n` by the caller) to the
/// plain payload.
pub(super) fn decode_cue_text(joined: &str) -> String {
    let without_tags = strip_tags(joined);
    decode_entities(&without_tags)
}

/// Remove every `<...>` tag. Tags cannot span lines in WebVTT, but we
/// tolerate the whole document being one string: an unterminated `<`
/// truncates the remainder (documented tolerance, never a panic).
fn strip_tags(s: &str) -> String {
    if !s.contains('<') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let after = &rest[lt + 1..];
        match after.find('>') {
            Some(gt) => rest = &after[gt + 1..],
            None => {
                // Unterminated tag: drop the tail.
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One pass of escape decoding. Anything that is not a *complete,
/// valid* reference is copied through literally (tolerance over
/// spec-strict rejection — dirty-world files routinely carry bare
/// `&`s).
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match decode_one(after) {
            Some((ch, consumed)) => {
                out.push(ch);
                rest = &after[consumed..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Try to decode the entity starting at the beginning of `s` (after
/// the `&`); returns the replacement char and the bytes it consumed.
fn decode_one(s: &str) -> Option<(char, usize)> {
    let semi = s.find(';')?;
    let name = &s[..semi];
    let consumed = semi + 1;
    match name {
        "amp" => Some(('&', consumed)),
        "lt" => Some(('<', consumed)),
        "gt" => Some(('>', consumed)),
        "nbsp" => Some(('\u{00a0}', consumed)),
        "lrm" => Some(('\u{200e}', consumed)),
        "rlm" => Some(('\u{200f}', consumed)),
        _ => {
            if let Some(digits) = name.strip_prefix('#') {
                let value = if let Some(hex) = digits
                    .strip_prefix('x')
                    .or_else(|| digits.strip_prefix('X'))
                {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    digits.parse::<u32>().ok()?
                };
                let ch = char::from_u32(value)?;
                // Control characters and the BOM cannot ride a text
                // channel; everything else passes.
                if ch.is_control() || ch == '\u{feff}' {
                    return None;
                }
                Some((ch, consumed))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::decode_cue_text;

    #[test]
    fn strips_common_tags() {
        assert_eq!(decode_cue_text("Hello <i>world</i>!"), "Hello world!");
        assert_eq!(decode_cue_text("<v Fred>Hi there"), "Hi there");
        assert_eq!(
            decode_cue_text("<v Mary Waves>Hello <b>bold</b> <u>under</u>"),
            "Hello bold under"
        );
        assert_eq!(decode_cue_text("<c.yellow>Moon</c>"), "Moon");
        assert_eq!(
            decode_cue_text("sync <00:00:01.000>karaoke"),
            "sync karaoke"
        );
        assert_eq!(decode_cue_text("<ruby>kanji<rt>reading</rt></ruby>"), "kanjireading");
    }

    #[test]
    fn decodes_named_and_numeric_escapes() {
        assert_eq!(decode_cue_text("a &amp; b"), "a & b");
        assert_eq!(decode_cue_text("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_cue_text("x &nbsp; y"), "x \u{00a0} y");
        assert_eq!(decode_cue_text("&lrm;&rlm;"), "\u{200e}\u{200f}");
        assert_eq!(decode_cue_text("&#65;&#x42;"), "AB");
        assert_eq!(decode_cue_text("&#x1F600;"), "\u{1f600}");
    }

    #[test]
    fn escapes_decode_exactly_once_and_order_is_tags_first() {
        // &amp;lt; → "&lt;" (literal), NOT "<" — single pass.
        assert_eq!(decode_cue_text("&amp;lt;"), "&lt;");
        // Escaped tags must not be stripped: tags go first, escapes
        // second, so &lt;i&gt; survives as text.
        assert_eq!(decode_cue_text("&lt;i&gt;real&lt;/i&gt;"), "<i>real</i>");
    }

    #[test]
    fn malformed_input_is_tolerated() {
        assert_eq!(decode_cue_text("a & b &notanentity; c"), "a & b &notanentity; c");
        assert_eq!(decode_cue_text("&#999999999999;"), "&#999999999999;");
        assert_eq!(decode_cue_text("&#xD800;"), "&#xD800;");
        assert_eq!(decode_cue_text("&#0;"), "&#0;");
        // Unterminated tag truncates; unbalanced close is inert.
        assert_eq!(decode_cue_text("keep <oops dropped"), "keep ");
        assert_eq!(decode_cue_text("just </close>"), "just ");
        // Multi-line payloads keep their newlines.
        assert_eq!(decode_cue_text("line one\n<b>line</b> two"), "line one\nline two");
        // Empty stays empty.
        assert_eq!(decode_cue_text(""), "");
    }
}
