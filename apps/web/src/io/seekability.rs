//! Seekability detection (card 7.H4; doc 09 §6.1: "seekability is
//! *detected, not assumed*") — the mirror of `crates/streaming`'s
//! ranged-probe semantics (card 6.A2a) for the fetch-shaped browser
//! path.
//!
//! # The probe (mirrors `ranged.rs`)
//!
//! One ranged `GET` — `Range: bytes=0-(chunk-1)` — doubles as the
//! first read-ahead. Its answer fixes the mode:
//!
//! | Probe answer | Detection | Consequence |
//! |---|---|---|
//! | `206` + parseable `Content-Range` | [`Seekability::Ranged`] (total from the header; `None` for `bytes a-b/*`) | Seekable: the server honors `Range:` |
//! | `200` (full body) | [`Seekability::Unseekable`] (length from `Content-Length`, cross-checked) | The no-range server: forward-only, live-style rules (VLC plays these too — card 6.A2b's scan mode) |
//! | `416` | [`Seekability::Empty`] | Nothing exists at offset 0 (`size() == Some(0)`) |
//! | other 4xx/5xx | [`FetchError::HttpStatus`] | The typed transport refusal |
//! | `206` without a parseable/agreeing `Content-Range`, or a body shorter than the promised span | [`FetchError::ProtocolViolation`] / [`FetchError::Truncated`] | The same typed protocol-violation posture `ranged.rs` keeps |
//!
//! `Accept-Ranges` is **advisory data only** (recorded, never
//! decision-making): the ranged GET is the proof, exactly as in
//! `ranged.rs` (which does not consult the header either — a server
//! may advertise `Accept-Ranges: bytes` and still answer `200`).
//!
//! # Where the consequence lands
//!
//! The detection feeds [`crate::io::staged_source`]'s mode and —
//! through it — `media.get-info`'s `seekable` field and
//! `playback.seek`'s honest answer: unseekable sources report so,
//! and live-style forward-only playback rules apply (doc 09 §6.1).
//! Redirects are followed by the transport (browser `fetch()` or the
//! mock) BEFORE detection runs; `final_url`/`redirected` are
//! recorded observables, never re-resolved here.

use crate::io::fetch_port::{FetchError, FetchRequest, FetchResponse};

/// The detected seekability of one http(s) source (doc 09 §6.1's
/// probe outcome, as data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seekability {
    /// The server honors `Range:` (probe answered `206` + a parseable
    /// `Content-Range`). `total` is the resource length; `None` for
    /// unknown-length streams (`bytes a-b/*`) — a value and its
    /// absence are different types (MP §8).
    Ranged {
        /// The `Content-Range` total, when revealed.
        total: Option<u64>,
    },
    /// The no-range server (probe answered `200`): playable but
    /// forward-only — seeks re-fetch from byte 0, `media.get-info`
    /// reports `seekable: false`, live-style rules apply.
    Unseekable {
        /// The resource length (`Content-Length`, cross-checked
        /// against the delivered body when both exist).
        length: Option<u64>,
    },
    /// The probe's `416` at offset 0: nothing exists at offset 0 —
    /// the empty-resource shape (`size() == Some(0)`), the same
    /// honoring `ranged.rs` applies.
    Empty,
}

impl Seekability {
    /// Whether `media.get-info`'s `seekable` field is true for this
    /// detection (the server truth — the host's seek *serving* is a
    /// separate, honestly-reported duty).
    #[must_use]
    pub fn is_seekable(&self) -> bool {
        matches!(self, Seekability::Ranged { .. })
    }

    /// The detected resource length, when known (Ranged total or
    /// Unseekable `Content-Length`).
    #[must_use]
    pub fn length(&self) -> Option<u64> {
        match *self {
            Seekability::Ranged { total } | Seekability::Unseekable { length: total } => total,
            Seekability::Empty => Some(0),
        }
    }
}

