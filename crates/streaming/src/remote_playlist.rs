//! Remote playlist open (card 6.E2) — M3U/XSPF over the streaming
//! layer, with relative-URI resolution against the playlist's **final**
//! URL and bounded redirect following.
//!
//! The `playlists` crate stays I/O-free by design (its module docs);
//! this module is the network side of the seam: it fetches the bytes
//! through [`HttpFetch`](crate::fetch::HttpFetch), sniffs the format
//! (XSPF is XML — a leading `<` after BOM/whitespace; everything else
//! is M3U, the loose format), parses with the existing parsers, and
//! resolves every entry location with [`playlists::resolve_uri`] (the
//! RFC 3986 §5 join: `../`, `./`, root-relative, absolute, Windows
//! passthrough).
//!
//! # Redirects
//!
//! Followed **manually** (bounded by [`MAX_PLAYLIST_REDIRECTS`], loop-
//! and revisit-checked) so the base for relative resolution is the URL
//! the body actually came from — a playlist served after
//! `/old/list.m3u → /new/list.m3u` resolves its entries against
//! `/new/`, matching browser/VLC semantics.
//!
//! # Scope notes
//!
//! * An HTML directory listing masquerading as a playlist is a
//!   `[PROPOSAL]` decision recorded in the wave card: v0 does **not**
//!   parse HTML (an `<`-leading body is treated as XSPF and fails
//!   typed if it is not one) — surfacing a directory listing as a
//!   playlist is deferred with the card's decision.
//! * PLS/MPD remote open are not in 6.E2's scope (M3U + XSPF only).
//! * URL userinfo is accepted and **stripped** (credential-free URLs
//!   from here on); answering auth challenges is 6.E1's
//!   [`AuthFetch`](crate::auth::AuthFetch) — compose it under this
//!   opener's `fetch` argument.

use crate::error::NetworkError;
use crate::fetch::{HttpFetch, HttpRequest, RedirectMode, Url};

/// The redirect budget for one remote-playlist open (card 6.E2:
/// bounded, deliberately tighter than the transport's generic 10).
pub const MAX_PLAYLIST_REDIRECTS: u32 = 5;

/// The format sniffed off a fetched playlist body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemotePlaylistFormat {
    /// Extended/plain M3U (the loose format — the default).
    M3U,
    /// XSPF (XML: a `<`-leading body after BOM/whitespace).
    Xspf,
}

/// One resolved remote-playlist entry.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteEntry {
    /// The entry's location, resolved against the playlist's final
    /// URL (absolute references pass through verbatim; Windows paths
    /// pass through untouched, never mangled into fake URLs).
    pub location: String,
    /// `#EXTINF` title / XSPF `<title>`, when present.
    pub title: Option<String>,
    /// `#EXTINF` duration in seconds, when present and finite.
    pub duration: Option<f64>,
}

/// A playlist opened from the network (card 6.E2).
#[derive(Debug, Clone, PartialEq)]
pub struct RemotePlaylist {
    /// The **final** URL the body came from (after redirects) — the
    /// base every entry location was resolved against.
    pub source_url: String,
    /// The sniffed format.
    pub format: RemotePlaylistFormat,
    /// Entries in source order, locations resolved.
    pub entries: Vec<RemoteEntry>,
    /// How many parse warnings the playlists crate reported (dirty
    /// lines skipped, dangling metadata, … — details live in that
    /// crate's warning vocabulary; counted, not re-surfaced).
    pub parse_warnings: usize,
}

