//! VobSub `.idx` parse (the index half of an IDX/SUB pair): the
//! de-facto text format the reference tooling (ffmpeg's vobsub
//! demuxer, vobsub.py, mkvtoolnix) documents.
//!
//! ```text
//! # VobSub index file, v7 (do not modify this line!)
//! size: 720x480
//! palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ... (16 × RRGGBB)
//! id: en, index: 0
//! alt: English
//! delay: 0:00:00:00
//! langidx: 0
//! timestamp: 00:00:01:234, filepos: 000000000
//! ```
//!
//! Tolerant by contract (D-4): unknown lines are skipped, partial
//! palettes keep their leading entries, timestamps before any `id:`
//! are typed damage, and a `#` comment never parses. Never panics.

/// Typed IDX parse damage (the decode continues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdxDamage {
    /// A `timestamp:` line before any `id:` line.
    TimestampBeforeStream,
    /// A malformed `timestamp:` line (unparseable time or filepos).
    BadTimestamp { line: String },
    /// A `palette:` line with fewer than 16 entries.
    ShortPalette { got: usize },
    /// A `size:` line that is not `WxH`.
    BadSize { line: String },
}

impl std::fmt::Display for IdxDamage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdxDamage::TimestampBeforeStream => {
                f.write_str("timestamp line before any id: line (entry dropped)")
            }
            IdxDamage::BadTimestamp { line } => write!(f, "unparseable timestamp line {line:?}"),
            IdxDamage::ShortPalette { got } => {
                write!(f, "palette has {got} of 16 entries (rest default)")
            }
            IdxDamage::BadSize { line } => write!(f, "unparseable size line {line:?}"),
        }
    }
}

/// One subtitle stream declared by the idx.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdxStream {
    /// The language tag (`id:` value).
    pub lang: String,
    /// The substream index (`index:` value, 0..32) — the `.sub`
    /// substream byte is `0x20 + index`.
    pub index: u8,
    /// The display title (`alt:`), when present.
    pub title: Option<String>,
    /// `(timestamp ms, filepos)` entries in file order.
    pub entries: Vec<IdxEntry>,
}

/// One idx timestamp line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdxEntry {
    /// The entry's PTS in milliseconds.
    pub timestamp_ms: i64,
    /// The byte offset of the entry's PES in the `.sub`.
    pub filepos: u64,
}

/// A parsed `.idx`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VobSubIdx {
    /// The video canvas (`size:`), when declared.
    pub size: Option<(u32, u32)>,
    /// The 16-entry palette (`palette:`), `RRGGBB` per entry —
    /// missing entries default to black.
    pub palette: [u32; 16],
    /// Whether the idx carried a palette at all.
    pub has_palette: bool,
    /// `langidx:` — the default stream index.
    pub lang_idx: Option<u8>,
    /// The per-stream entry lists (in declaration order).
    pub streams: Vec<IdxStream>,
    /// The damage ledger (uncapped — bounded by the line count).
    pub damage: Vec<IdxDamage>,
}