/// Run the detection over one probe response (the probe request the
/// caller issued — see [`crate::io::staged_source`], which mints it).
///
/// # Errors
/// [`FetchError`] — `HttpStatus` for uninterpreted statuses,
/// `ProtocolViolation` for a `206` whose `Content-Range` is missing,
/// unparseable, or disagrees with the requested start, for a lying
/// `Content-Length` on a `200`, or for a surfaced 3xx (the transport
/// follows redirects; one surfacing here is a transport breach);
/// `Truncated` for a body shorter than the `Content-Range` span
/// that does not end at the resource end.
pub fn detect_seekability(
    request: &FetchRequest,
    response: &FetchResponse,
) -> Result<Seekability, FetchError> {
    match response.status {
        206 => {
            let Some(range) = request.range else {
                return Err(FetchError::ProtocolViolation {
                    detail: "a 206 arrived for a rangeless request".to_string(),
                });
            };
            let header =
                response
                    .header("Content-Range")
                    .ok_or_else(|| FetchError::ProtocolViolation {
                        detail: "206 without a Content-Range header".to_string(),
                    })?;
            let (start, end, total) = parse_content_range(header)?;
            if start != range.start {
                return Err(FetchError::ProtocolViolation {
                    detail: format!(
                        "Content-Range start {start} disagrees with the requested {}",
                        range.start
                    ),
                });
            }
            let span = end.saturating_sub(start).saturating_add(1);
            if u64::try_from(response.body.len()).unwrap_or(u64::MAX) > span {
                return Err(FetchError::ProtocolViolation {
                    detail: "206 body longer than its Content-Range span".to_string(),
                });
            }
            // A body shorter than the promised span is only honest
            // when it ends at the resource end (the wire-end shape);
            // anything else is a truncation, never a silent EOF
            // (the file-source contract, mirrored from ranged.rs).
            let got = response.body.len() as u64;
            if got < span {
                let at_end = total.is_some_and(|total| end.saturating_add(1) == total);
                if !at_end {
                    return Err(FetchError::Truncated {
                        expected: span,
                        got,
                    });
                }
            }
            Ok(Seekability::Ranged { total })
        }
        200 => {
            // The no-range server. Length from Content-Length,
            // cross-checked against the delivered body: a
            // disagreement is a protocol violation (ranged.rs's
            // posture, verbatim).
            let declared = response
                .header("Content-Length")
                .and_then(|value| value.trim().parse::<u64>().ok());
            let delivered = response.body.len() as u64;
            if let Some(declared) = declared
                && declared != delivered
            {
                return Err(FetchError::ProtocolViolation {
                    detail: format!(
                        "Content-Length {declared} disagrees with the delivered {delivered} bytes"
                    ),
                });
            }
            Ok(Seekability::Unseekable {
                length: declared.or(Some(delivered)),
            })
        }
        416 => Ok(Seekability::Empty),
        300..=399 => Err(FetchError::ProtocolViolation {
            detail: format!(
                "a {} surfaced un-followed — the transport owns redirect following",
                response.status
            ),
        }),
        status => Err(FetchError::HttpStatus { status }),
    }
}

