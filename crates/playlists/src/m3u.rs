//! Extended/plain M3U and the HLS `#EXT-X-*` media-playlist subset
//! (cards 4.D1 + 5.O1a), plus RFC 3986 URI joining.
//!
//! # Format model
//!
//! * Line-based text (LF or CRLF terminators; a trailing `\r` is stripped).
//!   A leading UTF-8 BOM is tolerated and skipped.
//! * `#EXTM3U` — header; optional and ignored wherever it appears (a
//!   headerless file is a legal *plain* M3U; 5.O1 pins this as
//!   intentional tolerance, not accident).
//! * `#EXTINF:<duration>[,<title>]` — metadata for the **next** location
//!   line. `duration` is seconds, fractions included (`6.006000` parses
//!   exactly); `-1` = the spec's unknown marker, preserved verbatim;
//!   `title` is everything after the first comma.
//! * Any other `#EXT<NAME>:<value>` line (e.g. `#EXTVLCOPT:…`,
//!   `#EXTGRP:…`, unknown tags) is preserved as a directive on the next
//!   entry, in source order.
//! * Any other `#…` line is a comment and is ignored — including comments
//!   that happen to contain `:`, which are *not* directives.
//! * Blank (whitespace-only) lines are ignored.
//! * Everything else is a location line, kept verbatim after line-level
//!   whitespace trimming: URLs, Windows paths, Unix paths, relative paths.
//!   No resolution or normalization happens here — [`resolve_uri`] is a
//!   separate, explicit call.
//!
//! # HLS media-playlist subset (5.O1a)
//!
//! The v1 subset of media-playlist tags is parsed into typed fields on
//! [`Entry::hls`] / [`Playlist::end_list`] — **recorded, not resolved**
//! (no fetching, no decryption, no scheduling; the streaming scheduler is
//! Phase 6):
//!
//! * `#EXT-X-BYTERANGE:<n>[@<o>]` → [`ByteRange`] on the next segment
//!   (offset kept verbatim; the "continues the previous range" rule for
//!   an absent `@o` is the consumer's to apply).
//! * `#EXT-X-DISCONTINUITY` → `discontinuity: true` on the next segment.
//! * `#EXT-X-KEY:<attrs>` → [`HlsKey`] (method + URI + IV/keyformat,
//!   verbatim). Stateful: applies to every following segment until the
//!   next `#EXT-X-KEY`; `METHOD=NONE` clears it. The parser stamps the
//!   *effective* key on each segment.
//! * `#EXT-X-MAP:<attrs>` → [`HlsMap`] (URI + optional quoted
//!   `BYTERANGE="n[@o]"`), stateful like `KEY`.
//! * `#EXT-X-ENDLIST` → `Playlist::end_list = true`.
//! * Unknown `#EXT-X-*` tags (`VERSION`, `TARGETDURATION`,
//!   `MEDIA-SEQUENCE`, `PLAYLIST-TYPE`, …) stay ordered directives —
//!   preserved verbatim, never invented into fields we do not model.
//!
//! Attribute lists follow the HLS spec's value types (quoted-string,
//! decimal-integer, hexadecimal-sequence, enumerated). Commas inside
//! quoted strings do not split; unknown attribute names are ignored
//! (spec-compliant); duplicate/malformed attributes are dirty-world
//! damage ([`ParseWarning::BadExtXTag`], line skipped).
//!
//! # Dirty-world behavior (directives D-1/D-2)
//!
//! * Malformed `#EXTINF` (bad/empty/non-finite duration, missing colon)
//!   degrades to a typed [`ParseWarning`]; the entry still parses with the
//!   damaged field absent — entry-not-death.
//! * Malformed `#EXT-X-*` tags (broken attribute list, missing required
//!   attribute, junk on a valueless tag) degrade to
//!   [`ParseWarning::BadExtXTag`] with a line-level skip — parsing
//!   continues with prior state intact.
//! * Metadata still pending at EOF (no following location) is dropped with
//!   a [`ParseWarning::DanglingEntry`].
//! * A line longer than [`MAX_LINE_BYTES`] is fatal
//!   ([`ParseError::LineTooLong`]) — D-1 bounded input.
//! * Non-UTF-8 bytes are fatal ([`ParseError::InvalidUtf8`]) via
//!   [`parse_bytes`]; the parser never guesses an encoding.
//! * The parser never panics on any byte input (pinned by fuzz-lite).
//!
//! # URI resolution (5.O1a)
//!
//! [`resolve_uri`] implements the RFC 3986 §5 relative-reference join
//! (merge + dot-segment removal) as an explicit, side-effect-free
//! function — parse stays verbatim, resolution is the caller's policy.
//! Windows drive paths (`C:\…`, `C:/…`) and UNC paths (`\\…`) are not URI
//! references and are returned verbatim rather than mangled into fake
//! URLs; POSIX-absolute references against an http base join at the
//! server root per RFC (a leading `/` means root — that is the spec, not
//! a path-normalization opinion).
//!
//! # Writer
//!
//! [`write`] emits canonical extended M3U: the `#EXTM3U` header, then per
//! entry its directives (in order), the `#EXT-X-KEY`/`#EXT-X-MAP` lines
//! when they *change* (delta-encoded — re-parsing stamps the same
//! effective state), `#EXT-X-DISCONTINUITY`, `#EXT-X-BYTERANGE`, one
//! `#EXTINF` line when duration or title is present, then the location;
//! `#EXT-X-ENDLIST` closes a playlist flagged `end_list`. The roundtrip
//! law `parse(write(parse(x))) == parse(x)` is pinned by tests for every
//! input that parses, including the dirty corpus and ffmpeg-generated
//! HLS fixtures.

