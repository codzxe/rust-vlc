//! Typed HLS **media-playlist** model (card 6.B1a) — the bridge from
//! the recorded 5.O1a parse to the Phase-6 streaming session.
//!
//! The 4.D1/5.O1a M3U parser is deliberately *recording*: it keeps
//! `#EXT-X-*` tags as verbatim directives and stamps the segment-level
//! subset (`KEY`/`MAP`/`BYTERANGE`/`DISCONTINUITY`) on entries, but
//! attaches no streaming semantics. This module is the consumer that
//! **types** that record into an HLS media playlist:
//!
//! * [`parse_media_playlist`] — parse + validate + resolve in one
//!   call: `#EXT-X-VERSION`/`-TARGETDURATION`/`-MEDIA-SEQUENCE`/
//!   `-PLAYLIST-TYPE` are promoted from the directive stream into
//!   typed fields, and every segment URI is resolved against the
//!   caller's base URI (the playlist's own URL — RFC 3986 §5 joining
//!   via [`crate::resolve_uri`]; verbatim locations are preserved when
//!   no base is given).
//! * Refusals are typed, never guessed (MP §8): a **master playlist**
//!   (`#EXT-X-STREAM-INF` / `#EXT-X-MEDIA` where media was expected —
//!   card 6.B2a owns masters), input with **no `#EXTM3U` magic at
//!   all** (that is a plain M3U, not HLS — the magic line is the
//!   format's required header and the sniff signature), and a
//!   **malformed numeric value** on a promoted playlist-level tag.
//! * Everything else stays *recorded, not resolved*: `#EXT-X-KEY`
//!   presence, `BYTERANGE`, `MAP`, `DISCONTINUITY` ride on the
//!   segments; the **session** layer (streaming crate, same card)
//!   owns the play/not-yet decisions (live window, decryption,
//!   discontinuity handling are B1b+ cards) — this module reports, it
//!   does not refuse them.
//!
//! # Dirty-world tolerance
//!
//! The line-level discipline is the parser's (D-2 warnings, line
//! skips). This module adds model-level strictness only where a wrong
//! value would be a *lie*: a non-integer `TARGETDURATION` is a typed
//! [`HlsPlaylistError::BadTagValue`], not a silent `None`. The
//! `#EXTM3U` header scan accepts the magic on any line, matching the
//! parser's own tolerance (the spec says "first line"; the parser
//! deliberately tolerates it anywhere, and the model inherits that
//! posture so a re-serialized playlist always re-validates).
//!
//! I/O-free like the rest of the crate: bytes and base URIs arrive as
//! arguments; nothing is fetched here.

use crate::error::ParseWarning;
use crate::m3u::{Parsed, parse_detailed, resolve_uri};
use crate::types::{ByteRange, HlsKey, HlsMap};

/// A validated HLS media playlist (card 6.B1a): the typed view of a
/// parsed M3U whose `#EXT-X-*` tags say "this is HLS media".
///
/// Constructed by [`parse_media_playlist`]; the streaming session
/// consumes `segments` in order.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaPlaylist {
    /// `#EXT-X-VERSION` (decimal-integer), when present.
    pub version: Option<u64>,
    /// `#EXT-X-TARGETDURATION` (decimal-integer seconds), when
    /// present. The spec marks it REQUIRED for media playlists; this
    /// model records its absence honestly (`None`) rather than
    /// inventing a value — the live-reload cadence that consumes it
    /// is B1b's concern, and VOD playback timing is the container's.
    pub target_duration: Option<u64>,
    /// `#EXT-X-MEDIA-SEQUENCE` (decimal-integer), `0` when absent.
    /// The base sequence number of the first segment in the list.
    pub media_sequence: u64,
    /// `#EXT-X-PLAYLIST-TYPE` enumerated value (`VOD`/`EVENT`),
    /// verbatim, when present.
    pub playlist_type: Option<String>,
    /// `#EXT-X-ENDLIST` was present: the playlist asserts no further
    /// segments will be appended (a VOD/ended playlist). `false`
    /// means live — the B1a session refuses that shape typed.
    pub end_list: bool,
    /// Whether the `#EXTM3U` magic line was seen (required: it is
    /// what makes the file HLS rather than plain M3U).
    pub saw_extm3u: bool,
    /// Sum of the segments' `#EXTINF` durations (advisory only — the
    /// container's own timing is the truth; `0.0` when no EXTINF
    /// carried a finite duration).
    pub total_extinf_duration: f64,
    /// Media segments in playlist order, URIs resolved against the
    /// base when one was given.
    pub segments: Vec<MediaSegment>,
}