impl RemotePlaylist {
    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the playlist has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Open a remote M3U or XSPF playlist over the streaming layer.
///
/// Fetches `url` through `fetch` (bounded body, bounded redirects,
/// typed errors), sniffs the format, parses, and resolves entry
/// locations against the final URL. Compose an
/// [`AuthFetch`](crate::auth::AuthFetch) as the `fetch` argument for
/// authenticated sources.
///
/// # Errors
/// [`NetworkError`] for every failure: transport weather, the
/// redirect budget/loop, non-success statuses
/// ([`NetworkError::HttpStatus`]), and unparsable bodies
/// ([`NetworkError::ProtocolViolation`] — the playlists crate's
/// typed parse errors fold into the transport vocabulary's static
/// detail; no body bytes or URLs ride the error, MP §12).
pub fn open_remote_playlist(
    fetch: &dyn HttpFetch,
    url: &str,
) -> Result<RemotePlaylist, NetworkError> {
    // Userinfo is accepted and stripped: the fetch URL (and the
    // resolution base) never carries a credential.
    let (mut url, _userinfo) = Url::parse_with_userinfo(url)?;

    // Manual redirect following, bounded: the final URL is the
    // resolution base, so the chain is walked here, not in the
    // transport.
    let mut followed: u32 = 0;
    let mut visited: Vec<String> = vec![url.to_string()];
    let response = loop {
        let request = HttpRequest::get(url.to_string())
            .with_redirect_mode(RedirectMode::Manual)
            .with_header("Accept", "audio/x-mpegurl, application/xspf+xml, */*");
        let response = fetch.fetch(request)?;
        if matches!(response.status, 301 | 302 | 303 | 307 | 308)
            && let Some(location) = response.header("location")
        {
            if followed >= MAX_PLAYLIST_REDIRECTS {
                return Err(NetworkError::TooManyRedirects {
                    limit: MAX_PLAYLIST_REDIRECTS,
                    followed,
                });
            }
            let next = url.join(location)?;
            let canonical = next.to_string();
            if visited.contains(&canonical) {
                return Err(NetworkError::RedirectLoop {
                    hops: visited.len() as u32,
                });
            }
            visited.push(canonical);
            url = next;
            followed += 1;
            continue;
        }
        break response;
    };

    if !response.is_success() {
        return Err(NetworkError::http_status(response.status));
    }

    let format = sniff_format(&response.body);
    let base = url.to_string();
    const PARSE_REFUSED: NetworkError = NetworkError::ProtocolViolation {
        detail: "remote playlist body failed to parse (not valid UTF-8 or unparseable)",
    };
    let (entries, parse_warnings) = match format {
        RemotePlaylistFormat::M3U => {
            let parsed = playlists::parse_bytes(&response.body).map_err(|_| PARSE_REFUSED)?;
            (
                parsed
                    .playlist
                    .entries
                    .into_iter()
                    .map(|entry| RemoteEntry {
                        location: playlists::resolve_uri(&base, &entry.location),
                        title: entry.title,
                        duration: entry.duration,
                    })
                    .collect::<Vec<_>>(),
                parsed.warnings.len(),
            )
        }
        RemotePlaylistFormat::Xspf => {
            let parsed = playlists::parse_xspf_bytes(&response.body).map_err(|_| PARSE_REFUSED)?;
            (
                parsed
                    .playlist
                    .entries
                    .into_iter()
                    .map(|entry| RemoteEntry {
                        location: playlists::resolve_uri(&base, &entry.location),
                        title: entry.title,
                        duration: entry.duration,
                    })
                    .collect::<Vec<_>>(),
                parsed.warnings.len(),
            )
        }
    };

    Ok(RemotePlaylist {
        source_url: base,
        format,
        entries,
        parse_warnings,
    })
}

/// Sniff the format: XSPF is XML (a `<` after BOM/whitespace);
/// everything else is M3U (the loose format). Directory-listing HTML
/// also starts with `<` — it is treated as XSPF and fails typed when
/// it is not one (the HTML-parse decision is `[PROPOSAL]`, deferred
/// with the card).
fn sniff_format(body: &[u8]) -> RemotePlaylistFormat {
    let mut bytes = body;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = &bytes[3..];
    }
    let starts_with_xml = bytes
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| *byte == b'<');
    if starts_with_xml {
        RemotePlaylistFormat::Xspf
    } else {
        RemotePlaylistFormat::M3U
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mockhttp::{MockConfig, MockServer};
    use testkit::Must;

    /// M3U over the wire: relative, `../`, root-relative and absolute
    /// locations all resolve against the playlist URL; UTF-8 titles
    /// and durations survive the round trip.
    #[test]
    fn m3u_resolves_locations_against_the_playlist_url() {
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let body = "#EXTM3U\n\
                    #EXTINF:10.5,Café Über\n\
                    track1.mp3\n\
                    #EXTINF:-1,Live\n\
                    ../shared/track2.flac\n\
                    /sub/track3.mp3\n\
                    http://other.example/x.mp3\n";
        let server = MockServer::spawn(MockConfig {
            routes: vec![("/pl/list.m3u".to_string(), body.as_bytes().to_vec())],
            ..MockConfig::default()
        })
        .must();

        let playlist = open_remote_playlist(&server.client(), &server.url("/pl/list.m3u")).must();
        assert_eq!(playlist.format, RemotePlaylistFormat::M3U);
        assert_eq!(playlist.source_url, server.url("/pl/list.m3u"));
        assert_eq!(playlist.parse_warnings, 0);
        assert_eq!(playlist.len(), 4);

        let locations: Vec<&str> = playlist
            .entries
            .iter()
            .map(|entry| entry.location.as_str())
            .collect();
        assert_eq!(
            locations,
            vec![
                server.url("/pl/track1.mp3"),
                server.url("/shared/track2.flac"),
                server.url("/sub/track3.mp3"),
                "http://other.example/x.mp3".to_string(), // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            ]
        );
        assert_eq!(playlist.entries[0].title.as_deref(), Some("Café Über"));
        assert_eq!(playlist.entries[0].duration, Some(10.5));
        // The `-1` unknown-duration marker is preserved as written.
        assert_eq!(playlist.entries[1].duration, Some(-1.0));
    }

    /// XSPF over the wire: XML sniffed by the leading `<`, locations
    /// resolved, titles and durations carried.
    #[test]
    fn xspf_sniffed_and_resolved() {
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let body = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                    <playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n\
                    <trackList>\n\
                    <track><location>track1.mp3</location><title>Café</title>\
                    <duration>10500</duration></track>\n\
                    <track><location>../up.flac</location></track>\n\
                    <track><location>http://other.example/x.mp3</location></track>\n\
                    </trackList>\n\
                    </playlist>\n";
        let server = MockServer::spawn(MockConfig {
            routes: vec![("/pl/list.xspf".to_string(), body.as_bytes().to_vec())],
            ..MockConfig::default()
        })
        .must();

        let playlist = open_remote_playlist(&server.client(), &server.url("/pl/list.xspf")).must();
        assert_eq!(playlist.format, RemotePlaylistFormat::Xspf);
        assert_eq!(playlist.len(), 3);
        let locations: Vec<&str> = playlist
            .entries
            .iter()
            .map(|entry| entry.location.as_str())
            .collect();
        assert_eq!(
            locations,
            vec![
                server.url("/pl/track1.mp3"),
                server.url("/up.flac"),
                "http://other.example/x.mp3".to_string(), // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            ]
        );
        assert_eq!(playlist.entries[0].title.as_deref(), Some("Café"));
    }

    /// Redirects are followed (bounded) and the **final** URL is the
    /// resolution base: `/pl.m3u → /r1 → /r2` serves the body, and a
    /// relative entry resolves against `/r2`'s directory.
    #[test]
    fn redirects_followed_with_final_url_base() {
        let body = "#EXTM3U\nseg.mp3\n";
        let server = MockServer::spawn(MockConfig {
            redirects: 2,
            routes: vec![("/r2".to_string(), body.as_bytes().to_vec())],
            ..MockConfig::default()
        })
        .must();

        let playlist = open_remote_playlist(&server.client(), &server.url("/pl.m3u")).must();
        assert_eq!(playlist.source_url, server.url("/r2"));
        assert_eq!(playlist.entries[0].location, server.url("/seg.mp3"));
        // The chain walked exactly three requests (entry, r1, r2).
        let logged = server.requests();
        let paths: Vec<&str> = logged.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["/pl.m3u", "/r1", "/r2"]);
    }

    /// The redirect budget is the card's bound (5), and exceeding it
    /// is the typed too-many-redirects failure.
    #[test]
    fn redirect_budget_is_bounded_typed() {
        let server = MockServer::spawn(MockConfig {
            redirects: 8,
            routes: vec![("/r8".to_string(), b"#EXTM3U\n".to_vec())],
            ..MockConfig::default()
        })
        .must();

        let error = open_remote_playlist(&server.client(), &server.url("/pl.m3u"))
            .err()
            .must();
        assert!(
            matches!(
                error,
                NetworkError::TooManyRedirects {
                    limit: 5,
                    followed: 5
                }
            ),
            "the 6.E2 budget is five, got {error}"
        );
    }

    /// A self-referential redirect is the typed loop failure, never
    /// an infinite walk.
    #[test]
    fn redirect_loop_is_typed() {
        let server = MockServer::spawn(MockConfig {
            self_loop: true,
            ..MockConfig::default()
        })
        .must();
        let error = open_remote_playlist(&server.client(), &server.url("/pl.m3u"))
            .err()
            .must();
        assert!(matches!(error, NetworkError::RedirectLoop { .. }));
    }

    /// A missing playlist is the typed HTTP status; a body that is
    /// not UTF-8 (or not a playlist) is the typed protocol violation.
    #[test]
    fn failures_are_typed() {
        let missing = MockServer::spawn(MockConfig {
            routes: vec![("/pl.m3u".to_string(), b"irrelevant".to_vec())],
            ..MockConfig::default()
        })
        .must();
        let error = open_remote_playlist(&missing.client(), &missing.url("/nope.m3u"))
            .err()
            .must();
        assert_eq!(error.status_code(), Some(404));

        let garbage = MockServer::spawn(MockConfig {
            routes: vec![("/pl.m3u".to_string(), vec![0xFF, 0xFE, 0x00, 0x01, 0x02])],
            ..MockConfig::default()
        })
        .must();
        let error = open_remote_playlist(&garbage.client(), &garbage.url("/pl.m3u"))
            .err()
            .must();
        assert!(
            matches!(error, NetworkError::ProtocolViolation { .. }),
            "non-UTF-8 body must be a typed violation, got {error}"
        );
    }
}