use crate::error::{ParseError, ParseWarning};
use crate::types::{ByteRange, EncryptionMethod, Entry, HlsKey, HlsMap, HlsSegment, Playlist};

/// Maximum accepted line length in bytes, terminator excluded (D-1). A
/// longer line is rejected with [`ParseError::LineTooLong`]. 64 KiB is far
/// beyond any sane playlist line while bounding per-line work.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// A successful parse: the playlist plus every dirty-world warning seen on
/// the way (warnings never alter the entry set — they only name dropped
/// metadata).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The parsed playlist.
    pub playlist: Playlist,
    /// Non-fatal findings, in source order.
    pub warnings: Vec<ParseWarning>,
}

/// Parses an M3U playlist, discarding warnings.
pub fn parse(input: &str) -> Result<Playlist, ParseError> {
    Ok(parse_detailed(input)?.playlist)
}

/// Parses an M3U playlist, reporting dirty-world warnings alongside it.
pub fn parse_detailed(input: &str) -> Result<Parsed, ParseError> {
    // BOM tolerance: exactly one leading U+FEFF. A BOM elsewhere is just
    // (weird) line content — kept, not guessed at.
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    let mut entries: Vec<Entry> = Vec::new();
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut end_list = false;

    // Metadata pending for the next location line.
    let mut duration: Option<f64> = None;
    let mut title: Option<String> = None;
    let mut directives: Vec<(String, String)> = Vec::new();
    let mut pending_since: Option<usize> = None;
    let mark_pending = |line_no: usize, pending_since: &mut Option<usize>| {
        if pending_since.is_none() {
            *pending_since = Some(line_no);
        }
    };

    // HLS state (5.O1a): stateful tags carry forward; per-segment tags
    // wait for the next location line.
    let mut cur_key: Option<HlsKey> = None;
    let mut cur_map: Option<HlsMap> = None;
    let mut pend_byte_range: Option<ByteRange> = None;
    let mut pend_discontinuity = false;

    let mut rest: &str = input;
    let mut line_no: usize = 0;
    while !rest.is_empty() {
        line_no += 1;
        let (raw, tail): (&str, &str) = match rest.as_bytes().iter().position(|&b| b == b'\n') {
            Some(idx) => {
                let (line, after) = rest.split_at(idx);
                (line, &after[1..])
            }
            None => (rest, ""),
        };
        if raw.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong {
                line: line_no,
                length: raw.len(),
            });
        }
        rest = tail;

        // CRLF tolerance, then line-level whitespace trim (locations keep
        // their internal bytes verbatim).
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(after_hash) = trimmed.strip_prefix('#') {
            if after_hash == "EXTM3U" {
                // Header (tolerated anywhere, tolerated repeated).
                continue;
            }
            if let Some(after_tag) = after_hash.strip_prefix("EXTINF") {
                let Some(body) = after_tag.strip_prefix(':') else {
                    warnings.push(ParseWarning::ExtinfMissingColon { line: line_no });
                    continue;
                };
                let (dur_text, title_text) = match body.find(',') {
                    Some(i) => (&body[..i], Some(&body[i + 1..])),
                    None => (body, None),
                };
                let dur = parse_duration(dur_text);
                if dur.is_none() {
                    warnings.push(ParseWarning::BadDuration { line: line_no });
                }
                duration = dur;
                title = title_text.map(str::to_string).filter(|t| !t.is_empty());
                mark_pending(line_no, &mut pending_since);
            } else if after_hash.starts_with("EXT-X-") {
                // 5.O1a HLS subset. Split name/value once, then dispatch.
                let (name, value) = match after_hash.split_once(':') {
                    Some((n, v)) => (n, Some(v)),
                    None => (after_hash, None),
                };
                let value_ok = |v: Option<&str>| v.is_some_and(|v| !v.is_empty());
                match name {
                    "EXT-X-DISCONTINUITY" => {
                        if value_ok(value) {
                            warnings.push(ParseWarning::BadExtXTag { line: line_no });
                        } else {
                            pend_discontinuity = true;
                            mark_pending(line_no, &mut pending_since);
                        }
                    }
                    "EXT-X-ENDLIST" => {
                        // Playlist terminator, not entry metadata: never
                        // marks pending (a VOD file legitimately ends with
                        // it after the last segment).
                        if value_ok(value) {
                            warnings.push(ParseWarning::BadExtXTag { line: line_no });
                        } else {
                            end_list = true;
                        }
                    }
                    "EXT-X-BYTERANGE" => {
                        match value.filter(|v| !v.is_empty()).and_then(parse_byte_range) {
                            Some(br) => {
                                pend_byte_range = Some(br);
                                mark_pending(line_no, &mut pending_since);
                            }
                            None => warnings.push(ParseWarning::BadExtXTag { line: line_no }),
                        }
                    }
                    "EXT-X-KEY" => {
                        match value.filter(|v| !v.is_empty()).and_then(parse_ext_x_key) {
                            Some(key) => {
                                cur_key = key;
                                mark_pending(line_no, &mut pending_since);
                            }
                            None => warnings.push(ParseWarning::BadExtXTag { line: line_no }),
                        }
                    }
                    "EXT-X-MAP" => {
                        match value.filter(|v| !v.is_empty()).and_then(parse_ext_x_map) {
                            Some(map) => {
                                cur_map = Some(map);
                                mark_pending(line_no, &mut pending_since);
                            }
                            None => warnings.push(ParseWarning::BadExtXTag { line: line_no }),
                        }
                    }
                    _ => {
                        // Unknown EXT-X-* tag: preserved as a directive,
                        // exactly like any other unknown #EXT tag.
                        match value {
                            Some(v) => {
                                directives.push((name.to_string(), v.to_string()));
                                mark_pending(line_no, &mut pending_since);
                            }
                            None => {
                                warnings
                                    .push(ParseWarning::DirectiveMissingColon { line: line_no });
                            }
                        }
                    }
                }
            } else if after_hash.starts_with("EXT") {
                // Other (possibly unknown) directive: #EXT<NAME>:<value>.
                match after_hash.find(':') {
                    Some(i) if i > 0 => {
                        directives
                            .push((after_hash[..i].to_string(), after_hash[i + 1..].to_string()));
                        mark_pending(line_no, &mut pending_since);
                    }
                    Some(_) => {
                        // "#EXT:" — no name; treat as an ignored comment.
                    }
                    None => {
                        warnings.push(ParseWarning::DirectiveMissingColon { line: line_no });
                    }
                }
            }
            // Any other '#' line is a plain comment: ignored.
        } else {
            // Location line: flush pending metadata into a new entry.
            entries.push(Entry {
                location: trimmed.to_string(),
                duration,
                title,
                directives,
                hls: HlsSegment {
                    byte_range: pend_byte_range.take(),
                    discontinuity: pend_discontinuity,
                    key: cur_key.clone(),
                    map: cur_map.clone(),
                },
            });
            duration = None;
            title = None;
            directives = Vec::new();
            pending_since = None;
            pend_discontinuity = false;
        }
    }

    if let Some(line) = pending_since {
        warnings.push(ParseWarning::DanglingEntry { line });
    }

    Ok(Parsed {
        playlist: Playlist { entries, end_list },
        warnings,
    })
}