/// One media segment: the resolved URI plus everything the playlist
/// recorded for it (all fields are *recorded, not resolved* — the
/// session layer owns play/not-yet decisions).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaSegment {
    /// Segment URI: resolved against the base URI when one was
    /// given, otherwise verbatim as parsed.
    pub uri: String,
    /// `#EXTINF` duration in seconds, when present and finite.
    pub duration: Option<f64>,
    /// `#EXTINF` title, when present.
    pub title: Option<String>,
    /// `#EXT-X-BYTERANGE` in effect (offset kept verbatim — the
    /// "continues the previous range" rule is the consumer's).
    pub byte_range: Option<ByteRange>,
    /// `#EXT-X-KEY` in effect for this segment (recorded; decryption
    /// is a later card).
    pub key: Option<HlsKey>,
    /// `#EXT-X-MAP` in effect for this segment (recorded; fMP4 init
    /// sections are a later lane).
    pub map: Option<HlsMap>,
    /// `#EXT-X-DISCONTINUITY` appeared before this segment.
    pub discontinuity: bool,
}

/// Typed refusal of the media-playlist model (card 6.B1a).
///
/// House style (matching the streaming crate's vocabulary): a
/// thiserror-free enum with stable dotted codes, structured `Display`,
/// and the underlying parse error kept in the `source` chain. Display
/// never embeds URIs, paths, or line text (MP §12).
#[derive(Debug)]
pub enum HlsPlaylistError {
    /// No `#EXTM3U` magic line anywhere in the input
    /// (`playlists.hls.not-hls`): without the header this is a plain
    /// M3U file, not an HLS playlist — the model refuses to guess.
    NotHls,
    /// A master-playlist tag (`#EXT-X-STREAM-INF` or `#EXT-X-MEDIA`)
    /// appeared where a media playlist was expected
    /// (`playlists.hls.master-playlist`). Master playlists are card
    /// 6.B2a (variant selection); the refusal is typed, not silent.
    MasterPlaylist,
    /// A media-playlist shape (`#EXTINF` segment rows, no variant
    /// rows at all) appeared where a master playlist was expected
    /// (`playlists.hls.media-playlist`) — the mirror refusal, typed.
    MediaPlaylist,
    /// A promoted playlist-level tag carried a value that is not a
    /// decimal integer (`playlists.hls.bad-tag-value`) — a wrong
    /// number would be a lie, so the model refuses rather than
    /// recording `None`.
    BadTagValue {
        /// The tag name (e.g. `EXT-X-TARGETDURATION`).
        tag: &'static str,
    },
    /// The underlying M3U parse failed (`playlists.hls.parse-failed`)
    /// — only the D-1 line-cap violation reaches here; the parser's
    /// own dirty-world findings are warnings, not errors.
    Parse {
        /// The parser's typed error, preserved.
        source: crate::error::ParseError,
    },
}

impl HlsPlaylistError {
    /// The stable dotted context code for this refusal.
    pub fn code(&self) -> &'static str {
        match self {
            HlsPlaylistError::NotHls => "playlists.hls.not-hls",
            HlsPlaylistError::MasterPlaylist => "playlists.hls.master-playlist",
            HlsPlaylistError::MediaPlaylist => "playlists.hls.media-playlist",
            HlsPlaylistError::BadTagValue { .. } => "playlists.hls.bad-tag-value",
            HlsPlaylistError::Parse { .. } => "playlists.hls.parse-failed",
        }
    }
}

impl fmt::Display for HlsPlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HlsPlaylistError::NotHls => write!(
                f,
                "playlists: not an HLS playlist ({}): no #EXTM3U header line",
                self.code()
            ),
            HlsPlaylistError::MasterPlaylist => write!(
                f,
                "playlists: HLS master playlist where a media playlist was expected ({}); \
                 variant selection is a later card",
                self.code()
            ),
            HlsPlaylistError::MediaPlaylist => write!(
                f,
                "playlists: HLS media playlist where a master playlist was expected ({})",
                self.code()
            ),
            HlsPlaylistError::BadTagValue { tag } => write!(
                f,
                "playlists: #{} needs a decimal-integer value ({}); refusing to guess",
                tag,
                self.code()
            ),
            HlsPlaylistError::Parse { .. } => write!(
                f,
                "playlists: M3U parse failed before the HLS model could be built ({})",
                self.code()
            ),
        }
    }
}

impl std::error::Error for HlsPlaylistError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HlsPlaylistError::Parse { source } => Some(source),
            _ => None,
        }
    }
}

use std::fmt;

/// Parse `text` as an HLS media playlist, resolving segment URIs
/// against `base` (the playlist's own URL or path) when given.
///
/// Returns the typed model plus the parser's dirty-world warnings (the
/// same [`Parsed::warnings`] the plain M3U parse would report).
///
/// # Errors
/// [`HlsPlaylistError`] — see the variant docs. The B1a playability
/// refusals (live window, encryption, discontinuity, byte-ranges,
/// init sections) are **not** made here: the model records them and
/// the streaming session refuses them typed (card 6.B1a's split —
/// parsing types, the session decides).
pub fn parse_media_playlist(
    base: Option<&str>,
    text: &str,
) -> Result<(MediaPlaylist, Vec<ParseWarning>), HlsPlaylistError> {
    let parsed = parse_detailed(text).map_err(|source| HlsPlaylistError::Parse { source })?;
    let saw_extm3u = text.lines().any(|line| line.trim() == "#EXTM3U");
    let model = media_playlist_from_parsed(base, &parsed, saw_extm3u)?;
    Ok((model, parsed.warnings))
}