/// Parse `Content-Range: bytes <start>-<end>/<total|*>` (the mirror
/// of `ranged.rs`'s parser, restated because streaming's is private
/// — same grammar, same typed refusals).
///
/// # Errors
/// [`FetchError::ProtocolViolation`] for any malformed shape.
fn parse_content_range(header: &str) -> Result<(u64, u64, Option<u64>), FetchError> {
    let violation = |detail: String| FetchError::ProtocolViolation {
        detail: format!("malformed Content-Range `{header}`: {detail}"),
    };
    let value = header
        .trim()
        .strip_prefix("bytes")
        .ok_or_else(|| violation("missing the `bytes` unit token".to_string()))?
        .trim();
    let (span, total) = value
        .split_once('/')
        .ok_or_else(|| violation("missing the total".to_string()))?;
    let (start, end) = span
        .split_once('-')
        .ok_or_else(|| violation("missing the end".to_string()))?;
    let start: u64 = start
        .trim()
        .parse()
        .map_err(|_| violation("bad start".to_string()))?;
    let end: u64 = end
        .trim()
        .parse()
        .map_err(|_| violation("bad end".to_string()))?;
    let total = match total.trim() {
        "*" => None,
        digits => Some(
            digits
                .parse::<u64>()
                .map_err(|_| violation("bad total".to_string()))?,
        ),
    };
    if end < start {
        return Err(violation("end before start".to_string()));
    }
    Ok((start, end, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::fetch_port::RangeRequest;
    use testkit::{Must, assert_err};

    /// The probe request every test issues (the staged source's own
    /// probe shape: `Range: bytes=0-(chunk-1)`).
    fn probe() -> FetchRequest {
        FetchRequest::new(1, "https://media.example/a.avi").with_range(RangeRequest {
            start: 0,
            end_exclusive: 256 * 1024,
        })
    }

    fn response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> FetchResponse {
        FetchResponse {
            id: 1,
            status,
            headers: headers
                .iter()
                .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
                .collect(),
            body: body.to_vec(),
            final_url: "https://media.example/a.avi".to_string(),
            redirected: false,
        }
    }

    /// THE MATRIX, row 1 — accept-ranges server: `206` + agreeing
    /// `Content-Range` ⇒ `Ranged { total }`, seekable, length known.
    #[test]
    fn accept_ranges_206_yields_ranged_seekable() {
        let body = vec![0xAB; 1024];
        let seekability = detect_seekability(
            &probe(),
            &response(206, &[("Content-Range", "bytes 0-1023/5000")], &body),
        )
        .must();
        assert_eq!(seekability, Seekability::Ranged { total: Some(5000) });
        assert!(seekability.is_seekable());
        assert_eq!(seekability.length(), Some(5000));
    }

    /// THE MATRIX, row 1b — unknown-length servers (`bytes a-b/*`):
    /// seekable, `length()` absent (a value and its absence are
    /// different types).
    #[test]
    fn unknown_length_total_yields_ranged_without_total() {
        let seekability = detect_seekability(
            &probe(),
            &response(206, &[("Content-Range", "bytes 0-1023/*")], &[0xAB; 1024]),
        )
        .must();
        assert_eq!(seekability, Seekability::Ranged { total: None });
        assert!(seekability.is_seekable());
        assert_eq!(seekability.length(), None);
    }

    /// THE MATRIX, row 2 — no-range server: `200` full body ⇒
    /// `Unseekable { length }`, forward-only rules, seekable false.
    #[test]
    fn no_range_200_yields_unseekable_with_length() {
        let body = vec![0xCD; 4096];
        let seekability = detect_seekability(
            &probe(),
            &response(200, &[("Content-Length", "4096")], &body),
        )
        .must();
        assert_eq!(seekability, Seekability::Unseekable { length: Some(4096) });
        assert!(!seekability.is_seekable());
        assert_eq!(seekability.length(), Some(4096));
    }

    /// THE MATRIX, row 3 — partial content: a `206` body shorter
    /// than its promised span that does NOT end at the resource end
    /// is the typed truncation (never a silent EOF).
    #[test]
    fn truncated_206_body_is_the_typed_truncation() {
        let err = assert_err(
            detect_seekability(
                &probe(),
                &response(206, &[("Content-Range", "bytes 0-1023/5000")], &[0xAB; 512]),
            ),
            "truncated mid-resource",
        );
        assert_eq!(
            err,
            FetchError::Truncated {
                expected: 1024,
                got: 512
            },
            "{err}"
        );
        // …but a short body ending exactly at the resource end is the
        // honest wire-end shape (the last chunk of a 1500-byte
        // resource served under a 256 KiB range request).
        detect_seekability(
            &probe(),
            &response(
                206,
                &[("Content-Range", "bytes 0-1499/1500")],
                &[0xAB; 1500],
            ),
        )
        .must();
    }

    /// THE MATRIX, row 4 — redirects: followed by the TRANSPORT
    /// before detection (final_url/redirected are observables); a
    /// 3xx surfacing to the detector is the typed transport breach.
    #[test]
    fn surfaced_redirect_is_the_typed_transport_breach() {
        let err = assert_err(
            detect_seekability(&probe(), &response(302, &[("Location", "/final.avi")], &[])),
            "un-followed redirect",
        );
        assert!(matches!(err, FetchError::ProtocolViolation { .. }), "{err}");
        // The followed case is the mock's job (mock_fetch tests pin
        // the chain following + the observables).
    }

    /// THE MATRIX, row 5 — `416` at offset 0 is honored as the
    /// empty-resource shape (ranged.rs's own honoring).
    #[test]
    fn range_not_satisfiable_is_the_empty_resource() {
        let seekability = detect_seekability(
            &probe(),
            &response(416, &[("Content-Range", "bytes */0")], &[]),
        )
        .must();
        assert_eq!(seekability, Seekability::Empty);
        assert_eq!(seekability.length(), Some(0));
        assert!(!seekability.is_seekable());
    }

    /// THE MATRIX, row 6 — `206` without a parseable `Content-Range`
    /// (the garbage shape): the typed protocol violation, never a
    /// guess.
    #[test]
    fn garbage_206_is_the_protocol_violation() {
        for headers in [
            &[("X-None", "1")] as &[(&str, &str)],
            &[("Content-Range", "bytes 0-99")],
            &[("Content-Range", "bytes zero-99/100")],
            &[("Content-Range", "bytes 99-0/100")],
            &[("Content-Range", "octets 0-99/100")],
        ] {
            let err = assert_err(
                detect_seekability(&probe(), &response(206, headers, &[0; 100])),
                &format!("{headers:?}"),
            );
            assert!(
                matches!(err, FetchError::ProtocolViolation { .. }),
                "{headers:?}: {err}"
            );
        }
    }

    /// THE MATRIX, row 7 — a `200` whose `Content-Length` lies about
    /// the delivered body is the protocol violation (cross-checked,
    /// ranged.rs's posture verbatim).
    #[test]
    fn lying_content_length_is_the_protocol_violation() {
        let err = assert_err(
            detect_seekability(
                &probe(),
                &response(200, &[("Content-Length", "9999")], &[0; 128]),
            ),
            "lying Content-Length",
        );
        assert!(matches!(err, FetchError::ProtocolViolation { .. }), "{err}");
        // A missing Content-Length on a 200 is fine: the delivered
        // body IS the length.
        let seekability = detect_seekability(&probe(), &response(200, &[], &[0; 128])).must();
        assert_eq!(seekability.length(), Some(128));
    }

    /// Uninterpreted statuses are the typed HTTP refusal (branchable
    /// like the native `NetworkError::HttpStatus`).
    #[test]
    fn uninterpreted_statuses_are_typed() {
        for status in [404u16, 500, 503] {
            let err = assert_err(
                detect_seekability(&probe(), &response(status, &[], &[])),
                &format!("status {status}"),
            );
            assert_eq!(err, FetchError::HttpStatus { status }, "{err}");
        }
    }

    /// A `206` whose start disagrees with the request, or whose body
    /// exceeds its span, refuses typed (the agree-check ranged.rs
    /// keeps).
    #[test]
    fn disagreeing_or_oversized_206_refuses_typed() {
        let err = assert_err(
            detect_seekability(
                &probe(),
                &response(206, &[("Content-Range", "bytes 1-1024/5000")], &[0; 1024]),
            ),
            "disagreeing start",
        );
        assert!(matches!(err, FetchError::ProtocolViolation { .. }), "{err}");
        let err = assert_err(
            detect_seekability(
                &probe(),
                &response(206, &[("Content-Range", "bytes 0-511/5000")], &[0; 1024]),
            ),
            "oversized body",
        );
        assert!(matches!(err, FetchError::ProtocolViolation { .. }), "{err}");
    }

    /// A `206` for a rangeless request is the protocol violation
    /// (the transport answered a question nobody asked).
    #[test]
    fn rangeless_request_with_206_refuses_typed() {
        let request = FetchRequest::new(1, "https://media.example/a.avi");
        let err = assert_err(
            detect_seekability(
                &request,
                &response(206, &[("Content-Range", "bytes 0-0/1")], &[0]),
            ),
            "rangeless 206",
        );
        assert!(matches!(err, FetchError::ProtocolViolation { .. }), "{err}");
    }
}
