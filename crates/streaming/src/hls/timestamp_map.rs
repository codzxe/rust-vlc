//! The HLS `#EXT-X-TIMESTAMP-MAP` handling (card 6.B3): parse the
//! tag from a media playlist's raw text and apply the mapping to the
//! live session's wallclock surfaces.
//!
//! # The tag (public spec: RFC 8216 + the IETF HLS bis draft's tag
//! grammar — clean-room, no proprietary suites)
//!
//! ```text
//! #EXT-X-TIMESTAMP-MAP:MPEGTS=<decimal-integer>,LOCAL=<ISO-8601 datetime>
//! ```
//!
//! The tag establishes a **linear mapping from MPEG-2 Transport
//! Stream timestamps (the 90 kHz PTS/PCR clock) to wall-clock time**:
//! the `MPEGTS` value and the `LOCAL` value designate the same
//! instant. A mapping applies to the timestamps of the segments that
//! follow it in the playlist; a later `#EXT-X-TIMESTAMP-MAP`
//! supersedes an earlier one (an encoder restart republishes the
//! pair). Its canonical consumer is WebVTT-in-TS alignment (the
//! subtitle lane's record in `docs/formats/sub-vtt.md` carried
//! "X-TIMESTAMP-MAP unapplied" until this card); here the live
//! session also uses it for **live-edge wallclock derivation**.
//!
//! # The arithmetic (33-bit PTS wraparound)
//!
//! A PTS is a 33-bit counter: it wraps every `2^33 / 90 000` s ≈
//! 26.5 h. A naive subtraction is wrong across a wrap, so every
//! conversion here works in *signed* wrap distance: the difference
//! between two PTS values is the representative of
//! `(b - a) mod 2^33` closest to zero. That makes
//! [`TimestampMap::wallclock_ms_for_pts`] exact for any PTS within
//! ±13.25 h of the anchor (far beyond any legal playlist window),
//! and [`TimestampMap::pts_for_wallclock_ms`] its inverse.
//!
//! # What the live session derives from it
//!
//! [`crate::hls::live::HlsLiveStream::live_edge_wallclock_ms`]: the
//! wall-clock time of the **live edge** (the end of the newest
//! segment in the current window), computed as `LOCAL` advanced by
//! the window's total `EXTINF` duration. This rests on the anchoring
//! assumption that the tag's `MPEGTS`/`LOCAL` pair sits at the media
//! position of the **first segment that follows it** — how encoders
//! publish it in practice (the tag rides at the top of the window);
//! the spec does not guarantee the anchor, so the record marks the
//! derivation `[VERIFY]` against a real-world corpus.

use crate::hls::media::TAG_TIMESTAMP_MAP;

/// The PTS modulus: 2^33 ticks at 90 kHz (≈ 26.5 h before wrap).
pub const PTS_MODULUS: u64 = 1 << 33;
/// The PTS clock rate in Hz (MPEG-2 Systems, 90 kHz).
pub const PTS_HZ: u64 = 90_000;

/// One parsed `#EXT-X-TIMESTAMP-MAP`: the PTS↔wallclock anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampMap {
    /// The `MPEGTS` anchor: a PTS tick count, always `< 2^33`
    /// (values outside the 33-bit counter are refused by the
    /// parser, never clamped).
    pub mpegts: u64,
    /// The `LOCAL` anchor: wall-clock time as Unix-epoch
    /// milliseconds (the tag's ISO-8601 spelling resolved to UTC).
    pub local_ms: i64,
}