/// Build the typed model from an **already-parsed** M3U plus the
/// `#EXTM3U` presence flag (the parser tolerates the header anywhere
/// and does not record it, so the caller — usually
/// [`parse_media_playlist`] — supplies the scan result).
///
/// # Errors
/// [`HlsPlaylistError`] — same set as [`parse_media_playlist`] minus
/// the parse failure (parsing already happened).
pub fn media_playlist_from_parsed(
    base: Option<&str>,
    parsed: &Parsed,
    saw_extm3u: bool,
) -> Result<MediaPlaylist, HlsPlaylistError> {
    if !saw_extm3u {
        return Err(HlsPlaylistError::NotHls);
    }
    let mut version: Option<u64> = None;
    let mut target_duration: Option<u64> = None;
    let mut media_sequence: Option<u64> = None;
    let mut playlist_type: Option<String> = None;

    // Playlist-level tags land as directives on the first location
    // line they precede (the parser's attachment rule); the spec puts
    // them before the first segment, and the first occurrence wins.
    // A master-playlist tag anywhere is decisive: media was expected.
    for entry in &parsed.playlist.entries {
        for (name, value) in &entry.directives {
            match name.as_str() {
                "EXT-X-STREAM-INF" | "EXT-X-MEDIA" => {
                    // Exact-name match only: EXT-X-MEDIA-SEQUENCE is a
                    // media-playlist tag and must not trip this.
                    return Err(HlsPlaylistError::MasterPlaylist);
                }
                "EXT-X-VERSION" => {
                    if version.is_none() {
                        version = Some(parse_tag_uint("EXT-X-VERSION", value)?);
                    }
                }
                "EXT-X-TARGETDURATION" => {
                    if target_duration.is_none() {
                        target_duration = Some(parse_tag_uint("EXT-X-TARGETDURATION", value)?);
                    }
                }
                "EXT-X-MEDIA-SEQUENCE" => {
                    if media_sequence.is_none() {
                        media_sequence = Some(parse_tag_uint("EXT-X-MEDIA-SEQUENCE", value)?);
                    }
                }
                "EXT-X-PLAYLIST-TYPE" if playlist_type.is_none() => {
                    playlist_type = Some(value.trim().to_string());
                }
                _ => {}
            }
        }
    }

    let mut segments = Vec::with_capacity(parsed.playlist.entries.len());
    let mut total_extinf_duration = 0.0_f64;
    for entry in &parsed.playlist.entries {
        let uri = match base {
            Some(base) => resolve_uri(base, &entry.location),
            None => entry.location.clone(),
        };
        if let Some(d) = entry.duration {
            // Only finite, non-negative EXTINF values sum into the
            // advisory total; `-1` (the unknown marker) and negative
            // junk are recorded on the segment but not added.
            if d.is_finite() && d >= 0.0 {
                total_extinf_duration += d;
            }
        }
        segments.push(MediaSegment {
            uri,
            duration: entry.duration,
            title: entry.title.clone(),
            byte_range: entry.hls.byte_range.clone(),
            key: entry.hls.key.clone(),
            map: entry.hls.map.clone(),
            discontinuity: entry.hls.discontinuity,
        });
    }

    Ok(MediaPlaylist {
        version,
        target_duration,
        media_sequence: media_sequence.unwrap_or(0),
        playlist_type,
        end_list: parsed.playlist.end_list,
        saw_extm3u,
        total_extinf_duration,
        segments,
    })
}

/// A promoted tag's value must be a decimal integer (the spec's
/// `decimal-integer` for all four numeric playlist tags).
fn parse_tag_uint(tag: &'static str, value: &str) -> Result<u64, HlsPlaylistError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| HlsPlaylistError::BadTagValue { tag })
}

// ======================================================= 6.B2a: master playlists

/// A typed HLS **master playlist** (card 6.B2a): the variant rows
/// (`#EXT-X-STREAM-INF` + URI) and alternate-rendition rows
/// (`#EXT-X-MEDIA`) of RFC 8216 §4.3.4, parsed by
/// [`parse_master_playlist`].
///
/// The model **records**; the streaming crate's `hls::master` module
/// owns the selection/switching policy (bandwidth cap, hysteresis).
/// Variants keep playlist order (the producer's own ladder order);
/// selection sorts by its own policy, never by mutation here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MasterPlaylist {
    /// Variant streams in playlist order (rows with a parseable
    /// `BANDWIDTH`; damaged rows are skipped with a
    /// [`MasterWarning`], never guessed at).
    pub variants: Vec<Variant>,
    /// Alternate renditions (`#EXT-X-MEDIA`), playlist order.
    pub renditions: Vec<Rendition>,
}

/// One `#EXT-X-STREAM-INF` variant row.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// The variant's media-playlist URI, resolved against the base
    /// when one was given.
    pub uri: String,
    /// `BANDWIDTH` (bits per second) — REQUIRED by RFC 8216
    /// §4.3.4.2; a row without a parseable value is skipped with a
    /// warning rather than assigned a fabricated `0`.
    pub bandwidth: u64,
    /// `AVERAGE-BANDWIDTH`, when present and parseable.
    pub average_bandwidth: Option<u64>,
    /// `PROGRAM-ID` (the legacy Apple attribute, deprecated by the
    /// IETF spec but still emitted by real packagers), when present.
    pub program_id: Option<u64>,
    /// `CODECS` (quoted string), verbatim, when present.
    pub codecs: Option<String>,
    /// `RESOLUTION` (`WxH`), when present and well-formed.
    pub resolution: Option<Resolution>,
}