/// Parses raw bytes as an M3U playlist. Non-UTF-8 input is a typed error;
/// every byte input either returns or errors — never panics (fuzz-pinned).
pub fn parse_bytes(input: &[u8]) -> Result<Parsed, ParseError> {
    let text = std::str::from_utf8(input).map_err(|_| ParseError::InvalidUtf8)?;
    parse_detailed(text)
}

/// Writes a playlist as canonical extended M3U (see the module docs).
/// Fields originating from [`parse`] contain no terminators and
/// spec-shaped attribute values, so the output is well-formed by
/// construction. `KEY`/`MAP` lines are emitted only where the effective
/// value changes (delta encoding) — canonical HLS shape, and the
/// roundtrip law holds because re-parsing re-stamps every segment.
#[must_use]
pub fn write(playlist: &Playlist) -> String {
    let mut out = String::from("#EXTM3U\n");
    let mut prev_key: Option<&HlsKey> = None;
    let mut prev_map: Option<&HlsMap> = None;
    for entry in &playlist.entries {
        for (name, value) in &entry.directives {
            out.push('#');
            out.push_str(name);
            out.push(':');
            out.push_str(value);
            out.push('\n');
        }
        let key = entry.hls.key.as_ref();
        if key != prev_key {
            match key {
                Some(k) => write_ext_x_key(&mut out, k),
                None => out.push_str("#EXT-X-KEY:METHOD=NONE\n"),
            }
            prev_key = key;
        }
        let map = entry.hls.map.as_ref();
        if map != prev_map {
            if let Some(m) = map {
                out.push_str("#EXT-X-MAP:URI=\"");
                out.push_str(&m.uri);
                out.push('"');
                if let Some(br) = &m.byte_range {
                    out.push_str(",BYTERANGE=\"");
                    write_byte_range(&mut out, br);
                    out.push('"');
                }
                out.push('\n');
            }
            prev_map = map;
        }
        if entry.hls.discontinuity {
            out.push_str("#EXT-X-DISCONTINUITY\n");
        }
        if let Some(br) = &entry.hls.byte_range {
            out.push_str("#EXT-X-BYTERANGE:");
            write_byte_range(&mut out, br);
            out.push('\n');
        }
        if entry.duration.is_some() || entry.title.is_some() {
            out.push_str("#EXTINF:");
            if let Some(d) = entry.duration {
                // Shortest round-trip decimal: `123.0` prints as `123`,
                // `123.5` as `123.5`; both re-parse exactly.
                out.push_str(&format!("{d}"));
            }
            if let Some(t) = &entry.title {
                out.push(',');
                out.push_str(t);
            }
            out.push('\n');
        }
        out.push_str(&entry.location);
        out.push('\n');
    }
    if playlist.end_list {
        out.push_str("#EXT-X-ENDLIST\n");
    }
    out
}