/// Parse an `.idx` text (the file bytes; any newline style).
pub fn parse_idx(text: &[u8]) -> VobSubIdx {
    let mut idx = VobSubIdx {
        size: None,
        palette: [0; 16],
        has_palette: false,
        lang_idx: None,
        streams: Vec::new(),
        damage: Vec::new(),
    };
    let mut current: Option<usize> = None;
    // Per-stream accumulated `delay:` shifts (applied to every entry
    // of the stream at the end — delays may precede or follow the
    // timestamp lines).
    let mut delays: Vec<i64> = Vec::new();

    let content = String::from_utf8_lossy(text);
    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if let Some(rest) = line.strip_prefix("size:") {
            let rest = rest.trim();
            if let Some((w, h)) = rest.split_once('x')
                && let (Ok(w), Ok(h)) = (w.trim().parse::<u32>(), h.trim().parse::<u32>())
            {
                idx.size = Some((w, h));
                continue;
            }
            idx.damage.push(IdxDamage::BadSize {
                line: line.to_owned(),
            });
        } else if let Some(rest) = line.strip_prefix("palette:") {
            idx.has_palette = true;
            let mut got = 0usize;
            for (slot, entry) in idx.palette.iter_mut().zip(rest.split(',')) {
                if let Ok(v) = u32::from_str_radix(entry.trim(), 16) {
                    *slot = v & 0xFF_FFFF;
                    got += 1;
                }
            }
            if got < 16 {
                idx.damage.push(IdxDamage::ShortPalette { got });
            }
        } else if let Some(rest) = line.strip_prefix("id:") {
            let rest = rest.trim();
            let (lang, index) = match rest.split_once(',') {
                Some((lang, index_part)) => {
                    let lang = lang.trim().to_owned();
                    let index = index_part
                        .trim()
                        .strip_prefix("index:")
                        .and_then(|v| v.trim().parse::<u8>().ok())
                        .unwrap_or(0);
                    (lang, index)
                }
                None => (rest.to_owned(), 0),
            };
            idx.streams.push(IdxStream {
                lang,
                index,
                title: None,
                entries: Vec::new(),
            });
            delays.push(0);
            current = Some(idx.streams.len() - 1);
        } else if let Some(rest) = line.strip_prefix("alt:") {
            // `alt:` follows its `id:` line and titles THAT stream
            // (the vobsub.py / mkvtoolnix layout).
            if let Some(ci) = current {
                idx.streams[ci].title = Some(rest.trim().to_owned());
            }
        } else if let Some(rest) = line.strip_prefix("langidx:") {
            idx.lang_idx = rest.trim().parse::<u8>().ok();
        } else if let Some(rest) = line.strip_prefix("delay:") {
            // Delays apply to the whole stream's timestamps (both
            // before and after this line).
            if let Some(sign) = parse_delay(rest)
                && let Some(ci) = current
            {
                delays[ci] += sign;
            }
        } else if let Some(rest) = line.strip_prefix("timestamp:") {
            let Some(ci) = current else {
                idx.damage.push(IdxDamage::TimestampBeforeStream);
                continue;
            };
            match parse_timestamp(rest) {
                Some((ms, filepos)) => idx.streams[ci].entries.push(IdxEntry {
                    timestamp_ms: ms,
                    filepos,
                }),
                None => idx.damage.push(IdxDamage::BadTimestamp {
                    line: line.to_owned(),
                }),
            }
        }
        // `#` comments, `custom colors`, `Org.` etc.: skipped.
    }
    for (stream, delay) in idx.streams.iter_mut().zip(delays) {
        if delay != 0 {
            for entry in &mut stream.entries {
                entry.timestamp_ms += delay;
            }
        }
    }
    idx
}

/// `HH:MM:SS:ms, filepos: hex` → `(ms, filepos)`.
fn parse_timestamp(rest: &str) -> Option<(i64, u64)> {
    let (time, filepos) = rest.trim().split_once(',')?;
    let mut parts = time.trim().split(':');
    let hh: i64 = parts.next()?.trim().parse().ok()?;
    let mm: i64 = parts.next()?.trim().parse().ok()?;
    let ss: i64 = parts.next()?.trim().parse().ok()?;
    let ms: i64 = parts.next()?.trim().parse().ok()?;
    let filepos = filepos.trim().strip_prefix("filepos:")?;
    let pos = u64::from_str_radix(filepos.trim(), 16).ok()?;
    Some(((hh * 3600 + mm * 60 + ss) * 1000 + ms, pos))
}