/// A `RESOLUTION=WxH` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// Width in pixels.
    pub width: u64,
    /// Height in pixels.
    pub height: u64,
}

/// One `#EXT-X-MEDIA` alternate-rendition row.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendition {
    /// `TYPE` — the spec's enumerated set, with unknown values kept
    /// verbatim (`Other`) rather than refused: a future/typo'd TYPE is
    /// recorded data, not a lie about the ones we know.
    pub kind: RenditionType,
    /// `GROUP-ID` (quoted), when present.
    pub group_id: Option<String>,
    /// `NAME` (quoted), when present.
    pub name: Option<String>,
    /// `LANGUAGE` (quoted), when present.
    pub language: Option<String>,
    /// `DEFAULT=YES`.
    pub is_default: bool,
    /// `AUTOSELECT=YES`.
    pub autoselect: bool,
    /// `URI` (quoted), resolved against the base — absent for e.g.
    /// in-stream `CLOSED-CAPTIONS` renditions.
    pub uri: Option<String>,
}

/// The `TYPE` of an [`Rendition`] (RFC 8216 §4.3.4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenditionType {
    /// `TYPE=AUDIO`.
    Audio,
    /// `TYPE=VIDEO`.
    Video,
    /// `TYPE=SUBTITLES`.
    Subtitles,
    /// `TYPE=CLOSED-CAPTIONS`.
    ClosedCaptions,
    /// Any other token, verbatim.
    Other(String),
}

/// A dirty-world finding while typing a master playlist (never fatal:
/// the damaged *row* is skipped, the rest of the master survives —
/// same posture as the M3U parser's warnings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterWarning {
    /// 1-based line number of the damaged row.
    pub line: usize,
    /// What was wrong.
    pub kind: MasterWarningKind,
}

/// What a [`MasterWarning`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterWarningKind {
    /// A `#EXT-X-STREAM-INF` row without a parseable `BANDWIDTH`
    /// (missing, non-numeric, or a damaged attribute list): skipped,
    /// never assigned a fabricated value.
    VariantWithoutBandwidth,
    /// A well-formed attribute value this model does not type (e.g.
    /// a non-`WxH` `RESOLUTION`): the attribute is dropped, the row
    /// survives.
    BadAttributeValue,
    /// A `#EXT-X-MEDIA` row with a damaged attribute list: skipped.
    BadRendition,
}

/// Parse `text` as an HLS **master playlist** (the mirror of
/// [`parse_media_playlist`]): variant rows and rendition rows typed,
/// variant URIs resolved against `base` when given.
///
/// # Errors
/// [`HlsPlaylistError`] — `NotHls` (no `#EXTM3U` anywhere),
/// `MediaPlaylist` (segment lines/`#EXTINF` where variant rows were
/// expected), `Parse` (line-cap violation). Damaged rows are
/// warnings, not errors (dirty-world posture).
pub fn parse_master_playlist(
    base: Option<&str>,
    text: &str,
) -> Result<(MasterPlaylist, Vec<MasterWarning>), HlsPlaylistError> {
    let saw_extm3u = text.lines().any(|line| line.trim() == "#EXTM3U");
    if !saw_extm3u {
        return Err(HlsPlaylistError::NotHls);
    }
    let mut master = MasterPlaylist::default();
    let mut warnings: Vec<MasterWarning> = Vec::new();
    // A STREAM-INF line is pending until the URI line under it.
    let mut pending_stream_inf: Option<(usize, Vec<(String, String)>)> = None;
    let mut saw_media_shape = false;

    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw).trim();
        if line.is_empty() || line == "#EXTM3U" {
            continue;
        }
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            match split_attr_list(attrs) {
                Some(list) => pending_stream_inf = Some((line_no, list)),
                None => warnings.push(MasterWarning {
                    line: line_no,
                    kind: MasterWarningKind::VariantWithoutBandwidth,
                }),
            }
            continue;
        }
        if let Some(attrs) = line.strip_prefix("#EXT-X-MEDIA:") {
            match split_attr_list(attrs) {
                Some(list) => {
                    if let Some(r) = rendition_from_attrs(&list, base) {
                        master.renditions.push(r);
                    } else {
                        warnings.push(MasterWarning {
                            line: line_no,
                            kind: MasterWarningKind::BadRendition,
                        });
                    }
                }
                None => warnings.push(MasterWarning {
                    line: line_no,
                    kind: MasterWarningKind::BadRendition,
                }),
            }
            continue;
        }
        if line.starts_with('#') {
            // Media-playlist-only tags are decisive for the refusal
            // below; everything else is a comment/unknown tag.
            if line.starts_with("#EXTINF") || line.starts_with("#EXT-X-TARGETDURATION") {
                saw_media_shape = true;
            }
            continue;
        }
        // A bare URI line: the variant row's payload.
        if let Some((line_no, attrs)) = pending_stream_inf.take() {
            match variant_from_attrs(&attrs, line, base) {
                Some(v) => master.variants.push(v),
                None => warnings.push(MasterWarning {
                    line: line_no,
                    kind: MasterWarningKind::VariantWithoutBandwidth,
                }),
            }
        }
        // A URI with no pending STREAM-INF is damage the row-skip
        // posture tolerates silently only when variants exist; with
        // none at all the media-shape refusal below stays decisive.
    }

    if master.variants.is_empty() && master.renditions.is_empty() && saw_media_shape {
        return Err(HlsPlaylistError::MediaPlaylist);
    }
    Ok((master, warnings))
}