/// `#EXT-X-KEY:METHOD=…,URI="…"[,IV=0x…][,KEYFORMAT="…"][,…VERSIONS="…"]`
/// in the canonical attribute order.
fn write_ext_x_key(out: &mut String, key: &HlsKey) {
    out.push_str("#EXT-X-KEY:METHOD=");
    out.push_str(match &key.method {
        EncryptionMethod::Aes128 => "AES-128",
        EncryptionMethod::SampleAes => "SAMPLE-AES",
        EncryptionMethod::Other(m) => m,
    });
    out.push_str(",URI=\"");
    out.push_str(&key.uri);
    out.push('"');
    if let Some(iv) = &key.iv {
        out.push_str(",IV=0x");
        for b in iv {
            out.push_str(&format!("{b:02x}"));
        }
    }
    if let Some(kf) = &key.keyformat {
        out.push_str(",KEYFORMAT=\"");
        out.push_str(kf);
        out.push('"');
    }
    if let Some(kfv) = &key.keyformat_versions {
        out.push_str(",KEYFORMATVERSIONS=\"");
        out.push_str(kfv);
        out.push('"');
    }
    out.push('\n');
}

/// `<length>` or `<length>@<offset>`.
fn write_byte_range(out: &mut String, br: &ByteRange) {
    out.push_str(&br.length.to_string());
    if let Some(o) = br.offset {
        out.push('@');
        out.push_str(&o.to_string());
    }
}