/// `[+-]H:MM:SS:ms` → milliseconds.
fn parse_delay(rest: &str) -> Option<i64> {
    let rest = rest.trim();
    let (sign, rest) = match rest.strip_prefix('-') {
        Some(r) => (-1i64, r),
        None => (1i64, rest.strip_prefix('+').unwrap_or(rest)),
    };
    let mut parts = rest.trim().split(':');
    let hh: i64 = parts.next()?.trim().parse().ok()?;
    let mm: i64 = parts.next()?.trim().parse().ok()?;
    let ss: i64 = parts.next()?.trim().parse().ok()?;
    let ms: i64 = parts.next()?.trim().parse().ok()?;
    Some(sign * ((hh * 3600 + mm * 60 + ss) * 1000 + ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# VobSub index file, v7 (do not modify this line!)\n\
        size: 720x480\n\
        palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00? no—\n";

    #[test]
    fn full_idx_parse() {
        let text = "# VobSub index file, v7 (do not modify this line!)\n\
                    size: 720x480\n\
                    palette: 000000, 0000ff, 00ff00, ffff00, ff0000, ff00ff, ffff00, ffffff, 808080, 404040, 202020, 101010, 080808, 040404, 020202, 010101\n\
                    id: en, index: 0\n\
                    alt: English\n\
                    timestamp: 00:00:01:234, filepos: 000000000\n\
                    timestamp: 00:00:04:000, filepos: 000002000\n\
                    id: de, index: 1\n\
                    timestamp: 00:00:02:500, filepos: 000001000\n\
                    langidx: 0\n";
        let idx = parse_idx(text.as_bytes());
        assert!(idx.damage.is_empty(), "{:?}", idx.damage);
        assert_eq!(idx.size, Some((720, 480)));
        assert!(idx.has_palette);
        assert_eq!(idx.palette[1], 0x0000FF);
        assert_eq!(idx.palette[15], 0x010101);
        assert_eq!(idx.streams.len(), 2);
        assert_eq!(idx.lang_idx, Some(0));
        let en = &idx.streams[0];
        assert_eq!((en.lang.as_str(), en.index), ("en", 0));
        assert_eq!(en.title.as_deref(), Some("English"));
        assert_eq!(en.entries.len(), 2);
        assert_eq!(en.entries[0].timestamp_ms, 1234);
        assert_eq!(en.entries[0].filepos, 0);
        assert_eq!(en.entries[1].filepos, 0x2000);
        let de = &idx.streams[1];
        assert_eq!(de.index, 1);
        assert_eq!(de.entries[0].timestamp_ms, 2500);
        assert_eq!(de.entries[0].filepos, 0x1000);
    }

    #[test]
    fn tolerances_never_panic() {
        let idx = parse_idx(SAMPLE.as_bytes());
        assert!(matches!(
            idx.damage.first(),
            Some(IdxDamage::ShortPalette { got: 6 })
        ));
        assert!(!idx.has_palette || idx.palette[0] == 0);
        // Timestamp before any stream.
        let idx = parse_idx(b"timestamp: nonsense, filepos: zz");
        assert!(matches!(
            idx.damage.first(),
            Some(IdxDamage::TimestampBeforeStream)
        ));
        // Garbage everywhere.
        let idx = parse_idx(&[0xFF, 0xFE, 0x00, b'#', b'x']);
        assert!(idx.streams.is_empty());
        assert!(idx.damage.is_empty());
        // Bad size + bad timestamp lines.
        let idx =
            parse_idx(b"size: aandroidx\nid: und, index: 0\ntimestamp: xx:yy:zz:ww, filepos: 1");
        assert!(matches!(idx.damage[0], IdxDamage::BadSize { .. }));
        assert!(matches!(idx.damage[1], IdxDamage::BadTimestamp { .. }));
    }

    #[test]
    fn delay_shifts_entries() {
        let text = "id: en, index: 0\n\
                    delay: 0:00:00:500\n\
                    timestamp: 00:00:01:000, filepos: 000000000\n\
                    timestamp: 00:00:02:000, filepos: 000000100\n";
        let idx = parse_idx(text.as_bytes());
        assert_eq!(idx.streams[0].entries[0].timestamp_ms, 1500);
        assert_eq!(idx.streams[0].entries[1].timestamp_ms, 2500);
        let text = text.replace("0:00:00:500", "-0:00:01:000");
        let idx = parse_idx(text.as_bytes());
        assert_eq!(idx.streams[0].entries[0].timestamp_ms, 0);
        assert_eq!(idx.streams[0].entries[1].timestamp_ms, 1000);
    }
}