/// Type one variant row from its attribute list + URI line.
/// `None` = the row must be skipped (no parseable `BANDWIDTH`).
fn variant_from_attrs(
    attrs: &[(String, String)],
    uri: &str,
    base: Option<&str>,
) -> Option<Variant> {
    let bandwidth = attrs
        .iter()
        .find(|(n, _)| n == "BANDWIDTH")
        .and_then(|(_, v)| v.trim().parse::<u64>().ok())?;
    let find = |name: &str| attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v);
    let average_bandwidth = find("AVERAGE-BANDWIDTH").and_then(|v| v.trim().parse::<u64>().ok());
    let program_id = find("PROGRAM-ID").and_then(|v| v.trim().parse::<u64>().ok());
    let codecs = find("CODECS").cloned();
    let resolution = find("RESOLUTION").and_then(|v| {
        let (w, h) = v.split_once('x')?;
        Some(Resolution {
            width: w.trim().parse().ok()?,
            height: h.trim().parse().ok()?,
        })
    });
    let uri = match base {
        Some(base) => resolve_uri(base, uri),
        None => uri.to_owned(),
    };
    Some(Variant {
        uri,
        bandwidth,
        average_bandwidth,
        program_id,
        codecs,
        resolution,
    })
}

/// Type one `#EXT-X-MEDIA` row. `None` = skip the row (only a
/// missing `TYPE` is that fatal here — the spec marks it REQUIRED).
fn rendition_from_attrs(attrs: &[(String, String)], base: Option<&str>) -> Option<Rendition> {
    let find = |name: &str| attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v);
    let kind = match find("TYPE").map(|v| v.to_owned())? {
        t if t == "AUDIO" => RenditionType::Audio,
        t if t == "VIDEO" => RenditionType::Video,
        t if t == "SUBTITLES" => RenditionType::Subtitles,
        t if t == "CLOSED-CAPTIONS" => RenditionType::ClosedCaptions,
        t => RenditionType::Other(t),
    };
    let yes = |name: &str| find(name).is_some_and(|v| v.trim() == "YES");
    let uri = find("URI").map(|u| match base {
        Some(base) => resolve_uri(base, u),
        None => u.to_owned(),
    });
    Some(Rendition {
        kind,
        group_id: find("GROUP-ID").cloned(),
        name: find("NAME").cloned(),
        language: find("LANGUAGE").cloned(),
        is_default: yes("DEFAULT"),
        autoselect: yes("AUTOSELECT"),
        uri,
    })
}