// ------------------------------------------------------ HLS attribute lists

/// One parsed attribute value — the HLS spec's value types (5.O1a subset:
/// quoted-string, decimal-integer, hexadecimal-sequence, enumerated).
#[derive(Debug, Clone, PartialEq)]
enum AttrValue {
    /// `"…"` — content kept verbatim; no escape mechanism exists in HLS.
    Quoted(String),
    /// Unsigned decimal integer.
    Int(u64),
    /// `0x`/`0X`-prefixed hexadecimal sequence, decoded to bytes.
    Hex(Vec<u8>),
    /// Unquoted token (e.g. `AES-128`, `VOD`), kept verbatim.
    Enumerated(String),
}

impl AttrValue {
    fn as_quoted(&self) -> Option<&str> {
        match self {
            AttrValue::Quoted(s) => Some(s),
            _ => None,
        }
    }
    fn as_hex(&self) -> Option<&[u8]> {
        match self {
            AttrValue::Hex(h) => Some(h),
            _ => None,
        }
    }
}

/// Parses an HLS attribute list: comma-separated `Name=Value` pairs.
/// Commas inside quoted strings do not split. Names are `[A-Za-z0-9-]+`
/// per the spec; a name may not appear twice (spec MUST NOT — malformed).
/// Returns `None` on any syntax damage (empty list, missing `=`,
/// unterminated quote, junk after a closing quote, non-numeric integer,
/// odd-length hex, overflow).
fn parse_attribute_list(input: &str) -> Option<Vec<(String, AttrValue)>> {
    let mut attrs: Vec<(String, AttrValue)> = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        // Name: up to the '=' that must not sit inside a value (values
        // come after the '=', so a '"' before '=' is already malformed).
        let eq = rest.find('=')?;
        let name = &rest[..eq];
        let name_ok =
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !name_ok {
            return None;
        }
        if attrs.iter().any(|(n, _)| n == name) {
            return None; // spec: each attribute appears at most once
        }
        let (value, after) = scan_attr_value(&rest[eq + 1..])?;
        attrs.push((name.to_string(), value));
        match after.strip_prefix(',') {
            Some("") => return None, // trailing comma
            Some(next) => rest = next,
            None if after.is_empty() => break,
            None => return None, // unreachable: scan stops only at ',' or end
        }
    }
    if attrs.is_empty() {
        return None;
    }
    Some(attrs)
}