impl TimestampMap {
    /// Parse the anchor pair out of one `#EXT-X-TIMESTAMP-MAP:` tag
    /// value (the text after the colon). Attributes may appear in
    /// either order; both `MPEGTS` and `LOCAL` are required. A
    /// malformed or out-of-range value is `None` (the session then
    /// simply has no mapping — a tag we cannot understand must not
    /// refuse playback).
    #[must_use]
    pub fn parse_tag_value(value: &str) -> Option<TimestampMap> {
        let mut mpegts: Option<u64> = None;
        let mut local: Option<i64> = None;
        for attr in value.split(',') {
            let (name, val) = attr.split_once('=')?;
            match name.trim() {
                "MPEGTS" => {
                    let ticks: u64 = val.trim().parse().ok()?;
                    if ticks >= PTS_MODULUS {
                        return None; // outside the 33-bit PTS counter
                    }
                    mpegts = Some(ticks);
                }
                "LOCAL" => {
                    local = Some(parse_iso8601_utc(val.trim())?);
                }
                _ => {} // unknown attribute: tolerated, ignored
            }
        }
        Some(TimestampMap {
            mpegts: mpegts?,
            local_ms: local?,
        })
    }

    /// The **last** `#EXT-X-TIMESTAMP-MAP` in a media playlist's
    /// text (the mapping that governs the segments at the end of the
    /// window — the newest ones, i.e. the live edge). A playlist
    /// without a parseable tag maps to `None`.
    #[must_use]
    pub fn from_playlist_text(text: &str) -> Option<TimestampMap> {
        text.lines()
            .filter_map(|line| line.strip_prefix(TAG_TIMESTAMP_MAP))
            .filter_map(TimestampMap::parse_tag_value)
            .next_back()
    }

    /// Map a PTS (90 kHz ticks) to wall-clock Unix-epoch
    /// milliseconds, exact across 33-bit wraparound: the difference
    /// is taken as the signed representative closest to zero, so any
    /// PTS within ±13.25 h of the anchor is correct.
    #[must_use]
    pub fn wallclock_ms_for_pts(&self, pts: u64) -> i64 {
        self.local_ms + signed_wrap_delta(self.mpegts, pts) * 1000 / PTS_HZ as i64
    }

    /// The inverse of [`TimestampMap::wallclock_ms_for_pts`]: map a
    /// wall-clock Unix-epoch millisecond value to the PTS tick it
    /// corresponds to, wrapping into `0..2^33`.
    #[must_use]
    pub fn pts_for_wallclock_ms(&self, wallclock_ms: i64) -> u64 {
        let delta_ticks = (wallclock_ms - self.local_ms) * PTS_HZ as i64 / 1000;
        self.mpegts
            .wrapping_add(delta_ticks as u64)
            % PTS_MODULUS
    }
}

/// The signed wrap-distance `b - a` on the 33-bit PTS circle:
/// `(b - a) mod 2^33` rebased into `[-2^32, 2^32)`.
fn signed_wrap_delta(a: u64, b: u64) -> i64 {
    let raw = b.wrapping_sub(a) % PTS_MODULUS;
    if raw >= PTS_MODULUS / 2 {
        raw as i64 - PTS_MODULUS as i64
    } else {
        raw as i64
    }
}

