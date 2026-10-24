//! WebVTT timestamps (card 5.M3): the two legal spellings
//! `HH:MM:SS.mmm` and `MM:SS.mmm`, parsed greedily so the caller can
//! take whatever follows (cue settings) off the same line.
//!
//! Tolerance decisions (dirty world, D-4): milliseconds may be 1–3
//! digits (`0.5` = 500 ms, per-spec parsers only accept 3 — we accept
//! fewer and right-pad); seconds must be `0..=59`; minutes must be
//! `0..=59` **when an hours field is present** — in the hours-less
//! `MM:SS.mmm` form the leading field is the big one and any value is
//! accepted (`75:00.000` = 75 min, spec-permitted for long cues).
//! Hours may be any digit count; the µs total saturates at `i64::MAX`
//! rather than overflowing.

/// Parse a full string as a timestamp (whitespace-trimmed; the entire
/// remainder must be consumed).
pub(super) fn parse_timestamp(s: &str) -> Option<i64> {
    let trimmed = s.trim();
    let (value, rest) = parse_timestamp_prefix(trimmed)?;
    rest.is_empty().then_some(value)
}

/// Greedily parse a timestamp from the start of `s`, returning its µs
/// value plus the unconsumed remainder (cue settings live there).
pub(super) fn parse_timestamp_prefix(s: &str) -> Option<(i64, &str)> {
    // Consume the timestamp's character class: digits, ':' and '.'.
    let end = s
        .find(|c: char| !c.is_ascii_digit() && c != ':' && c != '.')
        .unwrap_or(s.len());
    let (candidate, rest) = s.split_at(end);
    if candidate.is_empty() {
        return None;
    }
    timestamp_us(candidate).map(|us| (us, rest))
}

/// The grammar core: `H+:MM:SS.mmm` or `MM:SS.mmm`.
fn timestamp_us(s: &str) -> Option<i64> {
    let (time_part, ms_part) = s.rsplit_once('.')?;
    let fields: Vec<&str> = time_part.split(':').collect();
    if !matches!(fields.len(), 2 | 3) {
        return None;
    }
    let (hours, minutes, seconds) = match fields[..] {
        [h, m, sec] => (Some(h), m, sec),
        [m, sec] => (None, m, sec),
        _ => return None,
    };
    // Digit-count sanity: sec exactly 1–2 digits (spec: 2; tolerate 1),
    // minutes 1–2 digits when hours are present, milliseconds 1–3.
    if seconds.is_empty() || seconds.len() > 2 {
        return None;
    }
    if ms_part.is_empty() || ms_part.len() > 3 {
        return None;
    }
    let seconds: u64 = seconds.parse().ok()?;
    if seconds > 59 {
        return None;
    }
    let minutes: u64 = minutes.parse().ok()?;
    if hours.is_some() && minutes > 59 {
        return None;
    }
    // Zero-pad milliseconds on the right: ".5" is 500 ms, ".05" is 50 ms.
    let ms_digits = format!("{ms_part:0<3}");
    let millis: u64 = ms_digits.parse().ok()?;
    if millis > 999 {
        return None;
    }
    let hours: u64 = match hours {
        Some(h) if !h.is_empty() => h.parse().ok()?,
        Some(_) => return None,
        None => 0,
    };
    // u128 intermediate: hours is unbounded digit-wise, µs must not
    // wrap — saturate to i64::MAX instead (MP §8: no surprise ints).
    let total = (hours as u128) * 3_600_000_000
        + (minutes as u128) * 60_000_000
        + (seconds as u128) * 1_000_000
        + (millis as u128) * 1_000;
    Some(total.min(i64::MAX as u128) as i64)
}

#[cfg(test)]
mod tests {
    use super::{parse_timestamp, parse_timestamp_prefix};
    use testkit::Must;

    #[test]
    fn full_forms() {
        assert_eq!(parse_timestamp("00:00:01.000"), Some(1_000_000));
        assert_eq!(parse_timestamp("01:02:03.004"), Some(3_723_004_000));
        assert_eq!(parse_timestamp("02:03.004"), Some(123_004_000));
        assert_eq!(parse_timestamp(" 00:05.500 "), Some(5_500_000));
        assert_eq!(parse_timestamp("0:00:00.000"), Some(0));
        // 75 minutes, hours-less leading field is legal.
        assert_eq!(parse_timestamp("75:00.000"), Some(4_500_000_000));
        // Fraction padding: .5 → 500 ms, .05 → 50 ms.
        assert_eq!(parse_timestamp("00:00:01.5"), Some(1_500_000));
        assert_eq!(parse_timestamp("00:00:01.05"), Some(1_050_000));
    }

    #[test]
    fn rejects_malformed() {
        for bad in [
            "",
            ":",
            ".",
            "00:",
            "00.000",
            ":00.000",
            "00:00.",
            "00:00.0000",
            "00:60.000",    // seconds > 59
            "00:00:60.000", // seconds > 59 with hours
            "01:60:00.000", // minutes > 59 with hours
            "a:b:c.d",
            "00:00:00:00.000",
            "1:2:3.4.5",
            "--1:00.000",
            "00:00.-1.000",
            "1e3:00.000",
            "00:00:01.1000",
        ] {
            assert_eq!(parse_timestamp(bad), None, "accepted {bad:?}");
        }
    }

    #[test]
    fn greedy_prefix_leaves_settings() {
        let (us, rest) = parse_timestamp_prefix("00:00:04.000 position:50% align:center").must();
        assert_eq!(us, 4_000_000);
        assert_eq!(rest, " position:50% align:center");
        let (us, rest) = parse_timestamp_prefix("12:34.567junk").must();
        assert_eq!(us, 754_567_000);
        assert_eq!(rest, "junk");
        assert_eq!(parse_timestamp_prefix(""), None);
        assert_eq!(parse_timestamp_prefix("plain text"), None);
    }

    #[test]
    fn saturates_at_i64_max() {
        // ~1e14 hours overflows; the result must clamp, not wrap.
        assert_eq!(parse_timestamp("99999999999999:00:00.000"), Some(i64::MAX));
    }
}