/// Scans one value from the start of `s` — a quoted string (up to the
/// closing `"`, which must be followed by end-of-list or `,`) or a bare
/// token up to the next `,`, typed as integer / hex / enumerated.
fn scan_attr_value(s: &str) -> Option<(AttrValue, &str)> {
    if let Some(after_quote) = s.strip_prefix('"') {
        let end = after_quote.find('"')?;
        let content = &after_quote[..end];
        let after = &after_quote[end + 1..];
        if after.is_empty() || after.starts_with(',') {
            return Some((AttrValue::Quoted(content.to_string()), after));
        }
        return None; // junk after the closing quote
    }
    let (raw, after) = match s.find(',') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    if raw.is_empty() {
        return None;
    }
    if raw.bytes().all(|b| b.is_ascii_digit()) {
        return Some((AttrValue::Int(raw.parse::<u64>().ok()?), after));
    }
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        let bytes = decode_hex(hex)?;
        return Some((AttrValue::Hex(bytes), after));
    }
    Some((AttrValue::Enumerated(raw.to_string()), after))
}

/// Hex digits (even count) → bytes. `None` on odd length or non-hex chars.
fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.as_chunks::<2>().0 {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

/// Strict unsigned decimal (non-empty, ASCII digits only, no overflow).
fn parse_decimal(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok()
}

/// `#EXT-X-BYTERANGE:<n>[@<o>]` — recorded verbatim.
fn parse_byte_range(value: &str) -> Option<ByteRange> {
    let (n, o) = match value.split_once('@') {
        Some((n, o)) => (n, Some(o)),
        None => (value, None),
    };
    Some(ByteRange {
        length: parse_decimal(n)?,
        offset: match o {
            Some(text) => Some(parse_decimal(text)?),
            None => None,
        },
    })
}

/// `#EXT-X-KEY:<attrs>` → `Some(None)` for `METHOD=NONE` (clears the
/// effective key), `Some(Some(key))` for a real key, `None` when the line
/// is malformed (caller warns + skips, keeping prior state).
fn parse_ext_x_key(value: &str) -> Option<Option<HlsKey>> {
    let attrs = parse_attribute_list(value)?;
    let mut method: Option<String> = None;
    let mut uri: Option<&str> = None;
    let mut iv: Option<&[u8]> = None;
    let mut keyformat: Option<&str> = None;
    let mut keyformat_versions: Option<&str> = None;
    for (name, val) in &attrs {
        match name.as_str() {
            // Unknown attribute names are ignored (spec-compliant).
            "METHOD" => method = Some(val.as_enumerated()?.to_string()),
            "URI" => uri = Some(val.as_quoted()?),
            "IV" => iv = Some(val.as_hex()?),
            "KEYFORMAT" => keyformat = Some(val.as_quoted()?),
            "KEYFORMATVERSIONS" => keyformat_versions = Some(val.as_quoted()?),
            _ => {}
        }
    }
    let method = method?;
    if method == "NONE" {
        // Spec: METHOD=NONE MUST NOT carry a URI. Contradictory input is
        // damage, not a key.
        if uri.is_some() {
            return None;
        }
        return Some(None);
    }
    let method = match method.as_str() {
        "AES-128" => EncryptionMethod::Aes128,
        "SAMPLE-AES" => EncryptionMethod::SampleAes,
        other => EncryptionMethod::Other(other.to_string()),
    };
    // URI is REQUIRED for every non-NONE method.
    let uri = uri?.to_string();
    Some(Some(HlsKey {
        method,
        uri,
        iv: iv.map(<[u8]>::to_vec),
        keyformat: keyformat.map(str::to_string),
        keyformat_versions: keyformat_versions.map(str::to_string),
    }))
}

/// `#EXT-X-MAP:<attrs>` — `URI` (quoted, required) and `BYTERANGE`
/// (quoted `"<n>[@<o>]"`, optional).
fn parse_ext_x_map(value: &str) -> Option<HlsMap> {
    let attrs = parse_attribute_list(value)?;
    let mut uri: Option<&str> = None;
    let mut byte_range: Option<ByteRange> = None;
    for (name, val) in &attrs {
        match name.as_str() {
            "URI" => uri = Some(val.as_quoted()?),
            "BYTERANGE" => byte_range = Some(parse_byte_range(val.as_quoted()?)?),
            _ => {}
        }
    }
    Some(HlsMap {
        uri: uri?.to_string(),
        byte_range,
    })
}

impl AttrValue {
    fn as_enumerated(&self) -> Option<&str> {
        match self {
            AttrValue::Enumerated(s) => Some(s),
            _ => None,
        }
    }
}

/// Duration text → finite seconds. Empty, malformed and non-finite
/// (`inf`, `nan`, overflow like `1e999`) all return `None` — the caller
/// turns that into a `BadDuration` warning (D-2: honest degradation).
fn parse_duration(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.parse::<f64>() {
        Ok(d) if d.is_finite() => Some(d),
        _ => None,
    }
}

// ------------------------------------------------------------- URI joining

/// Resolves a playlist location against a base URI — the RFC 3986 §5
/// relative-reference join (merge + dot-segment removal), the minimal
/// honest subset (5.O1a).
///
/// * An absolute reference (`http://…`, `mailto:…`) is returned verbatim.
/// * `../`/`./`/bare-relative references merge onto the base's directory
///   and have their dot segments removed (`..` never climbs above root).
/// * A leading `/` joins at the base's root (server root for URLs — the
///   RFC meaning, not a path opinion); `//…` reuses the base's scheme.
/// * Windows drive paths (`C:\…`, `C:/…`) and UNC paths (`\\…`) are not
///   URI references: returned verbatim, never mangled into fake URLs.
///   Separators are never rewritten in either direction (the
///   Windows-vs-POSIX note: `C:\a\b` stays backslashed, `/a/b` stays
///   POSIX).
/// * A local-path base (no scheme) works too: `resolve_uri("d/l.m3u",
///   "s.mp3") == "d/s.mp3"`.
///
/// Query/fragment handling follows §5.3: a reference's query replaces the
/// base's; the base's fragment never survives; an empty reference returns
/// the base minus its fragment.
#[must_use]
pub fn resolve_uri(base: &str, reference: &str) -> String {
    // Windows drive/UNC references are filesystem paths, not URI
    // references — verbatim (see the module docs).
    if is_windows_path(reference) {
        return reference.to_owned();
    }
    // Absolute reference (valid scheme): verbatim.
    if uri_scheme(reference).is_some() {
        return reference.to_owned();
    }

    let ref_no_frag = up_to_hash(reference);
    let (r_path, r_query) = match ref_no_frag.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (ref_no_frag, None),
    };
    let r_frag = after_hash(reference);

    // Empty or fragment-only reference: base minus its fragment (RFC §5.3).
    if r_path.is_empty() && r_query.is_none() {
        let mut out = up_to_hash(base).to_owned();
        if let Some(f) = r_frag {
            out.push_str(f);
        }
        return out;
    }

    // Network-path reference: reuse the base's scheme.
    if let Some(net) = ref_no_frag.strip_prefix("//") {
        return match uri_scheme(base) {
            Some((scheme, _)) => {
                let mut out = format!("{scheme}://{net}");
                if let Some(f) = r_frag {
                    out.push_str(f);
                }
                out
            }
            // Scheme-less base: nothing to join onto — verbatim (the
            // reference keeps its own query and fragment).
            None => reference.to_owned(),
        };
    }

    let b = split_base(base);
    let (path, query): (String, Option<&str>) = if r_path.is_empty() {
        // Query-only reference ("?y"): keep the base path.
        (b.path.to_owned(), r_query)
    } else if r_path.starts_with('/') {
        // Root-relative: replaces the whole path.
        (remove_dot_segments(r_path), r_query)
    } else if b.has_authority && b.path.is_empty() {
        (remove_dot_segments(&format!("/{r_path}")), r_query)
    } else {
        // Merge onto the base's directory, then remove dot segments.
        let dir = match b.path.rfind('/') {
            Some(i) => &b.path[..=i],
            None => "",
        };
        (remove_dot_segments(&format!("{dir}{r_path}")), r_query)
    };

    let mut out = String::with_capacity(base.len() + reference.len());
    out.push_str(b.prefix);
    out.push_str(&path);
    if let Some(q) = query {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = r_frag {
        out.push_str(f);
    }
    out
}

/// `s` up to (not including) the first `#`; the whole string if none.
fn up_to_hash(s: &str) -> &str {
    match s.find('#') {
        Some(i) => &s[..i],
        None => s,
    }
}

/// The `#…` suffix of `s` (including the `#`), if any.
fn after_hash(s: &str) -> Option<&str> {
    s.find('#').map(|i| &s[i..])
}

/// A URI scheme prefix: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`
/// followed by `:`. Returns the scheme and the text after the colon.
/// Single-letter schemes are syntactically legal per RFC 3986, but a
/// single letter followed by `/` or `\` is a Windows drive — that case
/// must be filtered by the caller before this is meaningful.
fn uri_scheme(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            if i == 0 {
                return None;
            }
            return Some((&s[..i], &s[i + 1..]));
        }
        if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
            return None;
        }
    }
    None
}