/// Parse the HLS ISO-8601 profile (`YYYY-MM-DDTHH:MM:SS[.f{1,9}]Z`,
/// UTC `Z` spelling only — the form every public HLS example uses)
/// into Unix-epoch milliseconds. Out-of-range fields (month 13,
/// Feb 30, hour 24, …) are refused, not clamped.
fn parse_iso8601_utc(text: &str) -> Option<i64> {
    let text = text.trim().strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() {
        return None;
    }
    let (whole, frac_ms) = match time.split_once('.') {
        Some((whole, frac)) => {
            if frac.is_empty() || frac.len() > 9 || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // Millisecond precision: the first three fractional
            // digits weigh 100/10/1 ms; anything finer is truncated.
            let mut ms: i64 = 0;
            for (i, b) in frac.bytes().take(3).enumerate() {
                let digit = i64::from(b - b'0');
                ms += digit * 10_i64.pow(2 - i as u32);
            }
            (whole, ms)
        }
        None => (time, 0),
    };
    let mut time_parts = whole.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;
    if time_parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(
        ((days * 24 + hour) * 60 + minute) * 60 * 1000 + second * 1000 + frac_ms,
    )
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days from 1970-01-01 for a civil date (Howard Hinnant's
/// `days_from_civil`, public-domain era arithmetic — no date crate).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (i64::from(m) + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The anchor pair from the tag grammar, both attribute orders.
    #[test]
    fn parses_the_tag_in_either_attribute_order() {
        let a = TimestampMap::parse_tag_value("MPEGTS=900000,LOCAL=1970-01-01T00:00:00Z")
            .expect("canonical order parses");
        let b = TimestampMap::parse_tag_value("LOCAL=1970-01-01T00:00:00Z,MPEGTS=900000")
            .expect("swapped order parses");
        assert_eq!(a, b);
        assert_eq!(a.mpegts, 900_000);
        assert_eq!(a.local_ms, 0);
    }

    /// The BOM mapping: anchor at the Unix epoch, `MPEGTS=0` —
    /// every PTS converts to pure wallclock offset (`90 000 ticks =
    /// 1 s`), and fractional-second LOCAL values carry through.
    #[test]
    fn bom_mapping_converts_pts_to_epoch_milliseconds() {
        let bom = TimestampMap::parse_tag_value("MPEGTS=0,LOCAL=1970-01-01T00:00:00Z")
            .expect("BOM mapping parses");
        assert_eq!(bom.wallclock_ms_for_pts(0), 0);
        assert_eq!(bom.wallclock_ms_for_pts(90_000), 1_000);
        assert_eq!(bom.wallclock_ms_for_pts(45_000), 500);
        // The inverse round-trips.
        assert_eq!(bom.pts_for_wallclock_ms(1_000), 90_000);
        assert_eq!(bom.pts_for_wallclock_ms(-500), PTS_MODULUS - 45_000);
    }

    /// A non-zero anchor: the mapping is affine, not BOM-relative.
    #[test]
    fn non_zero_anchor_maps_affinely() {
        // 2000-02-29T12:34:56.789Z (a leap day, fractional ms).
        let map =
            TimestampMap::parse_tag_value("MPEGTS=900000,LOCAL=2000-02-29T12:34:56.789Z")
                .expect("leap-day LOCAL parses");
        // 2000-02-29T12:34:56Z = 951 827 696 s since epoch.
        assert_eq!(map.local_ms, 951_827_696_000 + 789);
        assert_eq!(map.wallclock_ms_for_pts(900_000), map.local_ms);
        // 900 000 ticks = 10 s of media on either side of the anchor.
        assert_eq!(map.wallclock_ms_for_pts(0), map.local_ms - 10_000);
        assert_eq!(map.wallclock_ms_for_pts(1_800_000), map.local_ms + 10_000);
        assert_eq!(map.pts_for_wallclock_ms(map.local_ms + 500), 900_000 + 45_000);
    }

    /// The rollover vector: the anchor sits just before the 2^33
    /// wrap, the queried PTS just after — one wallclock second
    /// apart. A naive subtraction would report ~-26.5 h.
    #[test]
    fn pts_rollover_across_the_33_bit_wrap_is_exact() {
        let mpegts = PTS_MODULUS - 45_000; // 0.5 s before the wrap
        let map = TimestampMap {
            mpegts,
            local_ms: 5_000,
        };
        // 0.5 s after the wrap = 1 s of media after the anchor.
        assert_eq!(map.wallclock_ms_for_pts(45_000), 6_000);
        // And backwards across the wrap from the other side.
        let map2 = TimestampMap {
            mpegts: 45_000,
            local_ms: 0,
        };
        assert_eq!(map2.wallclock_ms_for_pts(PTS_MODULUS - 45_000), -1_000);
        // Round-trip across the wrap.
        assert_eq!(map.pts_for_wallclock_ms(6_000), 45_000);
    }

    /// A playlist's LAST tag governs (an encoder restart republishes
    /// the pair mid-playlist); occurrences that are malformed are
    /// skipped, never fatal.
    #[test]
    fn last_parseable_tag_in_the_playlist_wins() {
        let text = "#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-TIMESTAMP-MAP:MPEGTS=900000,LOCAL=2015-11-09T19:43:20Z\n\
            #EXTINF:2.0,\na.ts\n\
            #EXT-X-TIMESTAMP-MAP:MPEGTS=9000000,LOCAL=2015-11-09T19:44:00.500Z\n\
            #EXTINF:2.0,\nb.ts\n";
        let map =
            TimestampMap::from_playlist_text(text).expect("playlist carries a parseable tag");
        assert_eq!(map.mpegts, 9_000_000);
        assert_eq!(map.local_ms, 1_447_098_240_500); // 2015-11-09T19:44:00.5Z
        // A second occurrence that is garbage does not undo the last
        // good one before it.
        let dirty = "#EXTM3U\n\
            #EXT-X-TIMESTAMP-MAP:MPEGTS=45000,LOCAL=2020-01-01T00:00:00Z\n\
            #EXT-X-TIMESTAMP-MAP:MPEGTS=oops,LOCAL=2020-01-01T00:00:00Z\n";
        let map2 =
            TimestampMap::from_playlist_text(dirty).expect("the good occurrence survives");
        assert_eq!(map2.mpegts, 45_000);
        // No tag at all.
        assert_eq!(
            TimestampMap::from_playlist_text("#EXTM3U\n#EXTINF:1.0,\ns.ts\n"),
            None
        );
    }

    /// Malformed shapes refuse (return `None`), never panic or
    /// clamp: 33-bit overflow, impossible dates, non-UTC zone
    /// spellings, missing attributes.
    #[test]
    fn malformed_tags_refuse() {
        // PTS outside the 33-bit counter.
        assert_eq!(
            TimestampMap::parse_tag_value(&format!(
                "MPEGTS={PTS_MODULUS},LOCAL=1970-01-01T00:00:00Z"
            )),
            None
        );
        // Missing either required attribute.
        assert_eq!(TimestampMap::parse_tag_value("MPEGTS=900000"), None);
        assert_eq!(
            TimestampMap::parse_tag_value("LOCAL=1970-01-01T00:00:00Z"),
            None
        );
        // Impossible dates/times (Feb 30, month 13, hour 24).
        for bad_local in [
            "1970-02-30T00:00:00Z",
            "1970-13-01T00:00:00Z",
            "1970-01-01T24:00:00Z",
            "1970-01-01T00:60:00Z",
            "1970-01-01T00:00:60Z",
        ] {
            assert_eq!(
                TimestampMap::parse_tag_value(&format!("MPEGTS=0,LOCAL={bad_local}")),
                None,
                "{bad_local} must refuse"
            );
        }
        // Non-UTC zone spelling (the profile this parser admits).
        assert_eq!(
            TimestampMap::parse_tag_value("MPEGTS=0,LOCAL=1970-01-01T00:00:00+01:00"),
            None
        );
        // Fractional garbage.
        assert_eq!(
            TimestampMap::parse_tag_value("MPEGTS=0,LOCAL=1970-01-01T00:00:00.5xZ"),
            None
        );
    }

    /// The calendar arithmetic: spot dates against independently
    /// computed Unix days (no date crate, so the truth table is
    /// explicit).
    #[test]
    fn civil_date_arithmetic_truth_table() {
        let unix = |y, m, d| days_from_civil(y, m, d);
        assert_eq!(unix(1970, 1, 1), 0);
        assert_eq!(unix(1970, 1, 2), 1);
        assert_eq!(unix(1969, 12, 31), -1);
        assert_eq!(unix(2000, 2, 29), 11_016); // leap day 2000 (400-year rule)
        assert_eq!(unix(1900, 2, 28), -25_509); // 1900 is NOT a leap year
        assert_eq!(unix(2024, 12, 31), 20_088);
        assert_eq!(unix(2100, 1, 1), 47_482);
    }
}