/// Split an HLS attribute list into `(name, value)` pairs (RFC 8216
/// §4.2: comma-separated; commas inside quoted strings do not split;
/// each attribute appears at most once). `None` on structural damage
/// (missing `=`, unterminated quote, trailing comma, duplicate name).
fn split_attr_list(input: &str) -> Option<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        let eq = rest.find('=')?;
        let name = &rest[..eq];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        if out.iter().any(|(n, _)| n == name) {
            return None; // spec: each attribute appears at most once
        }
        let after_eq = &rest[eq + 1..];
        let (value, after) = if let Some(q) = after_eq.strip_prefix('"') {
            let close = q.find('"')?;
            (q[..close].to_owned(), &q[close + 1..])
        } else {
            let end = after_eq.find(',').unwrap_or(after_eq.len());
            (after_eq[..end].to_owned(), &after_eq[end..])
        };
        out.push((name.to_owned(), value));
        rest = match after.strip_prefix(',') {
            Some("") => return None, // trailing comma
            Some(next) => next,
            None if after.is_empty() => break,
            None => return None, // junk after a closing quote
        };
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ParseWarning;

    const VOD: &str = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:2\n\
        #EXT-X-MEDIA-SEQUENCE:5\n\
        #EXT-X-PLAYLIST-TYPE:VOD\n\
        #EXTINF:2.043356,\n\
        seg0.ts\n\
        #EXTINF:1.996922,\n\
        seg1.ts\n\
        #EXT-X-ENDLIST\n";

    /// The promoted tags land in typed fields; URIs resolve against
    /// the base; EXTINF durations sum into the advisory total.
    #[test]
    fn vod_playlist_types_and_resolves() {
        let (model, warnings) =
            match parse_media_playlist(Some("http://cdn.example/vod/pl.m3u8"), VOD) {
                Ok(pair) => pair,
                Err(e) => unreachable!("VOD fixture must type: {e}"),
            };
        assert!(warnings.is_empty());
        assert_eq!(model.version, Some(3));
        assert_eq!(model.target_duration, Some(2));
        assert_eq!(model.media_sequence, 5);
        assert_eq!(model.playlist_type.as_deref(), Some("VOD"));
        assert!(model.end_list);
        assert!(model.saw_extm3u);
        assert_eq!(model.segments.len(), 2);
        assert_eq!(model.segments[0].uri, "http://cdn.example/vod/seg0.ts");
        assert_eq!(model.segments[1].uri, "http://cdn.example/vod/seg1.ts");
        assert!((model.total_extinf_duration - 4.040_278).abs() < 1e-9);
        assert_eq!(model.segments[0].duration, Some(2.043_356));
        assert!(!model.segments[0].discontinuity);
    }

    /// No base: locations stay verbatim (the caller's policy).
    #[test]
    fn no_base_keeps_locations_verbatim() {
        let (model, _) = match parse_media_playlist(None, VOD) {
            Ok(pair) => pair,
            Err(e) => unreachable!("VOD fixture must type: {e}"),
        };
        assert_eq!(model.segments[0].uri, "seg0.ts");
    }

    /// Relative-reference shapes resolve per RFC 3986 §5 (the
    /// resolve_uri seam: parent dirs, absolute paths, absolute URLs).
    #[test]
    fn uri_resolution_shapes() {
        let text = "#EXTM3U\n#EXTINF:1.0,\n../a.ts\n#EXTINF:1.0,\n/b.ts\n\
            #EXTINF:1.0,\nhttp://other.example/c.ts\n#EXT-X-ENDLIST\n";
        let (model, _) = match parse_media_playlist(Some("http://cdn.example/vod/x/pl.m3u8"), text)
        {
            Ok(pair) => pair,
            Err(e) => unreachable!("fixture must type: {e}"),
        };
        assert_eq!(model.segments[0].uri, "http://cdn.example/vod/a.ts");
        assert_eq!(model.segments[1].uri, "http://cdn.example/b.ts");
        assert_eq!(model.segments[2].uri, "http://other.example/c.ts");
    }

    /// No #EXTM3U anywhere: a plain M3U, refused typed.
    #[test]
    fn plain_m3u_refuses_not_hls() {
        let Err(err) = parse_media_playlist(None, "#EXTINF:1.0,\nsong.mp3\n") else {
            unreachable!("plain M3U must refuse NotHls");
        };
        assert!(matches!(err, HlsPlaylistError::NotHls));
        assert_eq!(err.code(), "playlists.hls.not-hls");
    }

    /// A master playlist (variant rows carry #EXT-X-STREAM-INF) is
    /// refused typed with the B2a follow-up named.
    #[test]
    fn master_playlist_refuses_typed() {
        let master = "#EXTM3U\n\
            #EXT-X-STREAM-INF:BANDWIDTH=1280000\n\
            http://cdn.example/v_hi.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=640000\n\
            http://cdn.example/v_lo.m3u8\n";
        let Err(err) = parse_media_playlist(None, master) else {
            unreachable!("master must refuse");
        };
        assert!(matches!(err, HlsPlaylistError::MasterPlaylist));
        assert_eq!(err.code(), "playlists.hls.master-playlist");
        // EXT-X-MEDIA (rendition) is equally decisive…
        let rendition = "#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"en\",URI=\"a/en.m3u8\"\n\
            #EXT-X-STREAM-INF:BANDWIDTH=1280000\n\
            v.m3u8\n";
        let Err(rendition_err) = parse_media_playlist(None, rendition) else {
            unreachable!("rendition master must refuse");
        };
        assert!(matches!(rendition_err, HlsPlaylistError::MasterPlaylist));
        // …but EXT-X-MEDIA-SEQUENCE (media-playlist tag) must NOT
        // trip the master refusal.
        assert!(parse_media_playlist(None, VOD).is_ok());
    }

    /// Malformed numeric values on promoted tags refuse typed.
    #[test]
    fn bad_numeric_values_refuse_typed() {
        for (tag, value) in [
            ("EXT-X-VERSION", "three"),
            ("EXT-X-TARGETDURATION", "2.5s"),
            ("EXT-X-MEDIA-SEQUENCE", "-1"),
        ] {
            let text = format!("#EXTM3U\n#{tag}:{value}\n#EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n");
            let Err(err) = parse_media_playlist(None, &text) else {
                unreachable!("{tag}={value:?} must refuse");
            };
            assert!(
                matches!(err, HlsPlaylistError::BadTagValue { tag: t } if t == tag),
                "{tag}={value:?} must refuse BadTagValue"
            );
        }
    }

    /// The model RECORDS the B1b+ shapes instead of refusing them:
    /// live (no ENDLIST), encryption, byte-ranges, init sections,
    /// discontinuities — the session owns those refusals.
    #[test]
    fn model_records_not_refuses_b1b_shapes() {
        let live = "#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXTINF:1.0,\ns.ts\n";
        let (model, _) = match parse_media_playlist(None, live) {
            Ok(pair) => pair,
            Err(e) => unreachable!("live shape records: {e}"),
        };
        assert!(!model.end_list);

        let encrypted = "#EXTM3U\n#EXT-X-TARGETDURATION:2\n\
            #EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV=0x0102030405060708090a0b0c0d0e0f10\n\
            #EXTINF:1.0,\ns.ts\n#EXT-X-ENDLIST\n";
        let (model, _) = match parse_media_playlist(None, encrypted) {
            Ok(pair) => pair,
            Err(e) => unreachable!("encrypted shape records: {e}"),
        };
        let Some(key) = model.segments[0].key.as_ref() else {
            unreachable!("key recorded");
        };
        assert_eq!(key.uri, "k");
        assert_eq!(
            key.iv.as_deref(),
            Some(&[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16][..])
        );

        let heavy = "#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXT-X-MAP:URI=\"init.mp4\"\n\
            #EXT-X-BYTERANGE:1000@0\n#EXTINF:1.0,\ns.m4s\n#EXT-X-DISCONTINUITY\n\
            #EXTINF:1.0,\nt.m4s\n#EXT-X-ENDLIST\n";
        let (model, _) = match parse_media_playlist(None, heavy) {
            Ok(pair) => pair,
            Err(e) => unreachable!("heavy shape records: {e}"),
        };
        let Some(map) = model.segments[0].map.as_ref() else {
            unreachable!("map recorded");
        };
        assert_eq!(map.uri, "init.mp4");
        assert_eq!(
            model.segments[0].byte_range,
            Some(ByteRange {
                length: 1000,
                offset: Some(0)
            })
        );
        assert!(!model.segments[0].discontinuity);
        assert!(model.segments[1].discontinuity);
    }

    /// Dirty input keeps the parser's tolerance: a malformed EXTINF
    /// warns and the entry survives with `duration: None`.
    #[test]
    fn dirty_world_warnings_flow_through() {
        let dirty = "#EXTM3U\n#EXTINF:junk,\ns.ts\n#EXT-X-ENDLIST\n";
        let (model, warnings) = match parse_media_playlist(None, dirty) {
            Ok(pair) => pair,
            Err(e) => unreachable!("dirty shape types: {e}"),
        };
        assert_eq!(model.segments.len(), 1);
        assert_eq!(model.segments[0].duration, None);
        assert_eq!(model.total_extinf_duration, 0.0);
        assert!(matches!(
            warnings.first(),
            Some(ParseWarning::BadDuration { .. })
        ));
    }

    /// The ffmpeg-generated fixtures from card 5.O1a (committed
    /// under tests/fixtures/hls/) type into the model cleanly.
    #[test]
    fn ffmpeg_fixture_ts_vod_types_fully() {
        let text = include_str!("../tests/fixtures/hls/ts_vod/pl.m3u8");
        let (model, _) =
            match parse_media_playlist(Some("http://cdn.example/hls/ts_vod/pl.m3u8"), text) {
                Ok(pair) => pair,
                Err(e) => unreachable!("ffmpeg fixture must type: {e}"),
            };
        assert_eq!(model.version, Some(3));
        assert_eq!(model.target_duration, Some(2));
        assert_eq!(model.media_sequence, 0);
        assert_eq!(model.playlist_type.as_deref(), Some("VOD"));
        assert!(model.end_list);
        assert_eq!(model.segments.len(), 5);
        assert_eq!(
            model.segments[0].uri,
            "http://cdn.example/hls/ts_vod/pl0.ts"
        );
        assert_eq!(
            model.segments[4].uri,
            "http://cdn.example/hls/ts_vod/pl4.ts"
        );
    }

    // ------------------------------------------------ 6.B2a master playlists

    const MULTI: &str = "#EXTM3U\n\
        #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=YES,AUTOSELECT=YES,URI=\"audio/en.m3u8\"\n\
        #EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"sub\",NAME=\"Swedish\",LANGUAGE=\"sv\",URI=\"sub/sv.m3u8\"\n\
        #EXT-X-STREAM-INF:BANDWIDTH=2000000,AVERAGE-BANDWIDTH=1950000,PROGRAM-ID=1,CODECS=\"avc1.640028,mp4a.40.2\",RESOLUTION=1280x720\n\
        hi.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=800000,CODECS=\"avc1.42e01e,mp4a.40.2\",RESOLUTION=640x360\n\
        mid.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=200000\n\
        lo.m3u8\n";

    /// The multi-variant fixture: variants and renditions type, URIs
    /// resolve against the base, playlist order is kept.
    #[test]
    fn master_types_variants_and_renditions() {
        let (master, warnings) =
            match parse_master_playlist(Some("http://cdn.example/v/master.m3u8"), MULTI) {
                Ok(pair) => pair,
                Err(e) => unreachable!("multi-variant fixture must type: {e}"),
            };
        assert!(warnings.is_empty());
        assert_eq!(master.variants.len(), 3);
        assert_eq!(master.variants[0].uri, "http://cdn.example/v/hi.m3u8");
        assert_eq!(master.variants[0].bandwidth, 2_000_000);
        assert_eq!(master.variants[0].average_bandwidth, Some(1_950_000));
        assert_eq!(master.variants[0].program_id, Some(1));
        assert_eq!(
            master.variants[0].codecs.as_deref(),
            Some("avc1.640028,mp4a.40.2")
        );
        assert_eq!(
            master.variants[0].resolution,
            Some(Resolution {
                width: 1280,
                height: 720
            })
        );
        assert_eq!(master.variants[2].resolution, None);
        assert_eq!(master.renditions.len(), 2);
        assert_eq!(master.renditions[0].kind, RenditionType::Audio);
        assert!(master.renditions[0].is_default && master.renditions[0].autoselect);
        assert_eq!(master.renditions[0].language.as_deref(), Some("en"));
        assert_eq!(
            master.renditions[0].uri.as_deref(),
            Some("http://cdn.example/v/audio/en.m3u8")
        );
        assert_eq!(master.renditions[1].kind, RenditionType::Subtitles);
        assert!(!master.renditions[1].is_default);
    }

    /// A single-variant master is a master (selection just has one
    /// rung); a CLOSED-CAPTIONS rendition with no URI records `None`.
    #[test]
    fn master_single_variant_and_uri_less_rendition() {
        let single = "#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"cc\",NAME=\"CEA-608\",INSTREAM-ID=\"CC1\"\n\
            #EXT-X-STREAM-INF:BANDWIDTH=550000,CODECS=\"avc1.4d401f\"\n\
            only.m3u8\n";
        let (master, _) = match parse_master_playlist(None, single) {
            Ok(pair) => pair,
            Err(e) => unreachable!("single-variant master must type: {e}"),
        };
        assert_eq!(master.variants.len(), 1);
        assert_eq!(master.variants[0].bandwidth, 550_000);
        assert_eq!(master.variants[0].uri, "only.m3u8");
        assert_eq!(master.renditions.len(), 1);
        assert_eq!(master.renditions[0].kind, RenditionType::ClosedCaptions);
        assert_eq!(master.renditions[0].uri, None);
    }

    /// Malformed rows skip with warnings, never fabricated values:
    /// missing BANDWIDTH, unparseable BANDWIDTH, damaged attribute
    /// lists (unterminated quote, trailing comma).
    #[test]
    fn master_malformed_rows_skip_with_warnings() {
        let malformed = "#EXTM3U\n\
            #EXT-X-STREAM-INF:BANDWIDTH=1000000\n\
            good.m3u8\n\
            #EXT-X-STREAM-INF:RESOLUTION=640x360\n\
            no-bw.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=big\n\
            bad-bw.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=900000,CODECS=\"unterminated\n\
            broken-quote.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=900000,\n\
            trailing-comma.m3u8\n";
        let (master, warnings) = match parse_master_playlist(None, malformed) {
            Ok(pair) => pair,
            Err(e) => unreachable!("malformed master types with warnings: {e}"),
        };
        assert_eq!(master.variants.len(), 1, "only the clean row survives");
        assert_eq!(master.variants[0].uri, "good.m3u8");
        assert_eq!(warnings.len(), 4);
        assert!(
            warnings
                .iter()
                .all(|w| w.kind == MasterWarningKind::VariantWithoutBandwidth)
        );
    }

    /// The mirror refusals: a media playlist where a master was
    /// expected, and no `#EXTM3U` at all.
    #[test]
    fn master_refuses_media_shape_and_non_hls() {
        let Err(err) = parse_master_playlist(None, VOD) else {
            unreachable!("media playlist must refuse");
        };
        assert!(matches!(err, HlsPlaylistError::MediaPlaylist));
        assert_eq!(err.code(), "playlists.hls.media-playlist");
        let Err(err) = parse_master_playlist(None, "hi.m3u8\n") else {
            unreachable!("no magic must refuse");
        };
        assert!(matches!(err, HlsPlaylistError::NotHls));
    }

    /// The committed master fixtures (card 6.B2a) type cleanly.
    #[test]
    fn master_fixture_files_type() {
        let multi = include_str!("../tests/fixtures/hls/master/multi.m3u8");
        let (master, warnings) =
            match parse_master_playlist(Some("http://cdn.example/master/multi.m3u8"), multi) {
                Ok(pair) => pair,
                Err(e) => unreachable!("fixture must type: {e}"),
            };
        assert!(warnings.is_empty());
        assert_eq!(master.variants.len(), 3);
        assert_eq!(master.renditions.len(), 2);

        let single = include_str!("../tests/fixtures/hls/master/single.m3u8");
        let (master, _) = match parse_master_playlist(None, single) {
            Ok(pair) => pair,
            Err(e) => unreachable!("fixture must type: {e}"),
        };
        assert_eq!(master.variants.len(), 1);

        let malformed = include_str!("../tests/fixtures/hls/master/malformed.m3u8");
        let (master, warnings) = match parse_master_playlist(None, malformed) {
            Ok(pair) => pair,
            Err(e) => unreachable!("fixture must type: {e}"),
        };
        assert_eq!(master.variants.len(), 1);
        assert!(!warnings.is_empty());
    }
}