/// Windows drive path (`X:` + `/` or `\`) or UNC path (`\\…`).
fn is_windows_path(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'\\' && b[1] == b'\\' {
        return true;
    }
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
}

/// A base URI split just far enough for joining: everything before the
/// path is kept as one `prefix` string (`scheme://authority`, `scheme:`,
/// or empty for a scheme-less local-path base).
struct BaseUri<'a> {
    prefix: &'a str,
    /// Path component (no query, no fragment).
    path: &'a str,
    has_authority: bool,
}

fn split_base(base: &str) -> BaseUri<'_> {
    let no_frag = up_to_hash(base);
    let path_q = match no_frag.split_once('?') {
        Some((p, _)) => p,
        None => no_frag,
    };
    let (scheme, after_scheme) = match uri_scheme(path_q) {
        Some((s, rest)) => (Some(s), rest),
        None => (None, path_q),
    };
    if let Some(rest) = after_scheme.strip_prefix("//") {
        // Authority runs to the first '/' (query and fragment are already
        // stripped, so the remainder is all path).
        match rest.find('/') {
            Some(i) => {
                let auth_end = path_q.len() - rest.len() + i;
                BaseUri {
                    prefix: &path_q[..auth_end],
                    path: &path_q[auth_end..],
                    has_authority: true,
                }
            }
            None => BaseUri {
                prefix: path_q,
                path: "",
                has_authority: true,
            },
        }
    } else {
        let scheme_len = scheme.map_or(0, |s| s.len() + 1);
        BaseUri {
            prefix: &path_q[..scheme_len],
            path: &path_q[scheme_len..],
            has_authority: false,
        }
    }
}

/// RFC 3986 §5.2.4 dot-segment removal, as a segment stack (equivalent
/// to the spec's algorithm for absolute paths; also sane for the
/// scheme-less local-path bases this crate supports, where `..` may
/// consume the entire path). An empty trailing segment is a trailing
/// slash, preserved exactly — `a///` stays `a///` (only `.`/`..`
/// segments collapse, never literal ones).
fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut stack: Vec<&str> = Vec::new();
    let segments: Vec<&str> = path.split('/').collect();
    for (i, seg) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        match *seg {
            // "." contributes nothing; as the final segment it still
            // denotes a directory (one trailing slash).
            "." => {
                if last {
                    stack.push("");
                }
            }
            ".." => {
                stack.pop();
                if last {
                    stack.push("");
                }
            }
            "" => {
                if i == 0 && absolute {
                    // The root slash's own empty first segment.
                } else {
                    // Interior ("a//b" keeps its empty segment) and the
                    // trailing slash ("a/") alike.
                    stack.push("");
                }
            }
            s => stack.push(s),
        }
    }
    let mut out = String::with_capacity(path.len());
    if absolute {
        out.push('/');
    }
    out.push_str(&stack.join("/"));
    // A relative path consumed entirely ("a/..") normalizes to "." per
    // POSIX; an absolute one is already exactly "/".
    if out.is_empty() && !absolute {
        out.push('.');
    }
    out
}
